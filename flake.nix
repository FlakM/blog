{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    theme = {
      url = "github:luizdepra/hugo-coder";
      flake = false;
    };
  };
  
  outputs = { self, nixpkgs, flake-utils, theme, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system;};
      in rec {


      packages.default = pkgs.stdenv.mkDerivation {
        pname = "static-website";
        version = "0.1.0";
        src = ./.;
        nativeBuildInputs = with pkgs; [ hugo git ];
        buildPhase = "mkdir -p themes/hugo-coder/ && cp -r ${theme}/* themes/hugo-coder/ && ls -al themes && HUGO_ENV=production hugo --minify";
        installPhase = "cp -r public $out";
        submodules = [ theme ];
      };

      apps = {
        default = flake-utils.lib.mkApp {
          drv = packages.default;
        };
      };



        devShell= pkgs.mkShell {
          buildInputs = with pkgs; [
            hugo
          ];
        };
      }
    );


}
