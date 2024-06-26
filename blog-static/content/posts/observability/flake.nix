{
  description = "A Nix flake with a development shell that starts FHS";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

  outputs = { self, nixpkgs }: {
    devShells.default = nixpkgs.lib.mkShell {
      buildInputs = [
        (nixpkgs.pkgs.buildFHSUserEnvBubblewrap {
          name = "fhs-shell";
          targetPkgs = pkgs: [
            pkgs.bash
            pkgs.coreutils
            pkgs.bcc
            pkgs.bpftrace
            # Add other dependencies here
          ];
        })
      ];

      shellHook = ''
        echo "Starting FHS shell..."
      '';
    };
  };
}
