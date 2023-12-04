{
  description = "Build a cargo project";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }: {
    formatter.x86_64-linux = nixpkgs.legacyPackages.x86_64-linux.nixpkgs-fmt;
  } //
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

        buildInputs = [
          # Add additional build inputs here
        ];
      };

      craneLibLLvmTools = craneLib.overrideToolchain
        (fenix.packages.${system}.complete.withComponents [
          "cargo"
          "llvm-tools"
          "rustc"
        ]);

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      # Build the actual crate itself, reusing the dependency
      # artifacts from above.
      server = craneLib.buildPackage (commonArgs // {
        inherit cargoArtifacts;
      });

    in
    {
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
            };

            services.nginx.virtualHosts.${cfg.domain} = {
              locations."/" = { proxyPass = "http://127.0.0.1:3000"; };
            };
          };
        };
      checks = {
        # Build the crate as part of `nix flake check` for convenience
        inherit server;

        # Run clippy (and deny all warnings) on the crate source,
        # again, resuing the dependency artifacts from above.
        #
        # Note that this is done as a separate derivation so that
        # we can block the CI if there are issues here, but not
        # prevent downstream consumers from building our crate by itself.
        server-clippy = craneLib.cargoClippy (commonArgs // {
          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });

        server-doc = craneLib.cargoDoc (commonArgs // {
          inherit cargoArtifacts;
        });

        # Check formatting
        server-fmt = craneLib.cargoFmt {
          inherit src;
        };

        # Audit dependencies
        server-audit = craneLib.cargoAudit {
          inherit src advisory-db;
        };

        # Run tests with cargo-nextest
        # Consider setting `doCheck = false` on `server` if you do not want
        # the tests to run twice
        server-nextest = craneLib.cargoNextest (commonArgs // {
          inherit cargoArtifacts;
          partitions = 1;
          partitionType = "count";
        });


        integration = import ./nixos-test.nix {
          makeTest = import (nixpkgs + "/nixos/tests/make-test-python.nix");
          inherit system;
          inherit pkgs;
          inherit self;
        };

        server-coverage = craneLib.cargoTarpaulin (commonArgs // {
          inherit cargoArtifacts;
        });
      };

      packages = {
        default = server;
        server-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs // {
          inherit cargoArtifacts;
        });
      };

      apps.default = flake-utils.lib.mkApp {
        drv = server;
      };

      devShells.default = pkgs.mkShell { };
    });

}
