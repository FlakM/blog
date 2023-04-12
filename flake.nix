{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    ananke = {
      url = "github:FlakM/gohugo-theme-ananke";
      flake = true;
    };
  };
  
  outputs = { self, nixpkgs, flake-utils, ananke, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system;};
      in rec {

      packages.default = pkgs.stdenv.mkDerivation {
        pname = "static-website";
        version = "0.1.0";
        src = ./.;
        nativeBuildInputs = with pkgs; [ hugo git ];
        buildPhase = "mkdir -p themes/ananke/ && cp -r ${ananke}/* themes/ananke/ && ls -al themes && HUGO_ENV=production hugo --minify";
        installPhase = "cp -r public $out";
        submodules = [ ananke ];
      };
      
      apps.default = flake-utils.lib.mkapp {
        drv = packages.default;
      };


        devShell= pkgs.mkShell {
          buildInputs = with pkgs; [
            hugo
          ];
        };
      }
    );


}
