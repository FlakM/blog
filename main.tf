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

variable "ZONE_ID" {
  # This is taken from VAR_ZONE_ID env
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

