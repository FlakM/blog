---
title: "Consuming the flake"
date: 2023-04-06T20:58:49+02:00
draft: true

series: ["Breeding cobras"]

Summary: '
Example of how a flake module should be tested
'
---


So after the previous part we have a package that builds a final binary.
Let's expose this piece of amazing code as nix module.
You can read more about it [here](https://xeiaso.net/blog/nix-flakes-3-2022-04-07) but NixOs module is just a function that takes certain input and returns additional output that is evaluated
when the whole system gets built. 

To do so we should do the following in flake.nix (some parts are omitted for brevity you can check the full changes by `git show 283e3bc`)

```nix
{
  # ...

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }: {
    formatter.x86_64-linux = nixpkgs.legacyPackages.x86_64-linux.nixpkgs-fmt;
  } //
  #               👇 this used to be  eachDefaultSystem but now we build only on linux!
  flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
    let
    # ...
    in
    {
      nixosModules.default = { config, lib, ... }: with lib;
        let
          cfg = config.services.backend;
        in
        {
          options.services.backend = {
            enable = mkEnableOption "Enables the backend HTTP service";

            domain = mkOption rec {
              type = types.str;
              default = "localhost";
              example = default;
              description = "The domain name";
            };
          };

          config = mkIf cfg.enable {
            systemd.services.backend = {
              wantedBy = [ "multi-user.target" ];
              serviceConfig = {
                Restart = "on-failure";
                ExecStart = "${my-crate}/bin/quick-start";
                DynamicUser = "yes";
                RuntimeDirectory = "backend";
                RuntimeDirectoryMode = "0755";
                StateDirectory = "backend";
                StateDirectoryMode = "0700";
                CacheDirectory = "backend";
                CacheDirectoryMode = "0750";
              };
            };

            services.nginx.virtualHosts.${cfg.domain} = {
              locations."/" = { proxyPass = "http://127.0.0.1:3000"; };
            };
          };
        };
      checks = {
        #... 
        # 👇 this is new check that we added
        integration = import ./nixos-test.nix {
          makeTest = import (nixpkgs + "/nixos/tests/make-test-python.nix");
          inherit system;
          inherit pkgs;
          inherit self;
        };

        my-crate-coverage = craneLib.cargoTarpaulin (commonArgs // {
          inherit cargoArtifacts;
        });
      };
        #...
    });

}
```

We defined new section called `nixosModules.default` - it defines systemd service which will execute `my-crate` binary.
Users of this module can configure it by options that we defined for our new module:

```nix
enable = mkEnableOption "Enables the backend HTTP service";

domain = mkOption rec {
  type = types.str;
  default = "localhost";
  example = default;
  description = "The domain name";
};
```

Notice that configuration options are auto documented by using descriptions and examples where necessary. It's a very clean way of documenting your application.

## Integration testing

It's extreamly important to reduce feedback loops when working on software.
In case of our deployment I'd like to get a response to question: `is this commit deployable`.
To do so traditionally you would have to use CI/CD system - deploy to your linux box/k8s cluster and run some hopefully automated testing.
Since I'm planing on managing this system by myself I'd prefer if it's tested even before commiting the changes.


To shorten feedback cycle it's possible to use exported module in NixOs VM test.
It's possible using standart tooling to declare test which will start a number of VMs using qemu and KVM and run a test script against it.

```nix
{ system
, pkgs
, makeTest
, self
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
      networking.firewall.allowedTCPPorts = [ test_port ];
      imports = [ sharedModule self.nixosModules.${system}.default ];
      services.backend.enable = true;
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

    server.wait_for_open_port(${toString test_port})

    expected = "<h1>Hello, World!</h1>"

    actual = client.succeed(
            "${pkgs.curl}/bin/curl -vv http://server:${toString test_port}"
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
```


Above code imports default `nixosModule` from parent flake and enables `backend` service:

```nix
services.backend.enable = true;
services.nginx.enable = true;
```

Additionally it contains python `testScript` that enables us to write archestation for our machines and add assertions.
If our test fails it will stop the process of building. It's absolutely amazing.
Since we added `integration` as one of the checks it will be run every time a flake is built or updated.
It's aware of the inputs so the tests won't be run if none of the inputs has not been changed.

The tests can be additionally run as a single command using:

```bash
nix run -L .\#checks.x86_64-linux.integration
```

To debug and test when developing the tets can be also run using interactive mode:

```bash
nix run -L .\#checks.x86_64-linux.integration.driverInteractive
```

### Resources

- [Integration testing using virtual machines](https://nix.dev/tutorials/integration-testing-using-virtual-machines)
- [NixOs tests main documentation](https://nixos.org/manual/nixos/stable/index.html#sec-nixos-tests)
- [NixOs modules](https://nixos.wiki/wiki/NixOS_modules)
