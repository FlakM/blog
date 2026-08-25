{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
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
          nativeBuildInputs = with pkgs; [ hugo git tailwindcss_4 ];
          buildPhase = "mkdir -p themes/hugo-coder/ && cp -r ${theme}/* themes/hugo-coder/ && tailwindcss -i assets/css/tailwind.css -o assets/css/site.css --minify && HUGO_ENV=production hugo --minify";
          installPhase = "cp -r public $out";
          submodules = [ theme ];
        };
        discussionTemplateTest = pkgs.runCommand "discussion-template-test"
          {
            nativeBuildInputs = with pkgs; [ hugo git tailwindcss_4 ];
          } ''
          cp -r ${./.} site
          chmod -R u+w site
          mkdir -p site/themes/hugo-coder site/test-content/posts
          cp -r ${theme}/* site/themes/hugo-coder/
          cp ${pkgs.writeText "with-links.md" ''
            ---
            title: Discussion links fixture
            date: 2024-08-25
            discussion_links:
              - source: hacker_news
                url: https://news.ycombinator.com/item?id=1
              - source: reddit
                url: https://www.reddit.com/r/rust/comments/example
              - source: lobsters
                label: Lobsters thread
                url: https://lobste.rs/s/example
            ---
            Fixture.
          ''} site/test-content/posts/with-links.md
          cp ${pkgs.writeText "one-link.md" ''
            ---
            title: One discussion link fixture
            date: 2024-08-25
            discussion_links:
              - source: hacker_news
                url: https://news.ycombinator.com/item?id=2
            ---
            Fixture.
          ''} site/test-content/posts/one-link.md
          cp ${pkgs.writeText "without-links.md" ''
            ---
            title: Empty discussion links fixture
            date: 2024-08-25
            ---
            Fixture.
          ''} site/test-content/posts/without-links.md
          tailwindcss -i site/assets/css/tailwind.css -o site/assets/css/site.css --minify
          hugo --source site --contentDir "$PWD/site/test-content" --destination "$TMPDIR/public"
          grep -q 'Hacker News' "$TMPDIR/public/posts/with-links/index.html"
          grep -q 'Reddit' "$TMPDIR/public/posts/with-links/index.html"
          grep -q 'Lobsters thread' "$TMPDIR/public/posts/with-links/index.html"
          grep -q 'discussion-section-with-links' "$TMPDIR/public/posts/with-links/index.html"
          grep -q 'discussion-section-one-link' "$TMPDIR/public/posts/one-link/index.html"
          grep -q 'bg-panel p-5 sm:p-6 hidden' "$TMPDIR/public/posts/without-links/index.html"
          grep -q 'data-discussion-slug="without-links"' "$TMPDIR/public/posts/without-links/index.html"
          touch $out
        '';

      in
      {

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
                    add_header Cache-Control "public, max-age=3600";
                  '';
                  priority = 100; # set a high priority to make it the last location
                };
              };
            };
          };

        packages.default = website;

        checks.discussion-template = discussionTemplateTest;

        apps = {
          default = flake-utils.lib.mkApp {
            drv = website;
          };
        };

        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            hugo
            tailwindcss_4
          ];
        };
      }
    );


}
