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

provider "cloudflare" {
  # Uses CLOUDFLARE_API_KEY and CLOUDFLARE_EMAIL environment variables
}

# Configuration for SSH key to be used with Hetzner Cloud instances
resource "hcloud_ssh_key" "yubi" {
  name       = "foo" 
  public_key = chomp(file("~/.ssh/id_rsa_yubikey.pub"))  
}

# Define a Hetzner Cloud Firewall
resource "hcloud_firewall" "web_firewall" {
  name = "web-firewall"

  # Allow TCP Port 443 (HTTPS)
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = "443"
    source_ips = ["0.0.0.0/0", "::/0"]  # Allow from any IP
  }

  # Allow TCP Port 80 (HTTP)
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = "80"
    source_ips = ["0.0.0.0/0", "::/0"]  # Allow from any IP
  }

  # Allow outgoing TCP to *:80 and *:443
  rule {
    direction = "out"
    protocol  = "tcp"
    port      = "80"
    destination_ips = ["0.0.0.0/0", "::/0"]
  }

  rule {
    direction = "out"
    protocol  = "tcp"
    port      = "443"
    destination_ips = ["0.0.0.0/0", "::/0"]
  }

  # Allow UDP from :41641 to *:*
  rule {
    direction = "in"
    protocol  = "udp"
    port      = "41641"
    source_ips = ["0.0.0.0/0", "::/0"]
  }

  # Allow UDP to *:3478
  rule {
    direction = "out"
    protocol  = "udp"
    port      = "3478"
    destination_ips = ["0.0.0.0/0", "::/0"]
  }

  # Allow WireGuard UDP port 51820
  rule {
    direction = "in"
    protocol  = "udp"
    port      = "51820"
    source_ips = ["0.0.0.0/0", "::/0"]
  }
}

# Define a Hetzner Cloud Server resource for the blog
resource "hcloud_server" "blog" {
  name        = "blog-instance"
  image       = "ubuntu-22.04"   # After provisioning, NixOS will be installed see @install
  server_type = "cx33"           # Intel ® / AMD 4vcpu, 8GB RAM, 80GB SSD
  location    = "fsn1"
  ssh_keys    = [hcloud_ssh_key.yubi.id]  # SSH keys associated with the server
  # Associate the firewall
  #firewall_ids = [hcloud_firewall.web_firewall.id]
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

resource "cloudflare_dns_record" "blog_nginx" {
  zone_id = var.ZONE_ID
  name    = "blog.flakm.com"
  content = hcloud_server.blog.ipv4_address
  type    = "A"
  ttl     = 300
  proxied = false
}

resource "cloudflare_dns_record" "tata_nginx" {
  zone_id = var.ZONE_ID
  name    = "tata.flakm.com"
  content = hcloud_server.blog.ipv4_address
  type    = "A"
  ttl     = 300
  proxied = false
}

resource "cloudflare_dns_record" "plausible_nginx" {
  zone_id = var.ZONE_ID
  name    = "plausible.flakm.com"
  content = hcloud_server.blog.ipv4_address
  type    = "A"
  ttl     = 300
  proxied = false
}

resource "cloudflare_dns_record" "jellyfin_public_nginx" {
  zone_id = var.ZONE_ID
  name    = "jellyfin.public.flakm.com"
  content = hcloud_server.blog.ipv4_address
  type    = "A"
  ttl     = 300
  proxied = false
}

# Removed duplicate/invalid CNAME and page rule resources
# Keeping only flakm_root CNAME and page rule defined below


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

resource "cloudflare_dns_record" "flakm_root" {
  zone_id         = var.ZONE_ID
  name            = "@"
  content         = "blog.flakm.com"
  type            = "CNAME"
  ttl             = 1
  proxied         = true
}

resource "cloudflare_page_rule" "flakm_root" {
  zone_id  = var.ZONE_ID
  target   = "https://flakm.com/*"
  priority = 1
  status   = "active"

  actions = {
    cache_level = "cache_everything"
  }
}
