{ config, lib, pkgs, backend, nixpkgs, ... }: {

  imports = [
    "${nixpkgs}/nixos/modules/virtualisation/amazon-image.nix"
    backend.nixosModules.x86_64-linux.default
  ];

  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 80 443 ];
  };

  services = {
    backend = {
      enable = true;
      domain = "blog.flakm.com";
    };

    nginx = {
      enable = true;
      virtualHosts."blog.flakm.com" = {
        forceSSL = true;
        enableACME = true;

        #extraConfig = ''
        #  # Set the cache control headers
        #  add_header Cache-Control "public, max-age=3600";
        #'';
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
      "blog.flakm.com".email = "maciej.jan.flak@gmail.com";
    };
  };

}
