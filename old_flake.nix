{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    backend = {
      url = "github:FlakM/backend/7a09966d9d6218a7f76f6cd38f52c84758fcf7a5";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, ... }@attrs:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in
    {
      formatter.${system} = pkgs.nixpkgs-fmt;
      nixosConfigurations.blog = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = attrs;
        modules = [ ./configuration.nix ];
      };


      devShell.x86_64-linux = pkgs.mkShell {
        buildInputs = with pkgs; [
          opentofu
          cowsay
        ];
      };
    };


}
