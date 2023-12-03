---
title: "Screaming at the clouds ☁️"
date: 2023-04-07T14:55:28+02:00
draft: true

series: ["Perfect simple deployment"]

Summary: '
How to provision VM with valid DNS record and cloudflare caching.
'
---

## About the infrastructure

I'll provision the environment using Hetzner and Cloudflare.
This is not perfect but it will be simple to maintain and follow.


**Contents:**

{{< toc >}}

### Provisioning all the things

To host the code I'll need two things:

1. A DNS domain - `flakm.com`
2. A NixOs server that will host code

The server should be reachable for ssh (at least at the beginning) and have port 443 opened and a valid TLS certificate.

Since I heavily believe in self-documentation I have used `opentofu` for preparing the deployment.
I want to be able to redo the same setup and easily apply modifications without forgetting about one of the manual commands.

Since I'm more dev than ops. My requirements were the following: 

- There should be already some tutorials that I can start from
- The whole infrastructure should be provisioned by single command
- The solution should be fully automated

### Prerequisites

I've used the amazing wiki article [Deploying NixOS using Terraform](https://nix.dev/tutorials/deploying-nixos-using-terraform) as a base even though it's somehow dated.
To use the same configuration you will need the following things:

1. [Hetzner API token](https://docs.hetzner.com/cloud/api/getting-started/generating-api-token/)
2. [Cloudflare account and token](https://www.cloudflare.com/) 
3. DNS domain managed by CF

### Environment setup


I'm using a [yubikey smart card with gpg agent for ssh authentication](https://github.com/drduh/YubiKey-Guide#configure-smartcard) and my public key is located in `~/.ssh/id_rsa_yubikey.pub`.

I like this setup, the key never touched the disk on my computer. But there is no problem if you don't have one you might just use plain ssh keys:

```bash
# full resources: https://developers.yubico.com/PIV/Guides/Generating_keys_using_OpenSSL.html
openssl genrsa -out key.pem 2048
# extract public part
openssl rsa -in key.pem -outform PEM -pubout -out public.pem
```

Once you have all the other prerequisites setup you will need the following setup:

```bash
export CLOUDFLARE_API_TOKEN="..." # account token
export HCLOUD_TOKEN="" # Hetzner API token
```

## 🚢 to ☁️ 

### Provisioning

The aggregating flake.nix alongside `opentofu` module is in [repository](https://github.com/FlakM/blog_deployment). Here is the complete terraform deployment definition:

```terraform
terraform {
  required_providers {
    cloudflare = {
      source = "cloudflare/cloudflare"
    }
    hcloud = {
      source = "hetznercloud/hcloud"
    }
    null = {
      source = "hashicorp/null"
    }
  }
}

# SSH Key Configuration
resource "hcloud_ssh_key" "yubi" {
  name       = "foo"
  public_key = chomp(file("~/.ssh/id_rsa_yubikey.pub"))
}

# Hetzner Cloud Server Configuration
resource "hcloud_server" "blog" {
  name        = "blog-instance"
  image       = "ubuntu-22.04"
  server_type = "cpx11"
  location    = "fsn1"  # Frankfurt location, you can choose as per your requirement
  ssh_keys    = [hcloud_ssh_key.yubi.id]
}

# Output public IP of the instance
output "public_ip" {
  value = hcloud_server.blog.ipv4_address
}

provider "cloudflare" {
  # this is taken from CLOUDFLARE_API_TOKEN env 
}

variable "ZONE_ID" {
  # This is taken from VAR_ZONE_ID env
}

variable "domain" {
  default = "flakm.com"
}

# This is the DNS record for the blog not behind the proxy
# It is used for ACMA challenge
resource "cloudflare_record" "blog_nginx" {
  zone_id = var.ZONE_ID
  name    = "blog.flakm.com"
  value   = hcloud_server.blog.ipv4_address
  type    = "A"
  proxied = false
}

# This is the DNS record for the blog behind the CF proxy that will cache the content
resource "cloudflare_record" "blog" {
  zone_id = var.ZONE_ID
  name    = "@"
  value   = "blog.flakm.com"
  type    = "CNAME"
  proxied = true
}

# Settings for flakm.com domain
resource "cloudflare_zone_settings_override" "flakm-com-settings" {
  zone_id = var.ZONE_ID

  settings {
    tls_1_3                  = "on"
    automatic_https_rewrites = "on"
    ssl                      = "strict"
    cache_level              = "aggressive"  # This can be set to "simplified", "aggressive", or "basic" depending on your caching requirements
  }
}

# Add a page rule to the domain
resource "cloudflare_page_rule" "blog" {
  zone_id = var.ZONE_ID
  target = "https://flakm.com"
  priority = 1

  actions {
    # This will cache also the html 
    cache_level = "cache_everything"
  }
}

module "system-build" {
  source            = "github.com/nix-community/nixos-anywhere//terraform/nix-build"
  attribute         = ".#nixosConfigurations.blog.config.system.build.toplevel"
}

module "disko" {
  source         = "github.com/nix-community/nixos-anywhere//terraform/nix-build"
  attribute      = ".#nixosConfigurations.blog.config.system.build.diskoScript"
}

module "install" {
  source            = "github.com/nix-community/nixos-anywhere//terraform/install"
  nixos_system      = module.system-build.result.out
  nixos_partitioner = module.disko.result.out
  target_host       = hcloud_server.blog.ipv4_address
}

```

To run it you will have to setup your environment. The flake in project's repository contains `devShell` section that provides the binaries required to run it.
If you have [direnv integration](https://nixos.wiki/wiki/Flakes#Direnv_integration) setup you will just need to run following command:

```bash
direnv allow
```

And your shell will magically receive all the required binaries (tofu).
To deploy the code run:


```bash
tofu init # downloads the providers
tofu apply
```

At this point after couple of minutes you should be able to check the DNS adress.
It should point to valid ip address of the machine that you have just created:

```bash
nslookup blog.flakm.com
```

If it does you should be able to login to the machine using ssh: 

```bash
ssh root@blog.flakm.com
```

### Job well done

At this point I've managed to create following resources:

1. Hetzner instance with public IP with ssh that gives root access
2. NixOs installed on the machine
3. DNS A record that points to our instance
4. Proxied page in cloudflare
5. Way to deploy changes using nix

We are ready to use nix!

## Taking over the ubuntu host <3

## NixOs deployment

Ok, so now we want to take our NixOs module and install it on our fresh machine! We need to create a new flake inside `blog_deployment` repository.

This is very straight forward. We just need to add new input for now pointing to specific commit and define NixOs configuration called `blog`.
It inherits `system` so in our case - value `"x86_64-linux"`, passes `specialArgs` and finally import a module from file config.nix  which contains our system configuration.


```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    backend = {
      url = "github:FlakM/backend/7a09966d9d6218a7f76f6cd38f52c84758fcf7a5";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, ... }@attrs:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      formatter.${system} = pkgs.nixpkgs-fmt;
      nixosConfigurations.blog = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = attrs;
        modules = [ ./configuration.nix ];
      };


      devShell.x86_64-linux = pkgs.mkShell {
        buildInputs = with pkgs; [
          terraform
          awscli2
        ];
      };
    };
}
```

And configuration:

```nix
# configuration.nix
{ config, lib, pkgs, backend, nixpkgs, ... }: {

  imports = [
    "${nixpkgs}/nixos/modules/virtualisation/amazon-image.nix"
    backend.nixosModules.x86_64-linux.default
  ];

  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 80 443 ];
  };

  services = {
    backend = {
      enable = true;
      domain = "blog.flakm.com";
    };

    nginx = {
      enable = true;
      virtualHosts."blog.flakm.com" = {
        forceSSL = true;
        enableACME = true;
      };
    };
  };


  security.acme = {
    acceptTerms = true;
    certs = {
      "blog.flakm.com".email = "maciej.jan.flak@gmail.com";
    };
  };
}
```

It's also a rather uncomplicated set of settings copied mainly from [NixOs wiki entry for nginx](https://nixos.wiki/wiki/Nginx).
It uses Acme bot to set up TLS certificate for our backend. 
Finally, it enables and configures our service called `backend`

And here is the part that seemed absolutely magic to me. 
If we want to deploy our flake to the machine we'll have to just run the:

```bash
# just tells the nix to use my GPG agent
# you won't need it if you are using plain ssh keys in ~/.ssh/ directory
export NIX_SSHOPTS="-o IdentityAgent=$SSH_AUTH_SOCK"
nixos-rebuild --flake .#blog \
  --target-host root@blog.flakm.com  \
  switch
```

It will build the flake locally using the whole power of the local machine with all the valid inputs and ship the binary packages over ssh:

```bash
# ...  redacted
copying path '/nix/store/hsqvcmn9w9cin2xy74l7gma2n579z09j-quick-start-0.1.0' to 'ssh://root@blog.flakm.com'...
copying path '/nix/store/yilxvsbxh1nybiy7j4zfw6dlvzw9rn2h-unit-backend.service' to 'ssh://root@blog.flakm.com'...
copying path '/nix/store/k7sgdp698500a53lbb3mi7hx67kf19xr-system-units' to 'ssh://root@blog.flakm.com'...
copying path '/nix/store/jkxgii5n3bakf6jxycadb1g3sr9m6gmi-etc' to 'ssh://root@blog.flakm.com'...
copying path '/nix/store/3lmdyrsja06r2ps5s601vhxcgwqijy5c-nixos-system-unnamed-23.05.20230406.0e19daa' to 'ssh://root@blog.flakm.com'...
```

Can it be working already? Let's check in the browser, it can not be that simple!

{{< figure src="/images/page_working.png" class="img-sm">}}

```bash
❯ curl https://flakm.com  -I
HTTP/2 200
date: Fri, 21 Apr 2023 21:02:44 GMT
content-type: text/html; charset=utf-8
cache-control: max-age=14400
cf-cache-status: HIT
age: 613
last-modified: Fri, 21 Apr 2023 20:52:31 GMT
accept-ranges: bytes
report-to: {"endpoints":[{"url":"https:\/\/a.nel.cloudflare.com\/report\/v3?s=hpJTiB0O1JL00CWG%2BSwpHShG8wab8ixvGh8O3z7laYm3AuiYzjg0QGU7HvSmjkOapijegDA0NLckTY%2FDi%2Bsqf1i9JadG5JcNn8BsYNgoOjL%2Be7Z4r1GNEbhgfQw%3D"}],"group":"cf-nel","max_age":604800}
nel: {"success_fraction":0,"report_to":"cf-nel","max_age":604800}
server: cloudflare
cf-ray: 7bb891d51978bfc3-WAW
alt-svc: h3=":443"; ma=86400, h3-29=":443"; ma=86400
```

It is working, and caching works! 
Also, notice that at no point did I mention installing some binary on your system. It's been all taken care of by direnv configuration.

### Summary

So to sum up. Until now we managed to:

1. Build a flake that wraps a hello world rust server but also has a full suite of tests
2. Expose backend flake as a NixOs module with 2 flags that runs binary using systemd service
3. Write terraform module that provisions our machines, DNS records and configures Cloudflare to cache the content
4. Provision resources
5. Write flake that takes backend as an input and configures nginx with ACME bot for valid TLS configuration
6. Deploy that flake to production using a single command that could be easily performed on CI

Not too bad for such a simple setup, right?

