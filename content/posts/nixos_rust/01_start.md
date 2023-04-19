---
title: "Stop breeding cobras 🐍"
date: 2023-04-06T17:13:18+02:00
draft: true


series: ["Breeding cobras"]
toc: true

Summary: '
How not to breed cobras and reduce complexity
'

---

## Breeding cobras the good, the bad

According to anectode, during British colonial rule in India, the government was worried about the number of venomous cobras in Delhi. 
To address this concern, they offered a bounty for every dead cobra. 
Initially, this was a successful strategy; large numbers of snakes were killed for the reward.  

Eventually, however, startup people began to breed cobras for profit.
When the government found out, the program was ended.
When cobra breeders set their snakes free, the cobra population only increased... [^1]

**Table of contents:**

{{< toc >}}


### Modern cobra breeding

Similarly, to meet raging demand for tech innovation smart individuals and large organizations came up with new ideas for abstracting over deployment using contenerization.
But it wasn't enough! It's just a fancy process with extra steps. 


Since another layer of abstraction tends to solve the problem at hand - they created orchestrators.


The solutions got more and more features too:
operators, service discovery, canary deployments, rolling updates, load balancers, hooks, and templating language to produce even more y̶͕̤̰̜̺̣̫̬͇͑̊̈́ą̶̦̤͔͗̕m̶̧͍̼͓̤͈̣̈ͅl̶̛̖̲͚͋͒̽̔́͘.
You can read recent [changelog](https://github.com/kubernetes/kubernetes/blob/master/CHANGELOG/CHANGELOG-1.27.md#changes-by-kind-1) for kubernetes.
It's completely of if you have a bunch of people on top of the game doing just that.


But most of us are not working for the biggest organizations. 
According to stack overflow's survey: [^2]

> almost **50% percent of developers work for companies with less than 100 employes!**.

Those companies almost certainly don't have even the fraction of big tech budgets to burn. Especcialy now, when the economy is going to yaml.


We truly need to see the unnecesary complexity as a venomous cobra; anyone giving out "easy" solutions as the goverment, and the promise of reducing complexity as the bounty.

### Twisted incentives

But here is the nasty twist! Large organizations didn't stop encouraging breeding the cobras. 

They started to see their new ability to host stuff as a valuable market.
For instance by coutious estimations AWS has an enterprise value of $1.2-trillion!

Folks, I'm afraid that **we are actively breeding the cobras and we gladly pay for it** 🐍 


Since I'm currently working on building my personal blog to contain more dynamic content I thought I'll write it down.
Here are my main objectives for the whole system:

1. **Low maintenance and cost** - I'm a single person working on it without any revenue in sight.
2. **Open-source** solutions prefered where possible - others might find it more usefull.
3. **Do once - do right** - I'm spending my time in the evenings this is precious comodity.
4. **Self-documenting** - there is a high chance that I'll have to rediscover some parts many months later.
5. **No vendor/technology lock-in**.

### How to avoid getting bitten

I've been daily driving NixOs on my desktop for over a year now. It has proved to pass all of the above objectives with great success.
Additionally, recent fasterthanlime's [series](https://fasterthanli.me/series/building-a-rust-service-with-nix) encouraged me to write down my experience - this seems important.

For me NixOs, nixpkgs, and nix are valuable for following reasons:

1. **Aggregation of system components** - much like k8s with multiple helm charts but in declarative way
2. **Meta build tool**. It's far easier to use native build tools' mechanisms for caching then in docker since there is no notion of overlay file system.
3. **Self documentation** of the whole system
4. **Debuggability and testability** - NixOs configuration can be easlily deployed onto VM or tested using automated testing framework
5. [Reproducible builds](https://reproducible-builds.org/) If you have same source you will obtain bit for bit same result no matter when you run it. You can event read more about building 15yo software [here](https://blinry.org/nix-time-travel/). While in theory possible using [docker](https://fosdem.org/2023/schedule/event/container_reproducible_dockerfile/attachments/slides/5574/export/events/attachments/container_reproducible_dockerfile/slides/5574/FOSDEM2023_Bit_for_bit_reproducible_builds_with_Dockerfile.pdf) it's definietly not a practice.
6. Since builds are reproducible there is **no need to keep the binary data around**, which turned out to be a big [kerfuffle](https://www.theregister.com/2023/03/17/docker_free_teams_plan/) recently.
7. **Good practices encouraged**. It doesn't leave what I like to call a shell mess. Strange one-off bash scripts doing things that nobody remmembers doing.
   You can expose pinned versions of libraries that you want your users to use using direnv integration.


The system I'd like to build up - my blog as of writing those words is a statically generated hugo site that is hosted using github pages.
There are many things wrong with it. Over several posts, I'd like to document my road to building up a more useful, dynamic, performant, and secure system.
My side quest is to use as few dockers and yaml lines as possible. Because it's all fucking terrible!

*Disclaimer:* I'll provision the environment using AWS and Cloudflare since I believe that it will enable the most people to do the same if they wish.
Over the course of the series I'd like to move away from them to self hosted machine to showcase where NixOs really shines
(spoiler alert: It's NixOs with ZFS on crappy bare metal).

Let's dive in!

## Starting a new project

### Initial project setup

Since I'm a backend developer by day let's start with what I'm the most comfortable with.
Let's create a simple backend project using [github](https://github.com/new) let's create a project and clone it locally:

```bash
git clone git@github.com:FlakM/backend.git
```

I suppose that you've read the amazing blog series from Amos that I've mentioned before so I'm not going to show how to install nix and just go with it.

```bash
cd backend
nix flake init -t github:ipetkov/crane#quick-start

# and if you have cargo on your system
# cargo add axum anyhow tokio -F tokio/full
cat <<EOF >> Cargo.toml
anyhow = "1.0.70"
axum = "0.6.12"
tokio = { version = "1.27.0", features = ["full"] }
EOF
```


And copy the [hello world example](https://github.com/tokio-rs/axum/blob/main/examples/hello-world/src/main.rs) from axum page:


```rust
use axum::{response::Html, routing::get, Router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new().route("/", get(handler));

    // run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
```

### Testing the application


We can already run a working application by running `nix run`:

```bash
❯ git commit -m "chore: initial server"
# lets run the code
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

We have multiple outputs! For example, we can run predefined checks by running `nix flake check` it will run our beloved `clippy`, `cargo-audit`, `cargo-doc`, `fmt` and run tests we can also pick specific ones:

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

In the [next post](../02_integration) I will cover exposing this code as NixOs module that wraps our binary with systemd service.
I'll also showcase very powerfull feature of NixOs - integration tests using VMs using Quemu as backend! 


If you want to see more of similar write-ups you might follow me on mastodon [@flakm](https://hachyderm.io/@flakm).




[^1]: The story is an anectode, you can read more about it on [wikipedia](https://en.wikipedia.org/wiki/Perverse_incentive)
[^2]: 2022 SO [survey results for work company info](https://survey.stackoverflow.co/2022/#work-company-info)
