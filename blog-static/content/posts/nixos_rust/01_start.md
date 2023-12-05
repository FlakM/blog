---
title: "A first ride with NixOs 🚴"
date: 2023-12-01T12:00:00+02:00
draft: false

authors: ["Maciej Flak"]
series: ["Simple personal blog"]


toc: true

Summary: '
About picking the right size of solutions for the problem.
'
---

## Learning to ride a bicycle

Over a year ago I bought a bike for my dougther. Well actually, I've bought two different-sized bikes because sizing is hard and one of them was pink...

It soon has become clear that the first was waay too big for her, and the second was just right.


**Contents:**

{{< toc >}}


### The modern tech bike shop

Similarly, if you try to solve a technical problem with a solution that is too big for you - it will hurt.

Sadly the incentives of the industry are not aligned to help us pick just **the right solution™**.

The proposed products tend to solve a more generic problem. They serve a wider group of recipients, but also smack you in a face with a lot of accidental complexity.

**Accidental complexity** relates to problems that we created and that can be fixed or avoided.
The other part is **essential complexity** which relates the problem itself. It cannot be removed - it just is. [^1]

If we start to research the topic of deploying networked systems in 2023 we most definitely will find the following terms: cloud, docker, kubernetes, serverless.

While those are all fine ideas and each solves a very particular problem.

*The cloud* is amazing at covering very bursty loads, it doesn't scale economically for very steady loads (no matter what providers say).
Just take a look at [reference architecture](https://docs.aws.amazon.com/whitepapers/latest/best-practices-wordpress/reference-architecture.html) for a WordPress deployment, it's not a simple strategy.
And I'm for one sure that after implementing it will never leave AWS.

*Docker* seems like an extremely important step in the right direction, it's a developer-focused tool that abstracts over a deployment runtime.
But at the same time, it gives a false sense of security and invites a lot of nasty paper cuts.

According to internet *kubernetes* won the war for the standard orchestration solution and probably rightfully so.
But the basic - happy path experience differs greatly from when you have to understand the inner parts of the system. 
Especially when debugging in anger.
Many good things can be said about it, but it ain't simple.
If you don't believe me go ahead and read a recent [changelog for version 1.27](https://github.com/kubernetes/kubernetes/blob/master/CHANGELOG/CHANGELOG-1.27.md#changes-by-kind-1) and amazing postmortem for failed version migration at [reddit](https://www.reddit.com/r/kubernetes/comments/11xxsz1/long_detailed_post_mortem_on_a_reddit_failed_k8s/).

*Serverless* promises to be extremely scalable and easy to architect your services.
But it abstracts over stateless applications. To get the benefits one has to use the accompanying storage solutions.
Needless to say, it tends to introduce heavy vendor lock-in.


At the same time, according to stack overflow's survey, most of us are not working for the biggest organizations. [^2]

> Almost **50% percent of developers work for companies with less than 100 employees (not only tech)**.

Our companies almost certainly don't have even a small fraction of big tech budgets to burn.
Particularly now, when the economy is going to yaml.


### My incentives

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

I've been daily driving NixOs on my desktop for over a year now. It has been a fun ride and I feel like it's getting more and more attention everywhere.

Here is my idea about what NixOs (the declarative operating system), nix (the packaging system) bring to the table:

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
Lets create a subdirectory in the repository holding the static hugo blog.

```bash
mkdir backend
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

Actually, if we investigate right now our flake:

```
❯ nix flake show

warning: Git tree '/home/flakm/programming/flakm/blog' is dirty
git+file:///home/flakm/programming/flakm/blog?dir=backend
├───checks
│   ├───aarch64-linux
│   │   ├───audit: derivation 'crate-audit-0.0.0'
│   │   ├───clippy: derivation 'quick-start-clippy-0.1.0'
│   │   ├───fmt: derivation 'quick-start-fmt-0.1.0'
│   │   ├───integration: derivation 'integration'
│   │   ├───nextest: derivation 'quick-start-nextest-0.1.0'
│   │   └───server: derivation 'quick-start-0.1.0'
│   └───x86_64-linux
│       ├───audit: derivation 'crate-audit-0.0.0'
│       ├───clippy: derivation 'quick-start-clippy-0.1.0'
│       ├───fmt: derivation 'quick-start-fmt-0.1.0'
│       ├───integration: derivation 'integration'
│       ├───nextest: derivation 'quick-start-nextest-0.1.0'
│       └───server: derivation 'quick-start-0.1.0'
├───nixosModules
│   ├───aarch64-linux: NixOS module
│   └───x86_64-linux: NixOS module
└───packages
    ├───aarch64-linux
    │   └───default: package 'quick-start-0.1.0'
    └───x86_64-linux
        └───default: package 'quick-start-0.1.0'
```

We have multiple outputs! For example, we can run predefined checks by running `nix flake check` it will run our beloved `clippy`, `cargo-audit`, `cargo-doc`, and `fmt` and run tests we can also pick specific ones:

```bash
# -L specifies that we want to see more output
❯ nix build -L .\#checks.x86_64-linux.fmt
warning: Git tree '/home/flakm/programming/flakm/blog' is dirty
quick-start-fmt> cargoVendorDir not set, will not automatically configure vendored sources
quick-start-fmt> cargoArtifacts not set, will not reuse any cargo artifacts
quick-start-fmt> unpacking sources
quick-start-fmt> unpacking source archive /nix/store/wsibjlgaw1w03fzy8xq86fcmsam23pa0-source
quick-start-fmt> source root is source
quick-start-fmt> patching sources
quick-start-fmt> Executing configureCargoCommonVars
quick-start-fmt> updateAutotoolsGnuConfigScriptsPhase
quick-start-fmt> configuring
quick-start-fmt> default configurePhase, nothing to do
quick-start-fmt> building
quick-start-fmt> ++ command cargo --version
quick-start-fmt> cargo 1.73.0
quick-start-fmt> ++ command cargo fmt -- --check
quick-start-fmt> ensureTargetDir
quick-start-fmt> installing
quick-start-fmt> no previous artifacts found, compressing and installing full archive of target to /nix/store/c3hvg3l8k6a54h3g1bdw4xrmc3g09md9-quick-start-fmt-0.1.0/target.tar.zst
quick-start-fmt> /*stdin*\            :  0.77%   (  10.0 KiB =>     79 B, /nix/store/c3hvg3l8k6a54h3g1bdw4xrmc3g09md9-quick-start-fmt-0.1.0/target.tar.zst)
quick-start-fmt> post-installation fixup
quick-start-fmt> shrinking RPATHs of ELF executables and libraries in /nix/store/c3hvg3l8k6a54h3g1bdw4xrmc3g09md9-quick-start-fmt-0.1.0
quick-start-fmt> checking for references to /build/ in /nix/store/c3hvg3l8k6a54h3g1bdw4xrmc3g09md9-quick-start-fmt-0.1.0...
quick-start-fmt> patching script interpreter paths in /nix/store/c3hvg3l8k6a54h3g1bdw4xrmc3g09md9-quick-start-fmt-0.1.0
```

The results are stored using a content based hash.

```bash
❯ ll result
lrwxrwxrwx 1 flakm users 65 Dec  5 21:54 result -> /nix/store/c3hvg3l8k6a54h3g1bdw4xrmc3g09md9-quick-start-fmt-0.1.0
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


[^1]: http://worrydream.com/refs/Brooks-NoSilverBullet.pdf
[^2]: 2022 SO [survey results for work company info](https://survey.stackoverflow.co/2022/#work-company-info)
