{ pkgs, backend, static }:

pkgs.nixosTest {
  name = "browser-e2e";
  
  nodes = {
    server = {
      imports = [
        backend.nixosModules.x86_64-linux.default
        static.nixosModules.x86_64-linux.default
      ];
      
      virtualisation.graphics = false;
      
      # Add fake DNS entries for testing
      networking.extraHosts = ''
        127.0.0.1 server
      '';
      
      networking.firewall = {
        enable = true;
        allowedTCPPorts = [ 80 443 ];
      };
      
      # Python with properly compiled packages
      environment.systemPackages = let
        pythonEnv = pkgs.python3.withPackages (ps: with ps; [
          playwright
          termcolor
          setuptools
          greenlet
        ]);
      in [
        pkgs.xvfb-run
        pythonEnv
        pkgs.chromium
        pkgs.firefox
      ];
      
      # Add system dependencies for Playwright browsers
      environment.variables = {
        PLAYWRIGHT_BROWSERS_PATH = "${pkgs.playwright-driver.browsers}";
        PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS = "true";
      };

      services = {
        backend = {
          enable = true;
          domain = "server";
          posts_path = "${pkgs.writeText "test-posts.json" ''
            [
              {
                "title": "Automating the pain away",
                "slug": "automate_boring_stuff",  
                "description": "A test blog post for integration testing",
                "date": "2024-01-01T12:00:00Z",
                "featuredImage": null,
                "tags": ["test", "integration"],
                "url": "http://server/posts/automate_boring_stuff"
              }
            ]
          ''}";
        };

        static-website = {
          enable = true;
          domain = "server";
        };
        
        nginx.enable = true;

        # Configure PostgreSQL
        postgresql = {
          enable = true;
          package = pkgs.postgresql_15;
          
          ensureDatabases = [ "blog" ];
          ensureUsers = [
            {
              name = "blog";
              ensureDBOwnership = true;
            }
          ];
          
          authentication = pkgs.lib.mkOverride 10 ''
            local   blog        blog                    trust
            host    blog        blog    127.0.0.1/32    trust
            host    blog        blog    ::1/128         trust
            local   all         all                     trust
            host    all         all     127.0.0.1/32    ident
            host    all         all     ::1/128         ident
          '';
        };
      };
    };
  };
  
  extraPythonPackages = p: [ p.termcolor p.playwright p.greenlet ];

  
  testScript = ''
    start_all()
    
    server.wait_for_open_port(80)
    server.wait_for_open_port(3000)
    
    # Wait for services to be ready
    server.wait_until_succeeds("curl -f http://server/api/health")
    server.wait_until_succeeds("curl -f http://server/")
    
    # Now run the browser E2E tests directly on server
    # Copy browser test script to server
    server.copy_from_host("${./browser_e2e_tests.py}", "/tmp/browser_e2e_tests.py")
    
    # Run browser E2E tests on server
    server.succeed("""
      export PLAYWRIGHT_BROWSERS_PATH="${pkgs.playwright-driver.browsers}"
      export PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS=true
      cd /tmp
      ${pkgs.xvfb-run}/bin/xvfb-run -a -s '-screen 0 1280x720x24' \
        ${pkgs.python3.withPackages (ps: with ps; [
          playwright
          termcolor
          setuptools
          greenlet
        ])}/bin/python3 \
        browser_e2e_tests.py --url http://server
    """)
    
    # Copy screenshots from VM to host using NixOS test framework
    server.succeed("ls -la /tmp/playwright-screenshots/ || echo 'No screenshots directory found'")
    server.copy_from_vm("/tmp/playwright-screenshots", ".")  
  '';
}
