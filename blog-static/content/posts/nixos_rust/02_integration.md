+++
title =  "Blazing fast module 🚀"
date = 2023-12-04T12:00:00+02:00
draft = true

series = ["Simple personal blog"]

description = """
Exposing blazing fast hello world using NixOs module.
"""
+++

## Let's expose the flake!

In a [previous](./01_start.md) post we've built a very simple but blazing-fast http server that returns a hello world html page.
it builds an elf binary that we can run. it's basically where nixos starts.

We'll have to expose this piece of amazing code as a nixos module.
the nixos module is just a function that takes certain inputs and returns additional output that is evaluated when the whole system gets built. [^1]
that sounds cool but it's quite hard to grasp without knowing how to use it in practice.


**contents:**

{{< toc >}}

## Nixos module in practice

Let's apply the following changes in flake.nix (some parts are omitted for brevity you can check the full changes by `git show 283e3bc`)

```nix
{
  # ...

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }: {
    formatter.x86_64-linux = nixpkgs.legacypackages.x86_64-linux.nixpkgs-fmt;
  } //
  #               👇 this used to be  eachdefaultsystem but now we build only on linux!
  flake-utils.lib.eachsystem [ "x86_64-linux" "aarch64-linux" ] (system:
    let
    # ...
    in
    {
      nixosmodules.default = { config, lib, ... }: with lib;
        let
          cfg = config.services.backend;
        in
        {
          options.services.backend = {
            enable = mkenableoption "enables the backend http service";

            domain = mkoption rec {
              type = types.str;
              default = "localhost";
              example = default;
              description = "the domain name";
            };
          };

          config = mkif cfg.enable {
            systemd.services.backend = {
              wantedby = [ "multi-user.target" ];
              serviceconfig = {
                restart = "on-failure";
                execstart = "${my-crate}/bin/quick-start";
                dynamicuser = "yes";
                runtimedirectory = "backend";
                runtimedirectorymode = "0755";
                statedirectory = "backend";
                statedirectorymode = "0700";
                cachedirectory = "backend";
                cachedirectorymode = "0750";
              };
            };

            services.nginx.virtualhosts.${cfg.domain} = {
              locations."/" = { proxypass = "http://127.0.0.1:3000"; };
            };
          };
        };
      checks = {
        #... 
        # 👇 this is new check that we added
        integration = import ./nixos-test.nix {
          maketest = import (nixpkgs + "/nixos/tests/make-test-python.nix");
          inherit system;
          inherit pkgs;
          inherit self;
        };

        my-crate-coverage = cranelib.cargotarpaulin (commonargs // {
          inherit cargoartifacts;
        });
      };
        #...
    });

}
```

We defined new section called `nixosmodules.default` - it defines systemd service which will execute `my-crate` binary.
users of this module can configure it by options that we defined for our new module:

```nix
enable = mkenableoption "enables the backend http service";

domain = mkoption rec {
  type = types.str;
  default = "localhost";
  example = default;
  description = "the domain name";
};
```

Notice that configuration options are auto-documented by using descriptions and examples where necessary. it's a very clean way of documenting your application.

## Testing our module

We all know how extremely important to reduce feedback loops when working on software. 
It's a nightmare when one has to run the whole ci/cd pipeline with a lengthy deploy step and then check if the configuration change is working.


### The amazing benefits of using containerization instead

The below section will be a **completely unnecessary** dunking on docker. Feel free to skip.

Let's think what analogical deployment would look in docker world?

**Step 1** Build basic Dockerfile

We can build locally, *that was easy* - we didn't **just copy the shit out of some random out-of-date github page**.

**Step 2** Build in CI/CD

