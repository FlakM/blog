{ system
, pkgs
, makeTest
, self
}:
let
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
      networking.firewall = {
        enable = true;
        allowedTCPPorts = [ 80 443 ];
      };

      imports = [ sharedModule self.nixosModules.${system}.default ];
      services.backend = {
        enable = true;
        domain = "server";
      };
      services.nginx.enable = true;
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

    server.wait_for_open_port(80)
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
