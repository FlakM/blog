{ config, lib, pkgs, ... }:
let
  domain = "plausible.flakm.com";
in
{
  services = {
    plausible = {
      enable = true;

      server = {
        baseUrl = "https://${domain}";
        secretKeybaseFile = "/run/secrets/plausible_secret_key_base";
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
