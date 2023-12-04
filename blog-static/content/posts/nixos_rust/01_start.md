---
title: "Going for a first ride with NixOs 🚴"
date: 2023-12-01T12:00:00+02:00
draft: false

authors: ["Maciej Flak"]


series: ["Perfect simple deployment"]

toc: true

Summary: '
About picking the right size of solutions for the problem.
'
---

## Learning to ride a bicycle

Over a year ago I bought two different-sized bikes for my doughter because sizing is hard and one of them was pink...

The first was way too big for her, and the second was just right.
By accident, I've gained valuable insight into solving problems.

> **Inappropriate-sized solution will inevitably couse pain and kill motivation to learn.**


**Contents:**

{{< toc >}}


### The tech bike shop

Similarly, if you try to solve a technical problem with a solution that is too big for you - it will hurt.

The problem is that the incentives of the industry are not aligned to help us pick just the right solution.
The proposed solutions tend to solve a generic problem - for a wider group of recipients, but also bring a lot of accidental and unnecessary complexity.

> **Complexity** characterizes the behavior of a system or model whose components interact in multiple ways and follow local rules, leading to nonlinearity, randomness, collective dynamics, hierarchy, and emergence [^1]


If we start to research the topic of deploying networked systems design in 2023 we most definitely will find the following solutions:

1. Cloud
2. Docker
3. Kubernetes
4. Serverless

Those are all fine ideas and each solves a very particular problem.

