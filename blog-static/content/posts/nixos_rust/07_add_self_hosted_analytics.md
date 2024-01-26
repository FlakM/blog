+++ 
draft = true
date = 2023-12-26T21:08:00+01:00
title = "Add self hosted analytics"
tags = []
externalLink = ""
series = ["Simple personal blog"]

description = """
Adding privacy-friendly self hosted analytics
"""
+++


## Rationale for analytics

I'd like to understand which blog entries seem interesting and gather metrics to understand where to put my limited time.
Understandably, I'd also prefer not to invade on my user's privacy by sharing data with third parties like google.
This is why I've decided to use a self hosted solution. Hugo has already integrations with some providers.

Plausible is an open source solution licensed under AGPLv3. It's already packaged in NixOs so deployin it on the host should be simple.

## Enabling plausible

### Adding dns entry

First we need to add dns entry in tofu:

```terraform
# Cloudflare DNS A record configuration for the plausible analytics
resource "cloudflare_record" "plausible_nginx" {
  zone_id = var.ZONE_ID
  name    = "plausible.flakm.com"
  value   = hcloud_server.blog.ipv4_address
  type    = "A"
  proxied = false  # Direct DNS, no Cloudflare proxy
}
```

After running `tofu plan` and `tofu init` a new DNS A entry will be created pointing at the same hetzner service.

### Self host plausible instance

We can use NixOs service by modyfying flake.nix:

```nix
#flake.nix

      nixosConfigurations.blog = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = attrs;
        modules = [
          disko.nixosModules.disko
          ./configuration.nix 

          # 👇 this module import is added
          ./plausible.nix
        ];
      };
```

And the plausible configuration itself copied from [this blog entry](https://carjorvaz.com/posts/setting-up-plausible-analytics-on-nixos/):

```nix
# plausible.nix
{ config, lib, pkgs, ... }:
# 
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
        locations."/".proxyPass = "http://127.0.0.1:8000";
      };
    };
  };

  security.acme = {
    certs = {
      ${domain}.email = "me@flakm.com";
    };
  };

}
```

### Telling hugo site about plausible

Our static website has to know about the tracking. Luckily hugo provides a set of instructions for including [analytics](https://github.com/luizdepra/hugo-coder/blob/main/docs/analytics.md).
It is as simple as adding following configuration to `blog-static/config.toml`:

```toml
[params.plausibleAnalytics]
  domain = "flakm.com"
  serverURL = "plausible.flakm.com"
```

### Passing the original ip 


https://nixos.wiki/wiki/Nginx#Using_realIP_when_behind_CloudFlare_or_other_CDN

```nix
  services.nginx.commonHttpConfig =
    let
      realIpsFromList = lib.strings.concatMapStringsSep "\n" (x: "set_real_ip_from  ${x};");
      fileToList = x: lib.strings.splitString "\n" (builtins.readFile x);
      cfipv4 = fileToList (pkgs.fetchurl {
        url = "https://www.cloudflare.com/ips-v4";
        sha256 = "0ywy9sg7spafi3gm9q5wb59lbiq0swvf0q3iazl0maq1pj1nsb7h";
      });
      cfipv6 = fileToList (pkgs.fetchurl {
        url = "https://www.cloudflare.com/ips-v6";
        sha256 = "1ad09hijignj6zlqvdjxv7rjj8567z357zfavv201b9vx3ikk7cy";
      });
    in
    ''
      ${realIpsFromList cfipv4}
      ${realIpsFromList cfipv6}
      real_ip_header CF-Connecting-IP;
    '';
```
