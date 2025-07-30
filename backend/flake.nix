{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
        };

        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;

        sqlFilter = path: _type: null != builtins.match ".*sql$" path;
        sqlxFilter = path: _type: null != builtins.match ".*\\.sqlx/.*\\.json$" path;
        sqlOrCargoOrSqlx = path: type: (sqlFilter path type) || (sqlxFilter path type) || (craneLib.filterCargoSources path type);

        # Just use the full source for now to avoid filtering issues
        src = craneLib.path ./.;

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          buildInputs = [ pkgs.openssl ];
          nativeBuildInputs = [ pkgs.pkg-config ];
          # Disable offline mode for SQLx during builds due to missing cache
          SQLX_OFFLINE = "true";
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        server = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          doCheck = false;
          nativeBuildInputs = (commonArgs.nativeBuildInputs or [ ]) ++ [
            pkgs.sqlx-cli
          ];

        });

      in
      {
        # This is a NixOS module that can be imported into a NixOS
        # configuration to enable the backend service
        nixosModules.default = { config, lib, ... }: with lib;
          let
            cfg = config.services.backend;
          in
          {
            options.services.backend = {
              enable = mkEnableOption "Enables the backend HTTP service";

              domain = mkOption rec {
                type = types.str;
                default = "localhost";
                example = default;
                description = "The domain name";
              };

              posts_path = mkOption {
                type = types.path;
                default = "./posts.json";
                description = "The path to the posts json file";
              };
            };

            config = mkIf cfg.enable {
              systemd.services.backend = {
                wantedBy = [ "multi-user.target" ];
                after = [ "postgresql.service" "postgresql-setup.service" ];
                requires = [ "postgresql.service" ];
                serviceConfig = {
                  Restart = "on-failure";
                  ExecStart = "${server}/bin/backend ${config.services.backend.posts_path}";
                };
                environment = {
                  "RUST_LOG" = "INFO";
                  "DATABASE_URL" = "postgresql://blog:blog@localhost:5432/blog";
                  "OTEL_EXPORTER_OTLP_ENDPOINT" = "http://localhost:4317";
                  "OTEL_SERVICE_NAME" = "blog-backend";
                  "OTEL_SERVICE_VERSION" = "1.0.0";
                  "OTEL_RESOURCE_ATTRIBUTES" = "deployment.environment=production";
                };
              };

              services.nginx.virtualHosts.${cfg.domain} = {
                locations."/api/health" = {
                  proxyPass = "http://127.0.0.1:3000/health";
                  extraConfig = ''
                    proxy_set_header Host $host;
                    proxy_set_header X-Real-IP $remote_addr;
                    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
                    proxy_set_header X-Forwarded-Proto $scheme;
                  '';
                  priority = 10;
                };
                locations."/api/metrics" = {
                  proxyPass = "http://127.0.0.1:3000/metrics";
                  extraConfig = ''
                    proxy_set_header Host $host;
                    proxy_set_header X-Real-IP $remote_addr;
                    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
                    proxy_set_header X-Forwarded-Proto $scheme;
                  '';
                  priority = 10;
                };
                locations."~ ^/api/likes?/" = {
                  proxyPass = "http://127.0.0.1:3000";
                  extraConfig = ''
                    proxy_set_header Host $host;
                    proxy_set_header X-Real-IP $remote_addr;
                    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
                    proxy_set_header X-Forwarded-Proto $scheme;
                    rewrite ^/api(/.*) $1 break;
                  '';
                  priority = 10;
                };
              };
            };
          };
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit server;

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          fmt = craneLib.cargoFmt {
            inherit src;
          };

          # audit = craneLib.cargoAudit {
          #   inherit src advisory-db;
          # };

          nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
            preCheck = ''
              export RUST_LOG=debug
            '';
          });

          # Integration that are run using kvm virtual machines  
          nixos-integration = pkgs.nixosTest (import ./nixos-test.nix {
            inherit system pkgs self;
          });
        };

        # This will enable running `nix run` to start the server
        packages = {
          default = server;
        };

        devShell = with pkgs; mkShell { };
      });

}
