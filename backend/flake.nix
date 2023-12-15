{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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

        craneLib = crane.lib.${system};
        src = craneLib.cleanCargoSource (craneLib.path ./.);

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          buildInputs = [ pkgs.openssl ];
          nativeBuildInputs = [ pkgs.pkg-config ];
        };

        # Build *just* the cargo dependencies, so we can reuse
        # all of that work (e.g. via cachix) when running in CI
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual crate itself, reusing the dependency
        # artifacts from above.
        server = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          doCheck = false;
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
            };

            config = mkIf cfg.enable {
              systemd.services.backend = {
                wantedBy = [ "multi-user.target" ];
                serviceConfig = {
                  Restart = "on-failure";
                  ExecStart = "${server}/bin/quick-start";
                  DynamicUser = "yes";
                  RuntimeDirectory = "backend";
                  PermissionsStartOnly = true;
                };
                environment = {
                  "RUST_LOG" = "INFO";
                };
              };

              services.nginx.virtualHosts.${cfg.domain} = {
                locations."/" = { 
                  proxyPass = "http://127.0.0.1:3000"; 
                  extraConfig = "
                    proxy_set_header Host $host;
                  ";
                  priority = 10; # smaller number means higher priority
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

          audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `server` if you do not want
          # the tests to run twice
          nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });

          # Integration that are run using kvm virtual machines
          integration = import ./nixos-test.nix {
            makeTest = import (nixpkgs + "/nixos/tests/make-test-python.nix");
            inherit system;
            inherit pkgs;
            inherit self;
          };
        };

        # This will enable running `nix run` to start the server
        packages = {
          default = server;
        };

        devShell = with pkgs; mkShell {
          DATABASE_URL = "sqlite://db.sqlite";

        };
      });

}
