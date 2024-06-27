+++ 
draft = false
date = 2024-06-27T08:37:45+01:00
title = "Securing the blog host"
slug = ""
authors = []
externalLink = ""

series = ["Simple personal blog"]

description="""
🔒 Securing My Server with Tailscale & Systemd! 🔧

I dive into setting up Tailscale VPN on Hetzner to secure SSH access and manage firewall rules effortlessly! 
🚀 I also share how I hardened my backend services with advanced systemd settings. Perfect for fellow tech enthusiasts looking to boost server security. 🛡️
"""

tags = ["nixos", "security", "devops", "blog"]

+++


## Setting up tailscale

One sad thing is that the host currently exposes SSH over the Internet.
There is also a risk of accidentally exposing another service.
To mitigate this risk, we can use modern VPNs and add additional firewalls on the Hetzner side.

To avoid the hassle of managing my keys, I’ll be using a generous [tailscale](https://tailscale.com/unplugged) free tier.
To enable tailscale, we’ll have to add one line in our configuration:

```nix
# configuration.nix
services.tailscale.enable = true;
```

After switching to this configuration, we can manually over ssh configure tailscale using `tailscale login` to attach a new node to our tail net.
Tailscale uses persistent ips that we can check by using `ip` command:

```bash
[root@nixos:~/blog]# ip addr show tailscale0
3: tailscale0: <POINTOPOINT,MULTICAST,NOARP,UP,LOWER_UP> mtu 1280 qdisc fq_codel state UNKNOWN group default qlen 500
    link/none
    inet 100.96.101.15/32 scope global tailscale0
       valid_lft forever preferred_lft forever
...


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

Even if I start some network service, it will not be accessible by accident like with sshd:

```bash
❯ ssh root@blog.flakm.com
ssh: connect to host blog.flakm.com port 22: Connection refused
```

But the connection via ssh will still be available if we use the tailscale's dns name:

{{< 
    figure src="/images/nixos_rust/ssh-pony.png" class="img-lg" 
    caption= "Cute and secure pony over ssh over tailscale connection"
>}}

## Hardening the backend service

Since we are using systemd under the hood, we can now change the backend service to have additional systemd settings:

```nix
systemd.services.backend = {
  serviceConfig = {
    Restart = "on-failure";
    ExecStart = "${server}/bin/backend ${config.services.backend.posts_path}";
    # dynamically allocate new user and release them when the service stops
    DynamicUser = true;
    # mounts an empty tmpfs read only filesystem over the the space-separated list of filesystem paths you pass it
    TemporaryFileSystem = "/:ro";
    # /var/lib/backend will be mounted to the service
    BindPaths = "/var/lib/backend";
    # ensures that directory backend exists under /var/lib and has correct ownership
    StateDirectory = "backend";
    # sets working directory of process to this value
    WorkingDirectory = "/var/lib/backend";
    # the entire file system hierarchy is mounted read-only, except for the API file system subtrees /dev, proc and /sys
    ProtectSystem = "strict";
    # the directories /home, /root and /run/user are made inaccessible and empty for processes invoked by this unit
    ProtectHome = true;
    # sets up a new file system namespace for the executed processes and mounts private /tmp and /var/tmp directories inside it
    PrivateTmp = true;
    # hat the service process and all its children can never gain new privileges through `execve()`
    NoNewPrivileges = true;
  };
  environment = {
    "RUST_LOG" = "INFO";
    "DATABASE_PATH" = "/var/lib/backend/db.sqlite3";
  };
};
```

You can read more about systemd features [here](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html).
I was surprised by the number of knobs one can turn with systemd in this department.

