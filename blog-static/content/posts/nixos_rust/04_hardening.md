+++ 
draft = true
date = 2023-12-20T08:37:45+01:00
title = "Securing the blog host"
description = "A blog entry about hardening my nixos host"
slug = ""
authors = []
tags = []
categories = []
externalLink = ""

series = ["Simple personal blog"]
+++


## Setting up tailscale

One sad thing is that currently the host exposes ssh over the internet. There is also a risk of accidentaly exposing another service.
To mitigate this risk we can use modern vpns and add additional firewalls on hetzner side.

To avoid the hustle of managing my own keys I'll be using a generous tailscale's free tier.
To enable tailscale we'll have to add one line in our configuration:

```nix
# configuration.nix
services.tailscale.enable = true;
```

After switching to this configuration we can manually over ssh configure tailscale using `tailscale login` to attach new node to our tail net.
Tailscale uses persistant ips, we can check by using `ip` command:

```bash
[root@nixos:~/blog]# ip addr show tailscale0
3: tailscale0: <POINTOPOINT,MULTICAST,NOARP,UP,LOWER_UP> mtu 1280 qdisc fq_codel state UNKNOWN group default qlen 500
    link/none
    inet 100.96.101.15/32 scope global tailscale0
       valid_lft forever preferred_lft forever
...
```




## Exposing only the intended ports

We can observe the exposed ports using `nmap`

```bash
❯ nix run nixpkgs#nmap -- -sT  blog.flakm.com
PORT    STATE  SERVICE
22/tcp  open   ssh
80/tcp  open   http
443/tcp open   https
```

Now we can modify hetzner firewall rule: 

```terraform
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
}

# Define a Hetzner Cloud Server resource for the blog
resource "hcloud_server" "blog" {
  name        = "blog-instance"
  image       = "ubuntu-22.04"   # After provisioning, NixOS will be installed see @install
  server_type = "cpx11"          # AMD 2 vCPU, 2 GB RAM, 40 GB NVMe SSD
  location    = "fsn1"
  ssh_keys    = [hcloud_ssh_key.yubi.id]  # SSH keys associated with the server
  # 👇 Associate the firewall
  firewall_ids = [hcloud_firewall.web_firewall.id]
}
```


And now, after fast `tofu apply` the ssh is no longer publicly available:

```bash
# over public internet
❯ nix run nixpkgs#nmap -- -sT  blog.flakm.com
PORT    STATE  SERVICE
80/tcp  open   http
443/tcp open   https
# over tailscale
❯ nix run nixpkgs#nmap -- -sT  hetzner-blog
PORT    STATE SERVICE
22/tcp  open  ssh
80/tcp  open  http
443/tcp open  https
```

Even if I start some network service it will not be accessible by accident like with sshd:

```bash
❯ ssh root@blog.flakm.com
ssh: connect to host blog.flakm.com port 22: Connection refused
```

But the connection via ssh will be still available if we use the tailscale's dns name:

{{< 
    figure src="/images/nixos_rust/ssh-pony.png" class="img-lg" 
    caption= "Cute and secure pony over ssh over tailscale connection"
>}}
