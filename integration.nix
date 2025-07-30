{ pkgs
, system
, backend
, static
}:
let
  test_port = 80;
  sharedModule = {
    # Since it's common for CI not to have $DISPLAY available, we have to explicitly tell the tests "please don't expect any screen available"
    virtualisation.graphics = false;
  };

  # Test blog posts JSON data
  testBlogPosts = pkgs.writeText "test-posts.json" ''
    [
      {
        "title": "Test Blog Post",
        "slug": "test-post",  
        "description": "A test blog post for integration testing",
        "date": "2024-01-01T12:00:00Z",
        "featuredImage": null,
        "tags": ["test", "integration"],
        "url": "http://server/posts/test-post"
      }
    ]
  '';
in
{
  name = "integration";

  nodes = {
    server = {
      imports = [
        sharedModule
        backend.nixosModules.x86_64-linux.default
        static.nixosModules.x86_64-linux.default
      ];
      networking.firewall = {
        enable = true;
        allowedTCPPorts = [ 80 443 ];
      };

      services = {
        backend = {
          enable = true;
          domain = "server";
          posts_path = "${testBlogPosts}";
        };

        static-website = {
          enable = true;
          domain = "server";
        };
        nginx.enable = true;

        # Configure PostgreSQL (required by backend)
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
    client = {
      imports = [ sharedModule ];
    };
  };


  extraPythonPackages = p: [ p.termcolor ];

  testScript = ''
    start_all()
    import sys
    from termcolor import cprint

    server.wait_for_open_port(${toString test_port})
    server.wait_for_open_port(3000)

    expected = "OK"

    actual = client.succeed(
            "${pkgs.curl}/bin/curl -vv http://server/api/health"
    )

    if expected != actual:
        cprint("Test failed unexpected result: " + actual, "red", attrs=["bold"], file=sys.stderr)
        sys.exit(1)
  '';
}
