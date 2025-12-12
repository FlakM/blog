{ pkgs, config, lib, ... }:

let
  ngx_http_geoip2_module = pkgs.stdenv.mkDerivation rec {
    name = "ngx_http_geoip2_module";
    src = pkgs.fetchFromGitHub {
      owner = "leev";
      repo = "ngx_http_geoip2_module";
      rev = "3.4";
      sha256 = "sha256-CAs1JZsHY7RymSBYbumC2BENsXtZP3p4ljH5QKwz5yg=";
    };
    installPhase = ''
      mkdir $out
      cp -r * $out/
    '';
  };
in
{

  networking.wireguard.interfaces = {
    wg0 = {
      ips = [ "10.100.0.1/24" ];
      listenPort = 51820;
      privateKeyFile = config.sops.secrets.wireguard_hetzner_private_key.path;

      peers = [
        {
          publicKey = "wMVo8vbsOliaVV63dxV4xg/zuSwgy/qvkgyE//wpznA=";
          allowedIPs = [ "10.100.0.2/32" ];
        }
      ];
    };
  };

  networking.firewall.allowedUDPPorts = [ 51820 ];

  sops.secrets.wireguard_hetzner_private_key = {
    sopsFile = ./secrets/secrets.yaml;
  };

  services.nginx = {
    package = pkgs.nginxStable.overrideAttrs (oldAttrs: {
      configureFlags = oldAttrs.configureFlags ++ [
        "--add-module=${ngx_http_geoip2_module}"
      ];
      buildInputs = oldAttrs.buildInputs ++ [ pkgs.libmaxminddb ];
    });

    recommendedProxySettings = true;
    recommendedTlsSettings = true;
    recommendedOptimisation = true;
    recommendedGzipSettings = true;

    commonHttpConfig = ''
      log_format stripsecrets '$remote_addr $host - $remote_user [$time_local] '
                              '"$secretfilter" $status $body_bytes_sent '
                              '"$http_referer" "$http_user_agent"';

      map $request $secretfilter {
          ~*^(?<prefix1>.*[\?&]api_key=)([^&]*)(?<suffix1>.*)$  "''${prefix1}***$suffix1";
          ~*^(?<prefix1>.*[\?&]ApiKey=)([^&]*)(?<suffix1>.*)$  "''${prefix1}***$suffix1";
          default $request;
      }

      # Truncate request URI to 20 chars for Jellyfin logs to reduce PII footprint
      map $request_uri $jellyfin_short_uri {
        "~^(?<prefix>.{0,20}).*" $prefix;
        default $request_uri;
      }

      # JSON log format tailored for Coralogix ingestion
      log_format jellyfin_json escape=json
        '{'
          '"time":"$time_iso8601",'
          '"remote_addr":"$remote_addr",'
          '"host":"$host",'
          '"method":"$request_method",'
          '"uri_short":"$jellyfin_short_uri",'
          '"status":$status,'
          '"bytes_sent":$body_bytes_sent,'
          '"referer":"$http_referer",'
          '"user_agent":"$http_user_agent",'
          '"forwarded_for":"$proxy_add_x_forwarded_for"'
        '}';
    '';

    appendHttpConfig = ''
      geoip2 /var/lib/geoip-databases/GeoLite2-Country.mmdb {
        auto_reload 5m;
        $geoip2_data_country_code country iso_code;
      }

      map $geoip2_data_country_code $allowed_country {
        default no;
        PL yes;
      }

      access_log /var/log/nginx/access.log stripsecrets;
    '';

    virtualHosts = {
      "jellyfin.public.flakm.com" = {
        forceSSL = true;
        enableACME = true;
        http2 = true;

        extraConfig = ''
          client_max_body_size 20M;
          ssl_protocols TLSv1.3 TLSv1.2;

          # Avoid proxy header hash warnings for this header-heavy vhost
          proxy_headers_hash_max_size 1024;
          proxy_headers_hash_bucket_size 128;

          access_log /var/log/nginx/jellyfin_access.log jellyfin_json;
          error_log /var/log/nginx/jellyfin_error.log;

          if ($allowed_country = no) {
            return 403;
          }
        '';

        locations."/" = {
          extraConfig = ''
            proxy_pass http://10.100.0.2:8096;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_set_header X-Forwarded-Protocol $scheme;
            proxy_set_header X-Forwarded-Host $http_host;
            proxy_redirect http://10.100.0.2:8096 https://jellyfin.public.flakm.com;
            proxy_buffering off;

            add_header X-Content-Type-Options "nosniff" always;
            add_header Permissions-Policy "accelerometer=(), ambient-light-sensor=(), battery=(), bluetooth=(), camera=(), clipboard-read=(), display-capture=(), document-domain=(), encrypted-media=(), gamepad=(), geolocation=(), gyroscope=(), hid=(), idle-detection=(), interest-cohort=(), keyboard-map=(), local-fonts=(), magnetometer=(), microphone=(), payment=(), publickey-credentials-get=(), serial=(), sync-xhr=(), usb=(), xr-spatial-tracking=()" always;
            add_header Content-Security-Policy "default-src https: data: blob: ; img-src 'self' https://* ; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' https://www.gstatic.com https://www.youtube.com blob:; worker-src 'self' blob:; connect-src 'self'; object-src 'none'; font-src 'self'" always;
          '';
        };

        locations."/socket" = {
          extraConfig = ''
            proxy_pass http://10.100.0.2:8096;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            proxy_set_header X-Forwarded-Protocol $scheme;
            proxy_set_header X-Forwarded-Host $http_host;
          '';
        };
      };
    };
  };

  services.fail2ban = {
    enable = true;
    maxretry = 5;
    ignoreIP = [
      "127.0.0.1/8"
      "::1"
      "10.100.0.0/24"
    ];

    jails = {
      jellyfin = ''
        enabled = true
        port = http,https
        filter = jellyfin
        logpath = /var/log/jellyfin/log*.log
        maxretry = 3
        findtime = 3600
        bantime = 86400
        action = iptables-allports[name=jellyfin]
      '';
    };
  };

  environment.etc."fail2ban/filter.d/jellyfin.conf".text = ''
    [Definition]
    failregex = ^.*Authentication request for .* has been denied \(IP: "<HOST>"\)\..*$
                ^.*Failed login attempt from "<HOST>".*$
    ignoreregex =
  '';

  systemd.services.geoip-updater = {
    description = "Update GeoIP databases";
    wantedBy = [ "multi-user.target" ];
    after = [ "network-online.target" ];
    wants = [ "network-online.target" ];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = pkgs.writeShellScript "geoip-updater" ''
        set -e
        mkdir -p /var/lib/geoip-databases
        cd /var/lib/geoip-databases
        ${pkgs.curl}/bin/curl -L "https://github.com/P3TERX/GeoLite.mmdb/raw/download/GeoLite2-Country.mmdb" -o GeoLite2-Country.mmdb.tmp
        mv GeoLite2-Country.mmdb.tmp GeoLite2-Country.mmdb
        chmod 644 GeoLite2-Country.mmdb
      '';
    };
  };

  systemd.timers.geoip-updater = {
    description = "Update GeoIP databases weekly";
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnCalendar = "weekly";
      Persistent = true;
    };
  };

}
