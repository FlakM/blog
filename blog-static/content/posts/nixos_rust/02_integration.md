+++
title =  "Blazing fast module 🚀"
date = 2023-12-04T12:00:00+02:00
draft = false

series = ["Simple personal blog"]

description = """
Expose the blazing fast code using NixOs module and write a fancy integration test.
"""
+++

## Let's expose the flake!

In a [previous](./01_start.md) post, we built a simplistic but blazing-fast HTTP server that returns a hello world HTML page.
It produces an elf binary that we can run. It's where nixos starts.

We'll have to expose this piece of code as a nixos module.
A nixos module is just a function that takes inputs and returns additional output evaluated when the whole system is built [^1]
That sounds cool, but it's pretty challenging to grasp without knowing how to use it in practice.


**Contents:**

{{< toc >}}

## Nixos module in practice

Let's apply the following changes in `flake.nix` (I skipped over some parts  for brevity):

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

We defined a new section called `nixosmodules.default`, establishing a system service to execute the `my-crate` binary. 
Users of this module can configure the options that we specified for our new module:

```nix
enable = mkEnableOption "enables the backend http service";

domain = mkOption rec {
  type = types.str;
  default = "localhost";
  example = default;
  description = "the domain name";
};
```

Notice that configuration options are auto-documented by using descriptions and examples where necessary. it's a very clean way of documenting your application.

## Testing our module

We all know how essential it is to reduce feedback loops when working on software. 
It's a nightmare to run the whole ci/cd pipeline with a lengthy deploy step and then check if the configuration change works.


### The amazing benefits of using containerization instead

What would analogical deployment look like in the docker world?

**Step 1** Build a basic dockerfile

We can build locally. *That was easy* - we didn't **just copy the shit out of some random out-of-date github page**.

**Step 2** Build in CI/CD

