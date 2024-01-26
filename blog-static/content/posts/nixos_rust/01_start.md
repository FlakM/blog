+++ 
title = "Measure twice cut once with NixOs"
date = 2023-11-10T12:00:00+02:00
draft = false
series = ["Simple personal blog"]
toc = true

description = """
Discover the benefits of using NixOS for building a personal, federated blog in my latest series. Follow along as we explore its simplicity and efficiency.
"""

tags = ["rust", "nixos", "hugo", "blog"]
mastodonName = "@flakm@hachyderm.io"
+++ 

## Building a personal tech blog

Building a personal blog can be daunting, especially when choosing the right solution that will get out of the way.
Therefore, I'd like to have a static site generator split the site and add ActivityPub integration to interact with it over time.

I've also decided on self-hosting and using open protocols, especially after last year showed that building on top of free stuff is only sustainable in the short run. Not when the money stops being free.

This series will document this journey.

**Contents:**

{{< toc >}}


### Navigating technology, size does matter

Consider the challenge of tackling a tech problem with a solution that's too 'big'. It quickly becomes unwieldy and awkward.

The industry often doesn't align its incentives with helping us choose **the right solution™**.
Products typically address broad issues, appealing to a wide audience but introducing unnecessary complexity.

We call the problems that we create and can solve **accidental complexity**.
In contrast, **essential complexity** is inherent to the problem itself.
It cannot be removed - it just is. [^1]

In 2023, terms like cloud, docker, kubernetes, and serverless are ubiquitous in the dev/ops world, 
each offering a different set of strengths:

