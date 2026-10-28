# Public ntfy proxy: entry point for agents publishing/subscribing from
# outside the LAN. Proxies over the wireguard link to the raw ntfy listener
# on odroid (:2586, firewall restricted to wg0 there). Auth is enforced by
# ntfy itself (private instance, deny-all + users/ACL/tokens provisioned from
# secrets on odroid) - see hosts/odroid/ntfy.nix in nix_dots.
{ pkgs, lib, ... }: {
  services.nginx.virtualHosts."ntfy.public.flakm.com" = {
    forceSSL = true;
    enableACME = true;
    http2 = true;

    extraConfig = ''
      # attachments via POST/PUT are small; still be generous with body size
      client_max_body_size 20M;
      ssl_protocols TLSv1.3 TLSv1.2;

      proxy_headers_hash_max_size 1024;
      proxy_headers_hash_bucket_size 128;

      access_log /var/log/nginx/ntfy_access.log jellyfin_json;
      error_log /var/log/nginx/ntfy_error.log;

      if ($allowed_country = no) {
        return 403;
      }
    '';

    locations."/" = {
      # ntfy subscriptions are long-lived SSE/JSON streams (and websockets for
      # the web app); disable buffering and extend the read timeout.
      proxyWebsockets = true;
      extraConfig = ''
        proxy_pass http://10.100.0.2:2586;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_redirect http://10.100.0.2:2586 https://ntfy.public.flakm.com;
        proxy_buffering off;
        proxy_cache off;
        gzip off;
        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
      '';
    };
  };

  # Brute-force protection on the JSON access log (jellyfin_json fields).
  services.fail2ban.jails.ntfy = ''
    enabled = true
    filter = ntfy
    logpath = /var/log/nginx/ntfy_access.log
    findtime = 600
    maxretry = 15
    bantime = 3600
    action = iptables-allports[name=ntfy]
  '';

  environment.etc."fail2ban/filter.d/ntfy.conf".text = ''
    [Definition]
    failregex = ^.*"remote_addr":"<HOST>".*"host":"ntfy\.public\.flakm\.com".*"status":40[13].*$
    ignoreregex =
  '';
}
