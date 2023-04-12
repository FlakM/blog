---
title: "Setting up backend with nix"
date: 2023-04-06T17:13:18+02:00
draft: true

series: ["The perfect deployment"]

Summary: '
Deploying usefull stuff - a minimalist way
'

---


# The perfect deployment

Writing software is hard, writing good software and deploying it is even harder.


At the end of this series I'd like to have this very blog with small backend component built with simplicity in mind.
Here is my checklist for the solution:

1. Low maintainance
2. Open source solutions prefered
3. Do once - do right
4. Self documenting

I've been flirting with nixos on my desktop machine for over a year now and recently got to ship some production code using nix.
I've been inspired by [recent fasterthanlime's series](https://fasterthanli.me/series/building-a-rust-service-with-nix) to write down my experience.


This very site is a statically generated with hugo that is hosted on using github pages.
There are many things wrong about it 

1. It is hosted on github - if they change anything I'll need to change my stuff
2. The url is pointing to github domain
3. It does not have any dynamic content
4. It's using google analytics


Over a number of posts I'd like to document my road from using hosted static site to building up a more usefull system composing of many services.
My side quest is to use as few yaml line as possible. Because it's fucking terrible!


## Starting a new project

Since I'm a backend developer by day let's start with what I'm the most comfortable.
Let's create a simple backend project using github since I'm lazy: https://github.com/new lets create a project and clone it locally:

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
Actually if we investigate right now our flake:

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

We have multiple targets. For example we can run predefined checks by running `nix flake check` it will run clippy, cargo-audit, cargo-doc, fmt and run tests we can also pick specific one:

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

Notice different architectures? The same command will work on fancy m1 machine. And the results are stored using hash.

```bash
❯ ll result
lrwxrwxrwx 1 flakm users 65 Apr  6 19:00 result -> /nix/store/gghczbrnjnlflcs9478ndk7s9cmpn7nx-quick-start-fmt-0.1.0
```

Once computed nix is able to avoid doing recomputation because it knows none of the inputs have changed.
We will be also using this particular feature to do binary caches later on during deployment.
So we are now able to ship the binary. If we run `nix build` the result directory will contain ELF binary.

```bash
❯ file result/bin/quick-start
result/bin/quick-start: ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), dynamically linked, interpreter /nix/store/8xk4yl1r3n6kbyn05qhan7nbag7npymx-glibc-2.35-224/lib/ld-linux-x86-64.so.2, for GNU/Linux 3.10.0, not stripped
```

Let's expose this piece of amezing code as nix module. You can read more about it [here](https://xeiaso.net/blog/nix-flakes-3-2022-04-07) but NixOs module is just a function that takes certain input and returns additional output that is evaluated
when the whole system gets built. 
