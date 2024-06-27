{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    disko = {
      url = "github:nix-community/disko";
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

    landing = {
      url = "git+ssh://git@github.com/FlakM/coder_kata";
      inputs.nixpkgs.follows = "nixpkgs";
    };

  };

  outputs = { nixpkgs, disko, backend, static, ... }@attrs:
    let
      # The system we are building for. It will not work on other systems.
      # One might use flake-utils to make it work on other systems.
      system = "x86_64-linux";

      # Import the Nix packages from nixpkgs
      pkgs = import nixpkgs { inherit system; };

      # Integration that are run using kvm virtual machines
      integration = import ./integration.nix {
        makeTest = import (nixpkgs + "/nixos/tests/make-test-python.nix");
        inherit pkgs;
        inherit backend;
        inherit static;
      };
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
          ./configuration.nix # Include custom configuration
          ./plausible.nix
        ];
      };

      # Merge the checks from the backend flake into the root flake's checks
      checks.${system} = nixpkgs.lib.recursiveUpdate
        (backend.checks.${system} or { })
        {
          itg = integration;
        };

      # Development shell environment
      # It will include the packages specified in the buildInputs attribute
      # once the shell is entered using `nix develop` or direnv integration.
      devShell.${system} = pkgs.mkShell {
        DATABASE_PATH = "./db.sqlite";
        DATABASE_URL = "sqlite://./db.sqlite";

        buildInputs = with pkgs; [
          opentofu # provisioning tool for the OpenTofu project
          ponysay # just for fun run `ponysay hello`
          hugo
          sqlx-cli
        ];
      };
    };
}
