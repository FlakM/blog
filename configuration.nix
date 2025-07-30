{ system, lib, pkgs, modulesPath, backend, static, ... }: {
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



  nix.gc = {
    automatic = true;
    dates = "weekly";
    options = "--delete-older-than 10d";
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
    allowedTCPPorts = [ 80 443 4317 4318 ]; # Added OTLP ports
  };


  services = {
    backend = {
      enable = true;
      domain = "blog.flakm.com";
      posts_path = "${static.packages.x86_64-linux.default}/bloglist.json";
    };

    static-website = {
      enable = true;
      domain = "blog.flakm.com";
    };

    postgresql = {
      enable = true;
      package = pkgs.postgresql_15;

      ensureDatabases = [ "blog" ];
      ensureUsers = [
        {
          name = "blog";
          ensureDBOwnership = true;
        }
      ];

      # Set a password for the blog user (this should be changed in production)
      initialScript = pkgs.writeText "backend-initScript" ''
        CREATE USER IF NOT EXISTS blog WITH PASSWORD 'blog';
        ALTER USER blog CREATEDB;
        GRANT ALL PRIVILEGES ON DATABASE blog TO blog;
      '';

      authentication = pkgs.lib.mkOverride 10 ''
        # Allow local connections from the blog user
        local   blog        blog                    trust
        host    blog        blog    127.0.0.1/32    trust
        host    blog        blog    ::1/128         trust
        # Default entries
        local   all         all                     trust
        host    all         all     127.0.0.1/32    ident
        host    all         all     ::1/128         ident
      '';
    };

    # OpenTelemetry Collector for observability
    opentelemetry-collector = {
      enable = true;
      package = pkgs.opentelemetry-collector-contrib;

      settings = {
        receivers = {
          otlp = {
            protocols = {
              grpc = {
                endpoint = "0.0.0.0:4317";
              };
              http = {
                endpoint = "0.0.0.0:4318";
              };
            };
          };
          prometheus = {
            config = {
              scrape_configs = [
                {
                  job_name = "backend-metrics";
                  static_configs = [
                    {
                      targets = [ "localhost:3000" ];
                    }
                  ];
                  scrape_interval = "15s";
                  metrics_path = "/metrics";
                }
              ];
            };
          };
        };

        processors = {
          batch = { };
          resource = {
            attributes = [
              {
                key = "service.name";
                value = "blog-backend";
                action = "upsert";
              }
              {
                key = "deployment.environment";
                value = "production";
                action = "upsert";
              }
            ];
          };
        };

        exporters = {
          # For debugging - logs to systemd journal
          logging = {
            loglevel = "debug";
          };
          # Coralogix OTLP exporter
          coralogix = {
            domain = "coralogix.com";
            private_key = builtins.readFile "/var/secrets/coralogix_key";
            application_name = "blog-system";
            subsystem_name = "backend";
          };
          # Generic OTLP HTTP exporter as fallback
          otlphttp = {
            endpoint = "https://otlp.coralogix.com:443/v1/traces";
            headers = {
              "Authorization" = "Bearer ${builtins.readFile "/var/secrets/coralogix_key"}";
            };
          };
        };

        service = {
          pipelines = {
            traces = {
              receivers = [ "otlp" ];
              processors = [ "resource" "batch" ];
              exporters = [ "logging" "coralogix" ];
            };
            metrics = {
              receivers = [ "otlp" "prometheus" ];
              processors = [ "resource" "batch" ];
              exporters = [ "logging" "coralogix" ];
            };
            logs = {
              receivers = [ "otlp" ];
              processors = [ "resource" "batch" ];
              exporters = [ "logging" "coralogix" ];
            };
          };
        };
      };
    };

    nginx = {
      enable = true;
      appendHttpConfig = ''
        log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                        '$status $body_bytes_sent "$http_referer" '
                        '"$http_user_agent" "$http_x_forwarded_for" "$host"';
        access_log /var/log/nginx/access.log main;
        error_log /var/log/nginx/error.log;
      '';



      virtualHosts."blog.flakm.com" = {
        forceSSL = true;
        enableACME = true;
      };
      virtualHosts."tata.flakm.com" = {
        forceSSL = true;
        enableACME = true;
        root = "/var/www/tata";
      };
      virtualHosts."fedi.flakm.com" = {
        forceSSL = true;
        enableACME = true;
      };
      commonHttpConfig =
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
    };
  };

  security.acme = {
    acceptTerms = true;
    certs = {
      "blog.flakm.com".email = "me@flakm.com";
      "fedi.flakm.com".email = "me@flakm.com";
      "tata.flakm.com".email = "me@flakm.com";
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
    postgresql_15
    ponysay
  ];

  services.tailscale = {
    enable = true;
    openFirewall = true;
  };


}
