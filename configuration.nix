{ system, pkgs, modulesPath, backend, static, ... }: {
  imports = [
    # Adds availableKernelModules, kernelModules for instances running under QEMU (Ie Hetzner Cloud)
    (modulesPath + "/profiles/qemu-guest.nix")
    # Contains the configuration for the disk layout
    ./disk-config.nix
    backend.nixosModules.x86_64-linux.default
    static.nixosModules.x86_64-linux.default
  ];

  # enable experimental features flakes and nix-command
  nix = {
    extraOptions = ''
      experimental-features = nix-command flakes
    '';
  };


  # Use the GRUB 2 boot loader.
  boot.loader.grub = {
    # no need to set devices, disko will add all devices that have a EF02 partition to the list already
    # devices = [ ];
    efiSupport = true;
    efiInstallAsRemovable = true;
  };

  services.openssh = {
    enable = true;
  };

  # Enable ssh access to the root user
  users.users.root.openssh.authorizedKeys.keys = [
    "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDh6bzSNqVZ1Ba0Uyp/EqThvDdbaAjsJ4GvYN40f/p9Wl4LcW/MQP8EYLvBTqTluAwRXqFa6fVpa0Y9Hq4kyNG62HiMoQRjujt6d3b+GU/pq7NN8+Oed9rCF6TxhtLdcvJWHTbcq9qIGs2s3eYDlMy+9koTEJ7Jnux0eGxObUaGteQUS1cOZ5k9PQg+WX5ncWa3QvqJNxx446+OzVoHgzZytvXeJMg91gKN9wAhKgfibJ4SpQYDHYcTrOILm7DLVghrcU2aFqLKVTrHSWSugfLkqeorRadHckRDr2VUzm5eXjcs4ESjrG6viKMKmlF1wxHoBrtfKzJ1nR8TGWWeH9NwXJtQ+qRzAhnQaHZyCZ6q4HvPlxxXOmgE+JuU6BCt6YPXAmNEMdMhkqYis4xSzxwWHvko79NnKY72pOIS2GgS6Xon0OxLOJ0mb66yhhZB4hUBb02CpvCMlKSLtvnS+2IcSGeSQBnwBw/wgp1uhr9ieUO/wY5K78w2kYFhR6Iet55gutbikSqDgxzTmuX3Mkjq0L/MVUIRAdmOysrR2Lxlk692IrNYTtUflQLsSfzrp6VQIKPxjfrdFhHIfbPoUdfMf+H06tfwkGONgcej56/fDjFbaHouZ357wcuwDsuMGNRCdyW7QyBXF/Wi28nPq/KSeOdCy+q9KDuOYsX9n/5Rsw== cardno:000614320136"
  ];

  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 80 443 ];
  };

  services = {
    backend = {
      enable = true;
      domain = "fedi.flakm.com";
      posts_path = "${static.packages.x86_64-linux.default}/bloglist.json";
    };

    static-website = {
      enable = true;
      domain = "blog.flakm.com";
    };

    nginx = {
      enable = true;
      virtualHosts."blog.flakm.com" = {
        forceSSL = true;
        enableACME = true;
      };
      virtualHosts."fedi.flakm.com" = {
        forceSSL = true;
        enableACME = true;
      };
    };
  };

  security.acme = {
    acceptTerms = true;
    certs = {
      "blog.flakm.com".email = "me@flakm.com";
      "fedi.flakm.com".email = "me@flakm.com";
    };
  };

  system.stateVersion = "23.11";

  environment.systemPackages = with pkgs; [
    git
    vim
    tmux
    htop
    curl
    jq
    sqlite
    ponysay
  ];

  services.tailscale = {
    enable = true;
    openFirewall = true;
  };


}
