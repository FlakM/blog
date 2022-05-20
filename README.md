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

