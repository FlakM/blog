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
        pkgs = import nixpkgs { inherit system; };
        website = pkgs.stdenv.mkDerivation {
          pname = "static-website";
          version = "0.1.0";
          src = ./.;
          nativeBuildInputs = with pkgs; [ hugo git ];
          buildPhase = "mkdir -p themes/hugo-coder/ && cp -r ${theme}/* themes/hugo-coder/ && ls -al themes && HUGO_ENV=production hugo --minify";
          installPhase = "cp -r public $out";
          submodules = [ theme ];
        };

      in {

        # This is a NixOS module that can be imported into a NixOS
        # configuration to enable the static-website service
        nixosModules.default = { config, lib, ... }: with lib;
          let
            cfg = config.services.static-website;
          in
          {
            options.services.static-website = {
              enable = mkEnableOption "Enables the static website";

              domain = mkOption rec {
                type = types.str;
                default = "localhost";
                example = default;
                description = "The domain name";
              };
            };

            config = mkIf cfg.enable {
              services.nginx.virtualHosts.${cfg.domain} = {
                locations."/" = { 
                    root = "${website}";
                    tryFiles = "$uri $uri/ =404";
                    extraConfig = ''
                      add_header Cache-Control "public, max-age=31536000, immutable";
                    '';
                    priority = 100; # set a high priority to make it the last location
              };
            };
          };
        };

        packages.default = website;

        apps = {
          default = flake-utils.lib.mkApp {
            drv = website;
          };
        };

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            hugo
          ];
        };
      }
    );


}
