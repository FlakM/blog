{ config, lib, pkgs, ... }:

let
  cfg = config.services.ssologinDemo;
  demo = import ./package.nix { inherit pkgs; };
  stateDir = "/var/lib/ssologin-demo";

  # A bare IPv6 literal has to be bracketed in a proxy_pass URL.
  proxyHost =
    if lib.hasInfix ":" cfg.bindAddress then "[${cfg.bindAddress}]" else cfg.bindAddress;

  # nginx needs to accept connections from outside its own loopback when it
  # fronts the listener on a public interface; the listener itself stays private.
  firewallPorts = lib.optionals cfg.nginx.enable (
    lib.optional (!cfg.nginx.forceSSL) cfg.nginx.httpPort
    ++ lib.optionals cfg.nginx.forceSSL [ 443 80 ]
  );
in
{
  options.services.ssologinDemo = {
    enable = lib.mkEnableOption "the ssologin redirect demo listener";

    package = lib.mkOption {
      type = lib.types.package;
      default = demo.listener;
      defaultText = lib.literalExpression "(import ./package.nix { inherit pkgs; }).listener";
      description = "Listener package to run.";
    };

    bindAddress = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
      description = ''
        Address the listener binds. Keep the default when nginx runs on the
        same host; nginx proxies to it over loopback.
      '';
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8776;
      description = "Loopback port the listener binds and nginx proxies to.";
    };

    sts = lib.mkOption {
      type = lib.types.str;
      example = "https://api.eu2.coralogix.com";
      description = ''
        STS base URL used to redeem captured auth codes. Must be the same
        environment the login happened in.
      '';
    };

    allowNonLoopback = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = ''
        Allow a non-loopback `bindAddress`. Only needed when nginx runs in a
        different network namespace or container. The listener accepts an auth
        code from anyone who can reach it, so think twice.
      '';
    };

    nginx = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Add an nginx virtual host that proxies to the listener.";
      };

      hostName = lib.mkOption {
        type = lib.types.str;
        default = "ssologin-demo.local";
        example = "acme.attacker.example";
        description = ''
          Public host name of the demo. For f1 (forged Origin) the first label
          must be the team slug, e.g. `acme.attacker.example`.
        '';
      };

      serverAliases = lib.mkOption {
        type = lib.types.listOf lib.types.str;
        default = [ ];
        example = [ "coralogix.flakm.com" ];
        description = "Extra host names served by the same vhost.";
      };

      bindAddress = lib.mkOption {
        type = lib.types.str;
        default = "0.0.0.0";
        description = "Address the nginx virtual host listens on.";
      };

      httpPort = lib.mkOption {
        type = lib.types.port;
        default = 80;
        description = "HTTP port when `forceSSL` is off.";
      };

      enableACME = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Obtain a certificate for `hostName` through ACME.";
      };

      forceSSL = lib.mkOption {
        type = lib.types.bool;
        default = config.services.ssologinDemo.nginx.enableACME;
        description = "Redirect HTTP to HTTPS. Requires a certificate.";
      };
    };

    ntfy = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = ''
          Send an ntfy notification through the `agent-msg` script when a
          session is captured. The bearer token is passed as a systemd
          credential from `tokenFile`.
        '';
      };
      tokenFile = lib.mkOption {
        type = lib.types.str;
        default = "/run/secrets/ntfy_agent_token";
        description = ''
          Path to the ntfy bearer token (a sops secret). Read by root and
          exposed to the service through `LoadCredential`.
        '';
      };

      server = lib.mkOption {
        type = lib.types.str;
        default = "https://ntfy.house.flakm.com";
        description = "ntfy server passed to agent-msg via NTFY_SERVER.";
      };

      topic = lib.mkOption {
        type = lib.types.str;
        default = "agent-chat";
        description = "ntfy topic passed to agent-msg via NTFY_TOPIC.";
      };

      clickUrl = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "Optional URL opened when the notification is tapped.";
      };
    };

    entry = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = ''
          Enable the brand-host entry route: `GET /?flow=<id>` (or `/login`)
          mints a login with the legitimate dashboard Origin and this host as
          `redirectUrl`, then redirects to the IdP. Used to share a short
          coralogix-looking host instead of the SAML assertion URL.
        '';
      };

      team = lib.mkOption {
        type = lib.types.str;
        default = "";
        example = "c4c";
        description = "Team slug used by the entry route.";
      };

      dashboardFormat = lib.mkOption {
        type = lib.types.str;
        default = "https://{team}.app.eu2.coralogix.com";
        description = "Legitimate dashboard origin; `{team}` is replaced.";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.allowNonLoopback || lib.elem cfg.bindAddress [ "127.0.0.1" "::1" "localhost" ];
        message = ''
          services.ssologinDemo.bindAddress is not a loopback address. Set
          services.ssologinDemo.allowNonLoopback = true if that is intentional:
          anyone who can reach the listener can hand it an auth code.
        '';
      }
      {
        assertion =
          !cfg.nginx.forceSSL
          || cfg.nginx.enableACME
          || config.services.nginx.virtualHosts.${cfg.nginx.hostName}.sslCertificate != null;
        message = ''
          services.ssologinDemo.nginx.forceSSL needs a certificate: enable
          services.ssologinDemo.nginx.enableACME or set
          services.nginx.virtualHosts."${cfg.nginx.hostName}".sslCertificate yourself.
        '';
      }
    ];

    users.groups.ssologin-demo = { };
    users.users.ssologin-demo = {
      isSystemUser = true;
      group = "ssologin-demo";
      home = stateDir;
      createHome = false;
    };

    systemd.services.ssologin-demo = {
      description = "ssologin redirect demo listener";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ]
        ++ lib.optional cfg.ntfy.enable "sops-install-secrets.service";
      wants = [ "network-online.target" ];
      path = lib.optional cfg.ntfy.enable demo.agent-msg;
      environment = lib.optionalAttrs cfg.ntfy.enable {
        NTFY_NOTIFY = "1";
        NTFY_SERVER = cfg.ntfy.server;
        NTFY_TOPIC = cfg.ntfy.topic;
        NTFY_CLICK_URL = cfg.ntfy.clickUrl;
      };

      serviceConfig = {
        ExecStart = lib.escapeShellArgs (
          [
            "${cfg.package}/bin/ssologin-listener"
            "serve"
            "--bind"
            cfg.bindAddress
            "--port"
            (toString cfg.port)
            "--sts"
            cfg.sts
            "--out"
            "${stateDir}/capture.json"
          ]
          ++ lib.optional cfg.allowNonLoopback "--allow-non-loopback"
          ++ lib.optionals cfg.entry.enable [
            "--entry-team"
            cfg.entry.team
            "--entry-dashboard-format"
            cfg.entry.dashboardFormat
          ]
        );
        User = "ssologin-demo";
        Group = "ssologin-demo";
        StateDirectory = "ssologin-demo";
        StateDirectoryMode = "0700";
        UMask = "0077";
        Restart = "on-failure";
        RestartSec = 2;
        LoadCredential = lib.optionals cfg.ntfy.enable [
          "ntfy_agent_token:${cfg.ntfy.tokenFile}"
        ];

        NoNewPrivileges = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictAddressFamilies = [ "AF_UNIX" "AF_INET" "AF_INET6" "AF_NETLINK" ];
        RestrictNamespaces = true;
        LockPersonality = true;
        CapabilityBoundingSet = "";
        AmbientCapabilities = "";
        SystemCallArchitectures = "native";
      };
    };

    services.nginx = lib.mkIf cfg.nginx.enable {
      enable = true;
      recommendedProxySettings = true;
      recommendedTlsSettings = cfg.nginx.forceSSL;

      virtualHosts.${cfg.nginx.hostName} = {
        enableACME = cfg.nginx.enableACME;
        forceSSL = cfg.nginx.forceSSL;
        serverAliases = cfg.nginx.serverAliases;

        # When forceSSL is on the module adds the 443 listener itself.
        listen = lib.optionals (!cfg.nginx.forceSSL) [
          {
            addr = cfg.nginx.bindAddress;
            port = cfg.nginx.httpPort;
          }
        ];

        locations."/" = {
          proxyPass = "http://${proxyHost}:${toString cfg.port}";
          extraConfig = ''
            proxy_buffering off;
            # The query string carries a single-use auth code. Never log it.
            access_log off;
          '';
        };

        extraConfig = ''
          add_header Cache-Control "no-store" always;
        '';
      };
    };

    networking.firewall.allowedTCPPorts = firewallPorts;
  };
}
