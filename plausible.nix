{ config, lib, pkgs, ... }:
# copied from https://carjorvaz.com/posts/setting-up-plausible-analytics-on-nixos/
# one time setup must be done before this config is applied
# mkdir -p /var/secrets/plausible/
# openssl rand -base64 64 | tr -d '\n'  > /var/secrets/plausible/plausibleSecretKeybase 
# openssl rand -base64 64 | tr -d '\n' > /var/secrets/plausible/plausibleAdminPassword
let
  domain = "plausible.flakm.com";
in
{
  services = {
    plausible = {
      enable = true;

      server = {
        baseUrl = "https://${domain}";
        secretKeybaseFile = "/var/secrets/plausible/plausibleSecretKeybase";
      };
    };

    nginx = {
      virtualHosts.${domain} = {
        forceSSL = true;
        enableACME = true;
        locations."/" = {
          proxyPass = "http://127.0.0.1:8000";
          proxyWebsockets = true;

          recommendedProxySettings = true;
          extraConfig = ''
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
          '';
        };
      };
    };


  };

  security.acme = {
    certs = {
      ${domain}.email = "me@flakm.com";
    };
  };

  # Ensure PostgreSQL starts before Plausible services
  systemd.services.plausible-postgres = {
    after = [ "postgresql.service" ];
    wants = [ "postgresql.service" ];
  };
  
  systemd.services.plausible = {
    after = [ "postgresql.service" "plausible-postgres.service" ];
    wants = [ "postgresql.service" ];
  };
}
