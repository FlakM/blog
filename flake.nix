{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    disko = {
      url = "github:nix-community/disko";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    # Add the backend flake as an input
    backend = {
      url = "./backend"; # Points to the flake in the backend directory
      inputs.nixpkgs.follows = "nixpkgs";
    };

    static = {
      url = "./blog-static";
      inputs.nixpkgs.follows = "nixpkgs";
    };

  };

  outputs = { nixpkgs, disko, sops-nix, backend, static, ... }@attrs:
    let
      # The system we are building for. It will not work on other systems.
      # One might use flake-utils to make it work on other systems.
      system = "x86_64-linux";

      # Import the Nix packages from nixpkgs
      pkgs = import nixpkgs { inherit system; };

      # Integration that are run using kvm virtual machines
      integration = pkgs.nixosTest (import ./integration.nix {
        inherit pkgs system backend static;
      });

      # Browser E2E tests using nixosTest framework
      browser-e2e-test = import ./browser-e2e.nix { inherit pkgs backend static; };

      # Python with properly compiled packages
      pythonEnv = pkgs.python3.withPackages (ps: with ps; [
        playwright
        termcolor
        setuptools
        greenlet
      ]);

      # Browser E2E test script
      browser-e2e-script = pkgs.writeShellScriptBin "browser-e2e-test" ''
        set -e
        echo "Running browser E2E tests..."
        
        # Set up environment for Playwright
        export PLAYWRIGHT_BROWSERS_PATH="${pkgs.playwright-driver.browsers}"
        export PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS=true
        
        # Check if we have a display or need xvfb-run
        if [[ -z "$DISPLAY" ]]; then
          echo "No display found, using xvfb-run..."
          exec ${pkgs.xvfb-run}/bin/xvfb-run -a -s '-screen 0 1280x720x24' \
            ${pythonEnv}/bin/python3 ${./browser_e2e_tests.py}
        else
          echo "Display found, running directly..."
          exec ${pythonEnv}/bin/python3 ${./browser_e2e_tests.py}
        fi
      '';
    in
    {
      # Define a formatter package for the system to enable `nix fmt`
      formatter.${system} = pkgs.nixpkgs-fmt;


      # NixOS configuration for the 'blog' system
      nixosConfigurations.blog = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = attrs;
        modules = [
          disko.nixosModules.disko # Include disko module
          sops-nix.nixosModules.sops # Include sops module
          ./configuration.nix # Include custom configuration
          ./plausible.nix
        ];
      };

      # Merge the checks from the backend flake into the root flake's checks
      checks.${system} = nixpkgs.lib.recursiveUpdate
        (backend.checks.${system} or { })
        {
          itg = integration;
          browser-e2e = browser-e2e-test;
        };

      # Development shell environment
      # It will include the packages specified in the buildInputs attribute
      # once the shell is entered using `nix develop` or direnv integration.
      devShells.${system}.default = pkgs.mkShell {
        DATABASE_URL = "postgresql://blog:blog@localhost:5432/blog";
        PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
        PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
        SQLX_OFFLINE = "true";

        buildInputs = with pkgs; [
          # Rust toolchain from backend
          rustc
          cargo
          rustfmt
          clippy
          rust-analyzer
          pkg-config
          openssl
          # Root project tools
          opentofu # provisioning tool for the OpenTofu project
          ponysay # just for fun run `ponysay hello`
          hugo
          sqlx-cli
          python3
          python3Packages.cairosvg
          python3Packages.pillow
          python3Packages.svgwrite
          # Browser E2E testing tools
          browser-e2e-script
          xvfb-run
          python3Packages.playwright
          python3Packages.greenlet
          python3Packages.termcolor
        ];
      };
    };
}