Ok now let's set up a ci system - we will need to build a docker image.
Well we have [3 options](https://docs.gitlab.com/ee/ci/docker/using_docker_build.html#enable-docker-commands-in-your-cicd-jobs), **all of them have drowbacks** but *it's fine.*


**Step 3** caching

Ok, phew **that took a lot of time** but we can now run the docker build. 
wait, why is it always compiling all crates? oh, the layers are not cached.
let's search for the answer... 


shit, **it's not that easy to cache cargo build right**, [I'll have] to fake the inputs inside the build using an external tool or plain hackery for rust to distinguish that none of the dependencies have changed.
wait neat, there is experimental (hmmm it's not experimental anymore!) support for caches for certain directories - I'll just use this!
oh no local caches are not working as expected in [gitlab](https://gitlab.com/gitlab-org/cluster-integration/auto-build-image/-/merge_requests/68#should-this-support-local-caching) nor github wait because they should not be local.
let's read the documentation about [caching](https://docs.docker.com/build/cache/) and [remote caching](https://docs.docker.com/build/cache/backends/).
wow, this was simple it only **took 3 days to set up correctly(?)**.

**Step 4** Applying Docker file good practices

wait why are our **packages not up to date**? we are using the `:latest` image.
ohhh it's just old, let's update inside the dockerfile so the packages are up to date.
**Jesus, have you seen the image size**? what is taking up so much? 


ohh we are pulling all of those dev dependencies updates to our final image. 
now we know about [multistage builds](https://docs.docker.com/build/building/multi-stage/) *it's easy!*
and wait there is stuff we don't need let's copy other's developer code. they remove some files, they must know what they're doing.

*that's super obvious and simple* and btw definitely everyone is scanning their images using tools like trivy.
because they know that **using containerization doesn't make you that much safer** if you are running old software, right?

and for sure we have pinned the sha to a specific version so the build is more reproducible and someone malicious doesn't push something shady to `latest` tag!
to top it off we are using our own registries that proxy the more central ones like docker hub or github. but you know it's not like they are going to delete them, [right](https://www.reddit.com/r/programming/comments/11uhwtj/docker_is_deleting_open_source_organisations_what/)?

most definitely they are using rootless modes in either [podman](https://github.com/containers/podman/blob/main/docs/tutorials/rootless_tutorial.md) or [docker](https://docs.docker.com/engine/security/rootless/).
phef that's a relief, but anyway **it's been possible to pop the container already**, [right](https://nvd.nist.gov/vuln/detail/cve-2019-5736)?

now we have images! your process just has to configure one layer of configuration - hopefully using env variables, because it's easy to inject them both inside container and without containerization.
your docker image might have to add some additional settings to set up the process correctly or expose correct defaults using `env`.

for different local environments, the QA teams would like to use a configuration file since they run their tests against different environments.
easy! docker accepts [env files](https://docs.docker.com/compose/environment-variables/env-file/), but you'll have to remember to quote the variables [funny](https://dev.to/tvanantwerp/don-t-quote-environment-variables-in-docker-268h), simple.
that's fine, I can **just add yet another shell here, and there**.

**Step 5** integration tests in CI

now to the integration tests! your ci environment has to be set up correctly to pick up variables and set them up just a little bit differently to appraise the gods of the remote execution, i'm sure you will get the urls of different services just right.
wait, why the hostname of the service called `pg-ci` doesn't work. ahh obiously, it's `pg__ci` since we didn't specify a [service alias](https://docs.gitlab.com/ee/ci/services/#accessing-the-services) in gitlab.
that's ok we have all the time in the world to burn.

then your orchestrator steps in to inject the variables using different mechanisms, for instance as secret.
awesome there is [documentation](https://kubernetes.io/docs/concepts/configuration/secret/).

**Step 6** building k8s helm chart/deployment yaml

But wait! We are testing just the first layer. our image has to be wrapped to be deployed to kubernetes.
it will be **very time-consuming** to do right, especially to test e2e, hopefully in an automated fashion.

Notice that we are not exactly bringing any value.

### Alternative world

In the alternative reality, the whole package is already being formatted, tested, and audited for valnourabilites using `nix flake check`.
it reuses the cached dependencies built in previous steps without changing anything. we can add arbitrary steps to it, ie some cargo tests or test coverage.

We already know that the code upholds the unit tests and doc tests specified requirements.
But what about our deployment definition I'd like to get a response to question: `is this code deployable and running ok when run as on production`.

To do so traditionally as mentioned before you would have to use ci/cd system - deploy to your linux box/k8s cluster and run some test suite.
since i'm planing on managing this system by myself i'd prefer if it's tested even before commiting the changes.

### NixOs vm test

To shorten feedback cycle it's possible to use exported module in nixos vm test.
It's possible using standart tooling to declare test which will start a number of vms using qemu and kvm and run a test script against it.

```nix
{ system
, pkgs
, maketest
, self
}:
let
  test_port = 80;
  sharedmodule = {
    # since it's common for ci not to have $display available, we have to explicitly tell the tests "please don't expect any screen available"
    virtualisation.graphics = false;
  };
in
maketest
{
  name = "integration";

  nodes = {
    server = {
      networking.firewall.allowedtcpports = [ test_port ];
      imports = [ sharedmodule self.nixosmodules.${system}.default ];
      services.backend.enable = true;
      services.nginx.enable = true;
    };
    client = {
      imports = [ sharedmodule ];
    };
  };


  extrapythonpackages = p: [ p.termcolor ];

  testscript = ''
    start_all()
    import sys
    from termcolor import cprint

    server.wait_for_open_port(${tostring test_port})

    expected = "<h1>hello, world!</h1>"

    actual = client.succeed(
            "${pkgs.curl}/bin/curl -vv http://server:${tostring test_port}"
    )

    if expected != actual:
        cprint("test failed unexpected result: " + actual, "red", attrs=["bold"], file=sys.stderr)
        sys.exit(1)
  '';
}
{
  inherit pkgs;
  inherit (pkgs) system;
}
```

Above code imports default `nixosmodule` from parent flake and enables `backend` service, just like we would in final flake:

```nix
services.backend.enable = true;
services.nginx.enable = true;
```

Additionally it contains python `testscript` that enables us to write archestation for our machines and add assertions.
if our test fails it will stop the process of building. it's absolutely amazing that in metter of seconds you can deploy the code just as it will work on production.
since we added `integration` as one of the checks it will be run every time we run `nix flake check`.
it's aware of the inputs so the tests won't be run if none of the inputs has not been changed.

The tests can be additionally run as a single command using:

```bash
nix run -l .\#checks.x86_64-linux.integration
```

You can even debug and test when developing the tets can be also run using interactive mode:

```bash
nix run -l .\#checks.x86_64-linux.integration.driverinteractive
```

In the [next post](../03_deployment) i will write how to deploy current code onto real vm provisioned on Hetzner.


### resources

- [integration testing using virtual machines](https://nix.dev/tutorials/integration-testing-using-virtual-machines)
- [nixos tests main documentation](https://nixos.org/manual/nixos/stable/index.html#sec-nixos-tests)
- [nixos modules](https://nixos.wiki/wiki/nixos_modules)


- [^1] nixos modules exaplained in an awesome [blog post](https://xeiaso.net/blog/nix-flakes-3-2022-04-07)
