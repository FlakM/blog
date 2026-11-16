{
  description = "ssologin redirect demo listener and orchestration (authorized security testing)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
      pkgsFor = system: import nixpkgs { inherit system; };
      demoFor = system: import ./package.nix { pkgs = pkgsFor system; };
    in
    {
      packages = forAllSystems (system:
        let demo = demoFor system;
        in {
          default = demo.demo;
          inherit (demo) listener demo;
        });

      apps = forAllSystems (system:
        let demo = demoFor system;
        in {
          default = {
            type = "app";
            program = "${demo.demo}/bin/demo-ssologin-takeover";
          };
        });

      devShells = forAllSystems (system:
        let pkgs = pkgsFor system;
        in {
          default = pkgs.mkShell {
            packages = with pkgs; [ python3 curl jq nginx nixfmt-rfc-style ];
          };
        });

      nixosModules.default = import ./module.nix;
      nixosModules.ssologin-demo = self.nixosModules.default;
    };
}
