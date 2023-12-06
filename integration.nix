{ pkgs
, makeTest
, backend
, static
}:
let
  test_port = 80;
  sharedModule = {
    # Since it's common for CI not to have $DISPLAY available, we have to explicitly tell the tests "please don't expect any screen available"
    virtualisation.graphics = false;
  };
in
makeTest
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
          domain = "blog.flakm.com";
        };

        static-website = {
          enable = true;
          domain = "blog.flakm.com";
        };
        nginx.enable = true;
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

    expected = "<h1>Hello, World!</h1>"

    actual = client.succeed(
            "${pkgs.curl}/bin/curl -vv http://server/api"
    )

    if expected != actual:
        cprint("Test failed unexpected result: " + actual, "red", attrs=["bold"], file=sys.stderr)
        sys.exit(1)
  '';
}
{
  inherit pkgs;
  inherit (pkgs) system;
}
