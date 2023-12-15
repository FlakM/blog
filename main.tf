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

# Configuration for SSH key to be used with Hetzner Cloud instances
resource "hcloud_ssh_key" "yubi" {
  name       = "foo" 
  public_key = chomp(file("~/.ssh/id_rsa_yubikey.pub"))  
}

# Define a Hetzner Cloud Server resource for the blog
resource "hcloud_server" "blog" {
  name        = "blog-instance"
  image       = "ubuntu-22.04"   # After provisioning, NixOS will be installed see @install
  server_type = "cpx11"          # AMD 2 vCPU, 2 GB RAM, 40 GB NVMe SSD
  location    = "fsn1"
  ssh_keys    = [hcloud_ssh_key.yubi.id]  # SSH keys associated with the server
}

# Output the public IP address of the Hetzner Cloud Server
output "public_ip" {
  value = hcloud_server.blog.ipv4_address
}

# Define a variable for Cloudflare Zone ID
variable "ZONE_ID" {
  # Environment variable for Cloudflare Zone ID
  # export TF_VAR_ZONE_ID="..."
}

# Cloudflare DNS A record configuration for the blog
# This is used for the blog to be accessible directly via the IP ip address
# The blog will be also accessible via the domain name behind the Cloudflare proxy
# See @blog for the CNAME record and cloudflare_page_rule for the url
# This way the communication between Cloudflare and the blog is encrypted
resource "cloudflare_record" "blog_nginx" {
  zone_id = var.ZONE_ID
  name    = "blog.flakm.com"
  value   = hcloud_server.blog.ipv4_address
  type    = "A"
  proxied = false  # Direct DNS, no Cloudflare proxy
}


resource "cloudflare_record" "fedi_nginx" {
  zone_id = var.ZONE_ID
  name    = "fedi.flakm.com"
  value   = hcloud_server.blog.ipv4_address
  type    = "A"
  proxied = false  # Direct DNS, no Cloudflare proxy
}

# Cloudflare DNS CNAME record for the blog behind Cloudflare proxy
resource "cloudflare_record" "blog" {
  zone_id = var.ZONE_ID
  name    = "@"
  value   = "blog.flakm.com"
  type    = "CNAME"
  proxied = true  # Enable Cloudflare proxy
}

# Configure settings for the flakm.com domain in Cloudflare
resource "cloudflare_zone_settings_override" "flakm-com-settings" {
  zone_id = var.ZONE_ID

  settings {
    tls_1_3                  = "on"
    automatic_https_rewrites = "on"
    ssl                      = "strict"
    cache_level              = "aggressive"
  }
}

# Cloudflare page rule for caching and optimizations
resource "cloudflare_page_rule" "blog" {
  zone_id = var.ZONE_ID
  target = "https://flakm.com"
  priority = 1

  actions {
    cache_level = "cache_everything"  # Cache HTML and other assets
  }
}

# NixOS system build module from Nixos anywhere
module "system-build" {
  source    = "github.com/nix-community/nixos-anywhere//terraform/nix-build"
  attribute = ".#nixosConfigurations.blog.config.system.build.toplevel"
}

# Module for disk partitioning script
module "disko" {
  source    = "github.com/nix-community/nixos-anywhere//terraform/nix-build"
  attribute = ".#nixosConfigurations.blog.config.system.build.diskoScript"
}

# Module for installing NixOS on the provisioned server
module "install" {
  source            = "github.com/nix-community/nixos-anywhere//terraform/install"
  nixos_system      = module.system-build.result.out
  nixos_partitioner = module.disko.result.out
  target_host       = hcloud_server.blog.ipv4_address
}