- *The cloud* excels in handling bursty loads but doesn't scale economically for steady loads.
A custom set of services the vendor provides quickly leads to overcomplicated solutions
Just look at [the reference architecture](https://docs.aws.amazon.com/whitepapers/latest/best-practices-wordpress/reference-architecture.html) for a WordPress deployment. It's a ridiculously complex strategy. 
And I'm for one sure that after implementation, it will never leave AWS.

- *Docker* is a fantastic developer-focused tool that abstracts over a deployment runtime.
However, it can also give a false sense of security and lead to unexpected issues.

- According to the Internet, *kubernetes* won the war for the standard orchestration solution, probably rightfully so.
But the basic - happy path experience differs significantly from when you have to understand the inner parts of the system. 
Many good things can be said about it, but it can be complicated.
If you don't believe me, read a recent [changelog for version 1.27](https://github.com/kubernetes/kubernetes/blob/master/CHANGELOG/CHANGELOG-1.27.md#changes-by-kind-1) and a fantastic postmortem for failed version migration at [reddit](https://www.reddit.com/r/kubernetes/comments/11xxsz1/long_detailed_post_mortem_on_a_reddit_failed_k8s/).

- *Serverless* promises to be higlhly scalable and easy to architect your services.
To get the full benefits of storage one has to use the accompanying data solutions (hosted services etc).
Needless to say, it tends to introduce heavy vendor lock-in.

None of these solutions is ideal for small-scale deployments, as they introduce more accidental complexity than resolve.

### My host, my rules

Since I'm working on rebuilding my personal blog to become a part of a fediverse, I decided to write down the road trip.

Here is my crazy plan - skip containerization and orchestrators. 
Instead, I'll lean on the tried-and-true methods spiced up with modern([?](https://en.wikipedia.org/wiki/NixOS#History)) tools like NixOS.
My goals for the project:

1. **Low maintenance & cost** - it's just me here.
2. **Open-source** - it's just better.
3. **Efficiency** - I'm spending my time in the evenings. I aim to do things just once.
4. **Self-documenting** - I might need to rediscover certain parts after a while, so making them self-explanatory is crucial.
5. **No vendor/technology lock-in** - I want to avoid being boxed in significantly by any vendor or technology if they change their terms or pricing.

### The Why behind the NixOs

I've been using NixOS on my desktop daily for over a year, and it's been quite the experience. It is gaining traction everywhere.

So, what do NixOs (the declarative operating system), nix (the packaging system) offer? Here's my take:

1. **Aggregation of system components** - like k8s with multiple helm charts but in a declarative way. The state is held in your repository (just like in Argocd).
2. **Meta build tool** - like docker, nix is technology agnostic; it doesn't care if it's rust, go, or java. 
3. **Self-documentation** - since there is only one entry point to your system, it is by default documented. 
You won't forget about this little docker container you have run on the machine or this tiny package on the OS.
4. **Debuggability and testability** - NixOs configuration can be easily deployed onto a VM or tested using an automated testing framework
5. [Reproducible builds](https://reproducible-builds.org/) - if you have the same source (most of the time), you will obtain bit for bit the same result no matter when you run it. You can even read more about building 15yo software [here](https://blinry.org/nix-time-travel/). 
While it's possible to use [docker](https://fosdem.org/2023/schedule/event/container_reproducible_dockerfile/attachments/slides/5574/export/events/attachments/container_reproducible_dockerfile/slides/5574/FOSDEM2023_Bit_for_bit_reproducible_builds_with_Dockerfile.pdf) in theory, it's definitely not a practice.
6. **No need to keep the binary data around** - this is due to the reproducibility of builds. The artifacts storing story turned out to be a big [docker kerfuffle](https://www.theregister.com/2023/03/17/docker_free_teams_plan/) recently.
7. **Sound practices are encouraged**. It doesn't leave a shell mess. Strange one-off bash scripts casting dark spells.


The system I'd like to build up - my blog as of writing those words is a statically generated Hugo site hosted using GitHub pages.
There are many things wrong with it, but I like it (apart from writing regularly).

My side quest is to use as few dockers and yaml lines as possible. *Because it's all fucking terrible, and it should burn!*

I'll set up the environment with Hetzner and Cloudflare, making it easy for others to replicate.
This series shows how simple it is to switch providers or move to a self-hosted setup.

Let's get started!

## Starting a new project

### Initial project setup

Since I'm a backend developer by day, let's start with what I'm the most comfortable with.
Let's create a subdirectory in the repository holding the initial code.

```bash
mkdir backend
```

I hope you've read the [amazing blog series](https://fasterthanli.me/series/building-a-rust-service-with-nix) from Amos by now, so I won't show how to install nix and just go with it.

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

```shell
❯ git commit -m "chore: initial server"
# let's run the code
❯ nix run                                                                                                                      │
listening on 127.0.0.1:3000
# and from the other terminal, we can test it
❯ curl localhost:3000
<h1>Hello, World!</h1>%
```

Actually, if we investigate right now our flake:

```shell
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

We have multiple outputs! For example, we can run predefined checks by running `nix flake check`, which will run our beloved `clippy`, `cargo-audit`, `cargo-doc`, and `fmt` and run tests. We can also pick specific ones:

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

The results are stored using a content-based hash.

```bash
❯ ll result
lrwxrwxrwx 1 flakm users 65 Dec  5 21:54 result -> /nix/store/c3hvg3l8k6a54h3g1bdw4xrmc3g09md9-quick-start-fmt-0.1.0
```

Once computed, nix can avoid recomputation because it knows none of the inputs have changed.
We will use this property later to ship the packages to our target machine!


So, we are now able to ship the binary. If we run `nix build`, the result directory will contain ELF binary.

```bash
❯ file result/bin/quick-start
result/bin/quick-start: ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), dynamically linked, interpreter /nix/store/8xk4yl1r3n6kbyn05qhan7nbag7npymx-glibc-2.35-224/lib/ld-linux-x86-64.so.2, for GNU/Linux 3.10.0, not stripped
```

In the [next post](../02_integration), I will expose this code as a NixOs module that wraps our binary with the systemd service.
Plus, I'm excited to highlight a powerful feature of NixOS: integration tests using VMs, with QEMU as the backend!"

If you want to see more similar write-ups, you might follow me on mastodon [@flakm](https://hachyderm.io/@flakm).


[^1]: http://worrydream.com/refs/Brooks-NoSilverBullet.pdf
[^2]: 2022 SO [survey results for work company info](https://survey.stackoverflow.co/2022/#work-company-info)
