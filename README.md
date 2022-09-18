# About

Installation

```bash
git submodule update --init --recursive
nix-shell -p '(import <nixos-unstable> {}).hugo'
```

Running server

```bash
hugo server -D
```

Creating new post:

```bash
hugo new posts/my-first-post.md
```

Updating theme:

Blog is using [ananke theme](https://github.com/theNewDynamic/gohugo-theme-ananke)


## CI/CD 

Each CI job deploys code to flakm-test repository available under: https://flakm.github.io/flakm-test/

Master branch is deployed to https://flakm.github.io/
