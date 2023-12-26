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
      adminUser = {
        # activate is used to skip the email verification of the admin-user that's
        # automatically created by plausible. This is only supported if
        # postgresql is configured by the module. This is done by default, but
        # can be turned off with services.plausible.database.postgres.setup.
        name = "plausible";
        activate = true;
        email = "hello@plausible.local";
        passwordFile = "/var/secrets/plausible/plausibleAdminPassword";
      };

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

          recommendedProxySettings = true;
        };
        # include X-Forwarded-Ip header in proxied requests using realip module
        # https://nginx.org/en/docs/http/ngx_http_realip_module.html
      };
    };
  };

  security.acme = {
    certs = {
      ${domain}.email = "me@flakm.com";
    };
  };

}
