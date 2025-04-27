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