Ok, now let's set up a CI system - we must build a docker image.
Well, we have [3 options](https://docs.gitlab.com/ee/ci/docker/using_docker_build.html#enable-docker-commands-in-your-cicd-jobs), **and all of them have drawbacks**, but *it's okay.*


**Step 3** caching

Ok, phew, **that took a lot of time**, but we can now run the docker build. 
Wait, why is it consistently compiling all crates? Oh, the layers are not cached.
Let's search for the answer... 


Shit, **it's not that easy to cache cargo build right**, 
I'll have to fake the inputs inside the build using an external tool or plain hackery for rust to distinguish that none of the dependencies have changed. 
Wait, neat, and there is experimental (hmmm, it's not experimental anymore!) support for caches for specific directories - I'll use this! Oh no! Local caches are not working as expected in [gitlab](https://gitlab.com/gitlab-org/cluster-integration/auto-build-image/-/merge_requests/68#should-this-support-local-caching) nor github wait because they should not be local.
Let's read the documentation about [caching](https://docs.docker.com/build/cache/) and [remote caching](https://docs.docker.com/build/cache/backends/).
Wow, this was simple. It only **took days to set up correctly(?)**.

**Step 4** Applying docker file good practices

Wait, why are our **packages not up to date**? We are using the `:latest` image.
Oh, it's just old; let's update inside the docker file so the packages are up to date
**Jesus, have you seen the image size**? What is taking up so much? 


Oh, we are pulling all of those dev dependencies updates to our final image. 
Now we know about [multistage builds](https://docs.docker.com/build/building/multi-stage/), and *it's easy!*
And wait there is stuff we don't need. Let's copy other's developer code. They remove some files, and they must know what they're doing.


*That's super obvious and simple*, and by the way, everyone is definitely scanning their images using tools like trivy.
Because they know that using containerization only makes you a little safer if you run old software, right?


And for sure, we have pinned the sha to a specific version so the build is more reproducible and someone malicious doesn't push something shady to the latest tag!
To top it off, we use our registries that proxy the more central ones like docker hub or GitHub. 
But you know it's not like they will delete them, right, [right?](https://www.reddit.com/r/programming/comments/11uhwtj/docker_is_deleting_open_source_organisations_what/)

Most definitely, they are using rootless modes in either [podman](https://github.com/containers/podman/blob/main/docs/tutorials/rootless_tutorial.md) or [docker](https://docs.docker.com/engine/security/rootless/).
Phef, that's a relief, but **it hasn't been possible to pop the container already**, [right](https://nvd.nist.gov/vuln/detail/cve-2019-5736)?

Now we have images! Your process has to configure one layer of configuration - hopefully using env variables because injecting them both inside the container and without containerization is easy.
Your docker image might have to add some additional settings to set up the process correctly or expose correct defaults using env.

for different local environments, the QA teams would like to use a configuration file since they run their tests against different environments.
easy! docker accepts [env files](https://docs.docker.com/compose/environment-variables/env-file/), but you'll have to remember to quote the variables , simple.


The QA teams would like to use a configuration file for different local environments since they run their tests against different environments.
Easy! Docker accepts [env files](https://docs.docker.com/compose/environment-variables/env-file/), 
but you'll have to remember to quote the variables in a [funny](https://dev.to/tvanantwerp/don-t-quote-environment-variables-in-docker-268h) way.
That's fine, I can **just add yet another shell here and there**.

**Step 5** Integration tests in CI

Now, to the integration tests! Your CI environment has to be set up correctly to pick up variables and set them up differently to appraise the gods of the remote execution;
I'm sure you will get the URLs of different services just right.
Wait, why doesn't the hostname of the service called `pg-ci` work. 
Ahh, it's `pg__ci` since we didn't specify a [service alias](https://docs.gitlab.com/ee/ci/services/#accessing-the-services) in gitlab.
That's okay; we have all the time in the world to burn.

Then, your orchestrator steps in to inject the variables using different mechanisms, for instance as secret.
Excellent, there is [documentation](https://kubernetes.io/docs/concepts/configuration/secret/).

**Step 6** building k8s helm chart/deployment yaml

But wait! We are testing just the first layer. We have to wrap the image with deployment and only then ship it to kubernetes.

It will be **time-consuming** to do right, mainly to test e2e, hopefully in an automated fashion.

Notice that we are not precisely bringing any value.

### Alternative world

In the alternative reality, the package is already formatted, tested, and audited for vulnerabilities using the `nix flake check`.
It reuses the cached dependencies built in previous steps without changing anything. We can add arbitrary steps, i.e., cargo tests or test coverage..

We know that the code upholds the unit and doc tests specified requirements. 
But what about our deployment definition? I'd like to get a response to the question: `is this code deployable and running ok when run as on production?`

To do so, traditionally, as mentioned before, you would have to use the ci/cd system - deploy to your Linux box/k8s cluster and run some test suite.
Since I plan on managing this system by myself, I'd prefer it to be tested even before committing the changes.

### NixOs VM test

I can shorten the feedback cycle by using the exported module in the Nixos VM test.
It's possible to use standard tooling to declare a test, which will start several VMs using qemu and KVM and run a test script against it.

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

The above code imports the default `nixosmodule` from parent flake and enables `backend` service, just like we would in final flake:


```nix
services.backend.enable = true;
services.nginx.enable = true;
```


Additionally, it contains a Python `testscript` that enables us to write orchestration for our machines and add assertions.
If our test fails, it will stop the building process. It's impressive that you can deploy the code in seconds, just as it will work on production.


Since we added `integration` as one of the checks, it will be run every time we run the `nix flake check`.
It's aware of the inputs, so the tests will only be run if all of the inputs have been changed.

The tests can be additionally run as a single command using:

```bash
nix run -l .\#checks.x86_64-linux.integration
```

You can even debug and test when developing the tests, which can be also run using interactive mode:

```bash
nix run -l .\#checks.x86_64-linux.integration.driverinteractive
```

In the [next post](../03_deployment), I will write how to deploy the current code onto a real VM provisioned on Hetzner.


### Resources

- [integration testing using virtual machines](https://nix.dev/tutorials/integration-testing-using-virtual-machines)
- [nixos tests main documentation](https://nixos.org/manual/nixos/stable/index.html#sec-nixos-tests)
- [nixos modules](https://nixos.wiki/wiki/nixos_modules)


- [^1] nixos modules explained in an awesome [blog post](https://xeiaso.net/blog/nix-flakes-3-2022-04-07)