The cloud is amazing at covering very bursty loads, it doesn't scale economically for very steady loads (no matter what providers say).
Having a customized, provider owned set of services also tends to lead to overly complex solutions and vendor lock-in. 
Just take a look at [reference architecture](https://docs.aws.amazon.com/whitepapers/latest/best-practices-wordpress/reference-architecture.html) for a WordPress deployment, it's not a simple strategy.

Docker was an extremely important step in the right direction, it's a developer-focused tool that abstracts over a deployment runtime.
But at the same time, it gives a false sense of security and invites a lot of nasty paper cuts.

Kubernetes has pretty much won the war for the standard orchestration solution and probably rightfully so.
But the basic - happy path experience differs greatly from when you have to understand the inner parts of the system. 
Especially when debugging in anger.
Many good things can be said about it, but it ain't simple.
If you don't believe me go ahead and read a recent [changelog for version 1.27](https://github.com/kubernetes/kubernetes/blob/master/CHANGELOG/CHANGELOG-1.27.md#changes-by-kind-1) and amazing postmortem for failed version migration at [reddit](https://www.reddit.com/r/kubernetes/comments/11xxsz1/long_detailed_post_mortem_on_a_reddit_failed_k8s/).

Serverless promises to be extremely scalable and easy to architect your services.
But it abstracts over stateless applications. To get the benefits one has to use the accompanying storage solutions.
Needless to say, it tends to introduce heavy vendor lock-in.


At the same time, according to stack overflow's survey, most of us are not working for the biggest organizations. [^2]

> Almost **50% percent of developers work for companies with less than 100 employees**.

Our companies almost certainly don't have even a small fraction of big tech budgets to burn.
Particularly now, when the economy is going to yaml.


### Twisted incentives

If you think about it the reasons behind the above technologies are trivial.
Just look up the value of AWS, Azure, or Google Cloud...

Since I'm currently working on building my personal blog to become a part of a fediverse I thought I'll write the road trip down.

I would like to propose a crazy idea - *not using containerization and orchestrators*. 
Instead do it the good, old-school way - but with the help of new([?](https://en.wikipedia.org/wiki/NixOS#History)) tools.
Here are my main objectives for the whole system:

1. **Low maintenance** and **low cost** - I'm a single person working on it without any revenue in sight.
2. **Open-source** solutions preferred where possible - others might find it more useful.
3. **Do once - do right** - I'm spending my time in the evenings. This is a precious commodity.
4. **Self-documenting** - there is a high chance that I'll have to rediscover some parts many months/years later.
5. **No vendor/technology lock-in** - I'd hate to be forced to work on something just because some company decides to change the pricing model.

### How to avoid getting bitten

I've been daily driving NixOs on my desktop for over a year now. It has been a fun ride and I feel like it's getting more and more attention.

Companies like fly.io or Heroku will gladly take your docker image and host it with all the features like observability already there.
But they inherently have another problem. Once you rely on them too much you are on the hook - it will be hard to migrate to a different provider.
Understandable, to make ends meet they also have to be costlier than alternatives.

In my eyes, there is a nice alternative - NixOs, nixpkgs, and nix. Here is my idea about what they bring to the table:

1. **Aggregation of system components** - much like k8s with multiple helm charts but in declarative way. The state is held in your repository (kind of like argocd).
2. **Meta build tool** - just like docker, nix is technology agnostic it doesn't care if it's rust, go, or java. 
3. **Self-documentation** - since there is only one entry point to your system it is by default documented. You won't forget about this little docker container that you have run on the machine or this tiny tiny package on the OS.
4. **Debuggability and testability** - NixOs configuration can be easily deployed onto a VM or tested using an automated testing framework
5. [Reproducible builds](https://reproducible-builds.org/) - if you have the same source (most of the time) you will obtain bit for bit same result no matter when you run it. You can even read more about building 15yo software [here](https://blinry.org/nix-time-travel/). While in theory possible using [docker](https://fosdem.org/2023/schedule/event/container_reproducible_dockerfile/attachments/slides/5574/export/events/attachments/container_reproducible_dockerfile/slides/5574/FOSDEM2023_Bit_for_bit_reproducible_builds_with_Dockerfile.pdf) it's definietely not a practice.
6. **No need to keep the binary data around** - this is due reproducibility of builds. The artifacts storing story turned out to be a big [docker kerfuffle](https://www.theregister.com/2023/03/17/docker_free_teams_plan/) recently.
7. **Good practices encouraged**. It doesn't leave what I like to call a shell mess. Strange one-off bash scripts casting dark spells.
   You can almost by accident create an amazing developer experience using direnv and devShells.


The system I'd like to build up - my blog as of writing those words is a statically generated hugo site that is hosted using github pages.
There are many things wrong with it, but I like it (apart from writing regularly).

My side quest is to use as few dockers and yaml lines as possible. *Because it's all fucking terrible and it should burn!*

I'll provision the environment using Hetzner and Cloudflare since I believe that it will enable most people to do the same if they wish.
Over the course of the series, it should become clear that there is no problem with moving to different provider or to self hosted box.

Let's dive in!

## Starting a new project

### Initial project setup

Since I'm a backend developer by day let's start with what I'm the most comfortable with.
Let's create a simple backend project using [github](https://github.com/new) let's create a project and clone it locally:

```bash
git clone git@github.com:FlakM/backend.git
```

I suppose that you've read the [amazing blog series](https://fasterthanli.me/series/building-a-rust-service-with-nix) from Amos that I've mentioned before so I'm not going to show how to install nix and just go with it.

```bash
cd backend
nix flake init -t github:ipetkov/crane#quick-start

# and if you have cargo on your system
# cargo add axum anyhow tokio -F tokio/full
cat <<EOF >> Cargo.toml
anyhow = "1.0.70"
axum = "0.7.1"
tokio = { version = "1.34.0", features = ["full"] }
EOF
```


And copy the [hello world example](https://github.com/tokio-rs/axum/blob/main/examples/hello-world/src/main.rs) from axum page:


```rust
use axum::{response::Html, routing::get, Router};

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new().route("/", get(handler));

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
```

### Testing the application


We can already run a working application by running `nix run`:

```bash
❯ git commit -m "chore: initial server"
# let's run the code
❯ nix run                                                                                                                      │
listening on 127.0.0.1:3000
# and from the other terminal we can test it
❯ curl localhost:3000
<h1>Hello, World!</h1>%
```

You can see the whole code by showing the commit: `git show 16b7fe5`. 

Actually, if we investigate right now our flake:

```
❯ nix flake show
git+file:///home/flakm/programming/flakm/backend?ref=refs%2fheads%2fmain&rev=16b7fe52db02456a4549960cf9d9b14a3fe31239
├───apps
│   ├───aarch64-darwin
│   │   └───default: app
│   ├───aarch64-linux
│   │   └───default: app
│   ├───x86_64-darwin
│   │   └───default: app
│   └───x86_64-linux
│       └───default: app
├───checks
│   ├───aarch64-darwin
│   │   ├───my-crate: derivation 'quick-start-0.1.0'
│   │   ├───my-crate-audit: derivation 'crate-audit-0.0.0'
│   │   ├───my-crate-clippy: derivation 'quick-start-clippy-0.1.0'
│   │   ├───my-crate-doc: derivation 'quick-start-doc-0.1.0'
│   │   ├───my-crate-fmt: derivation 'quick-start-fmt-0.1.0'
│   │   └───my-crate-nextest: derivation 'quick-start-nextest-0.1.0'
│   ├───aarch64-linux
│   │   ├───my-crate: derivation 'quick-start-0.1.0'
│   │   ├───my-crate-audit: derivation 'crate-audit-0.0.0'
│   │   ├───my-crate-clippy: derivation 'quick-start-clippy-0.1.0'
│   │   ├───my-crate-doc: derivation 'quick-start-doc-0.1.0'
│   │   ├───my-crate-fmt: derivation 'quick-start-fmt-0.1.0'
│   │   └───my-crate-nextest: derivation 'quick-start-nextest-0.1.0'
│   ├───x86_64-darwin
│   │   ├───my-crate: derivation 'quick-start-0.1.0'
│   │   ├───my-crate-audit: derivation 'crate-audit-0.0.0'
│   │   ├───my-crate-clippy: derivation 'quick-start-clippy-0.1.0'
│   │   ├───my-crate-doc: derivation 'quick-start-doc-0.1.0'
│   │   ├───my-crate-fmt: derivation 'quick-start-fmt-0.1.0'
│   │   └───my-crate-nextest: derivation 'quick-start-nextest-0.1.0'
│   └───x86_64-linux
│       ├───my-crate: derivation 'quick-start-0.1.0'
│       ├───my-crate-audit: derivation 'crate-audit-0.0.0'
│       ├───my-crate-clippy: derivation 'quick-start-clippy-0.1.0'
│       ├───my-crate-coverage: derivation 'quick-start-tarpaulin-0.1.0'
│       ├───my-crate-doc: derivation 'quick-start-doc-0.1.0'
│       ├───my-crate-fmt: derivation 'quick-start-fmt-0.1.0'
│       └───my-crate-nextest: derivation 'quick-start-nextest-0.1.0'
├───devShells
│   ├───aarch64-darwin
│   │   └───default: development environment 'nix-shell'
│   ├───aarch64-linux
│   │   └───default: development environment 'nix-shell'
│   ├───x86_64-darwin
│   │   └───default: development environment 'nix-shell'
│   └───x86_64-linux
│       └───default: development environment 'nix-shell'
└───packages
    ├───aarch64-darwin
    │   ├───default: package 'quick-start-0.1.0'
    │   └───my-crate-llvm-coverage: package 'quick-start-llvm-cov-0.1.0'
    ├───aarch64-linux
    │   ├───default: package 'quick-start-0.1.0'
    │   └───my-crate-llvm-coverage: package 'quick-start-llvm-cov-0.1.0'
    ├───x86_64-darwin
    │   ├───default: package 'quick-start-0.1.0'
    │   └───my-crate-llvm-coverage: package 'quick-start-llvm-cov-0.1.0'
    └───x86_64-linux
        ├───default: package 'quick-start-0.1.0'
        └───my-crate-llvm-coverage: package 'quick-start-llvm-cov-0.1.0'
```

We have multiple outputs! For example, we can run predefined checks by running `nix flake check` it will run our beloved `clippy`, `cargo-audit`, `cargo-doc`, and `fmt` and run tests we can also pick specific ones:

```bash
# -L specifies that we want to see more output
❯ nix build -L .\#checks.x86_64-linux.my-crate-fmt
quick-start-fmt> cargoVendorDir not set, will not automatically configure vendored sources
quick-start-fmt> cargoArtifacts not set, will not reuse any cargo artifacts
quick-start-fmt> unpacking sources
quick-start-fmt> unpacking source archive /nix/store/glj3sb8dixasayic77iq0954mz2jh0na-source
quick-start-fmt> source root is source
quick-start-fmt> patching sources
quick-start-fmt> Executing configureCargoCommonVars
quick-start-fmt> configuring
quick-start-fmt> default configurePhase, nothing to do
quick-start-fmt> building
quick-start-fmt> ++ command cargo --version
quick-start-fmt> cargo 1.67.0
quick-start-fmt> ++ command cargo fmt -- --check
quick-start-fmt> ensureTargetDir
quick-start-fmt> installing
quick-start-fmt> installing target to /nix/store/gghczbrnjnlflcs9478ndk7s9cmpn7nx-quick-start-fmt-0.1.0
quick-start-fmt> post-installation fixup
quick-start-fmt> shrinking RPATHs of ELF executables and libraries in /nix/store/gghczbrnjnlflcs9478ndk7s9cmpn7nx-quick-start-fmt-0.1.0
quick-start-fmt> checking for references to /build/ in /nix/store/gghczbrnjnlflcs9478ndk7s9cmpn7nx-quick-start-fmt-0.1.0...
quick-start-fmt> patching script interpreter paths in /nix/store/gghczbrnjnlflcs9478ndk7s9cmpn7nx-quick-start-fmt-0.1.0
```

Notice different architectures? The same command will work on a fancy m1 machine. And the results are stored using a content based hash.

```bash
❯ ll result
lrwxrwxrwx 1 flakm users 65 Apr  6 19:00 result -> /nix/store/gghczbrnjnlflcs9478ndk7s9cmpn7nx-quick-start-fmt-0.1.0
```

Once computed nix is able to avoid doing recomputation because it knows none of the inputs have changed.
We will use this property later to ship the packages to our target machine!


So we are now able to ship the binary. If we run `nix build` the result directory will contain ELF binary.

```bash
❯ file result/bin/quick-start
result/bin/quick-start: ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), dynamically linked, interpreter /nix/store/8xk4yl1r3n6kbyn05qhan7nbag7npymx-glibc-2.35-224/lib/ld-linux-x86-64.so.2, for GNU/Linux 3.10.0, not stripped
```

In the [next post](../02_integration) I will cover exposing this code as a NixOs module that wraps our binary with the systemd service.
I'll also showcase a very powerful feature of NixOs - integration tests using VMs using Quemu as the backend! 


If you want to see more of similar write-ups you might follow me on mastodon [@flakm](https://hachyderm.io/@flakm).


[^1]: Complexity definition on [wikipedia](https://en.wikipedia.org/wiki/Complexity)
[^2]: 2022 SO [survey results for work company info](https://survey.stackoverflow.co/2022/#work-company-info)
