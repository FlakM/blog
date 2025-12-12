--- 
title: "Your Editor Can’t Do This (Unless It’s Good like Neovim)"
date: 2025-11-13T12:51:50+01:00
draft: false
---

## Tell me your editor can do that!

Copy a reference to *exactly this function* with correct types, at *exactly this commit*, paste it anywhere - markdown, slack claude code and jump back to it instantly, even days later.

{{< youtube q_NK80WzTlQ >}} 

## Why do I need this?

I write down every new task in a **[brag document](https://jvns.ca/blog/brag-documents/?ref=blog.pragmaticengineer.com)** - idea introduced to me by Julia Evans.

Every position links to the new note in a `tasks` directory. At the beginning, it's a simple link; over time, I refine the title to be more descriptive.

After I'm done with it, it's massive - for larger tasks, it might become *a directory with multiple notes* inside. The note itself contains all the knowledge I've gathered about the task - links to relevant PRs, issues, design documents, and so on.

This format used to work well - until I've started to sleep less :P 

## Why linking to text sucks

1. A file path doesn’t point to *meaning* - only a file
2. It's not bound to time - the file will change
3. Navigation is manual and slow - even with [super fast fuzzy sessions switching](/posts/fuzzy_tmux/)

## Sharing a reference to a place in the code

I've worked out a couple of shortcuts to make this easier:

| Shortcut | Mode | Description | Example |
|----------|------|-------------|---------|
| `<leader>cf` | n | Copy relative path from project root | `src/main.rs` |
| `<leader>cF` | n | Copy absolute path | `/home/user/project/src/main.rs` |
| `<leader>ct` | n | Copy filename only | `main.rs` |
| `<leader>ch` | n | Copy directory path | `/home/user/project/src` |
| `<leader>cg` | n | Copy GitHub link as markdown | `[main.rs#L42](https://github.com/...)` |
| `<leader>cg` | v | Copy GitHub link with line range | `[main.rs#L10-L20](https://github.com/...)` |
| `<leader>cm` | n | Copy markdown reference (local) | `[fn main](file:///path/to/main.rs#L42)` |
| `<leader>cgl` | n | Copy markdown with GitHub link | `[fn main](https://github.com/...)` |
| `<leader>j` | n | Jump to link from clipboard | - |
| `gj` | n | Jump to link under cursor | - |

The heavy lifting is done by a small rust cli called [jump](https://github.com/FlakM/jump). It generates stable, shareable references - and knows how to resolve them back to file paths.

```shell
jump on  main is 󰏗 v0.1.0 via  v1.91.1 via  impure (nix-shell-env) 
❯ jump copy-markdown --root . --file src/main.rs --line 11 --character 11
{
  "markdown": "[fn jump::main](file:///home/flakm/programming/flakm/jump/src/main.rs#L11)"
}
❯ jump copy-markdown --root . --file src/main.rs --line 11 --character 11 --github
{
  "markdown": "[fn jump::main](https://github.com/FlakM/jump/blob/1cb37f37c72ed98fd9c29507067417762a924d66/src/main.rs#L11)"
}
jump on  main is 󰏗 v0.1.0 via  v1.91.1 via  impure (nix-shell-env) 
❯ jump "[fn jump::main](https://github.com/FlakM/jump/blob/1cb37f37c72ed98fd9c29507067417762a924d66/src/main.rs#L11)"
{
  "status": "success",
  "session": "jump",
  "file": "/home/flakm/programming/flakm/jump/src/main.rs",
  "line": 11
}
```

## Findings

- [`neovim`](https://github.com/neovim/neovim) is absurdly extensible
- [`neovim-remote`](https://github.com/mhinz/neovim-remote) makes controlling running instances trivial
- [`lspmux`](https://codeberg.org/p2502/lspmux) lets multiple Neovim instances share LSPs
- NixOS + Home Manager made the whole setup reproducible
    - [cli](https://github.com/FlakM/jump) tool to do the jumping 
    - neovim configuration [home-manager/modules/nvim/config/init.lua#L247-L374](https://github.com/FlakM/nix_dots/blob/23f7470ae504006270be39791de2a87608b9723c/home-manager/modules/nvim/config/init.lua#L247-L374)
- [`hyprland`](https://hypr.land/) + `tmux` + `neovim` is a killer combo for productivity

## Final thoughts

I will not be touching tools that do not allow extensibility like this. 

Being a power user, to me, means shaping your tools instead of adapting to their limits.

Let me know if you have any questions or suggestions - I'm always happy to discuss tooling and workflows!

