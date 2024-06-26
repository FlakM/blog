+++ 
draft = false
date = 2024-03-28T08:58:48+01:00
title = "Finding stuff in the terminal"
description = """
How to find stuff in the terminal like files, patterns in files, git and shell history and more.
"""
tags = ["rust", "linux", "terminal"]
categories = []
mastodonName = "@flakm@hachyderm.io"
toc = true

images = [
    "/images/finding_stuff/feature.png"
    ]

featured_image =  "/images/finding_stuff/feature.png"
+++

Looking for stuff in code is an essential skill in daily tasks.
It lets me keep a laser focus on the problem at hand.
Examples of questions I find myself asking over and over again:

- *Does this repository contain this variable?* 
- *What was the context of changing this variable name?*
- *Who is the best person to ask questions about this project?*
- *What are all the versions of libc on my system?*

Here are my tips that make this process far more enjoyable.

**Contents:**

{{< toc >}}

## Using `grep` to locate strings

`grep` was released over 50 years ago and has become ubiquitous for text searching.
Even though there might be a more modern alternative, it's still an invaluable tool since most linux distributions ship it by default.
I find myself reaching for it when I have a file or directory and a pattern to look for or a command with long output like curl with `-i` flag, but I’m only interested in a small subset of specific headers.

`grep` will simply print the lines that match the given pattern. For instance, imagine we want to find references to `atuin` in the dotfiles directory.

```bash
# look for word atuin in all files with the nix extension
# -i to ignore case
❯ grep -i atuin **/*.nix
home-manager/amd-pc.nix:    ./modules/atuin.nix
home-manager/dell-xps.nix:    ./modules/atuin.nix
home-manager/modules/atuin.nix:  # go to https://github.com/atuinsh/atuin/issues/952
home-manager/modules/atuin.nix:  programs.atuin = {
home-manager/modules/atuin.nix:    package = pkgs-unstable.atuin;
home-manager/modules/atuin.nix:    # https://github.com/atuinsh/atuin/issues/1199#issuecomment-1940931241
home-manager/odroid.nix:    ./modules/atuin.nix
hosts/amd-pc/configuration.nix:  systemd.services.mount-atuin = {
hosts/amd-pc/configuration.nix:      ExecStart = "${pkgs.utillinux}/bin/mount /dev/zvol/rpool/nixos/atuin /home/flakm/.local/share/atuin";
hosts/odroid/configuration.nix:  systemd.services.mount-atuin = {
hosts/odroid/configuration.nix:      ExecStart = "${pkgs.utillinux}/bin/mount /dev/zvol/rpool/nixos/atuin /home/flakm/.local/share/atuin";
```
We can give more complex queries using `-e` (extended regex) flags, for example, to limit results only to links with atuin
```bash
# look for lines matching regex similar to url link
❯ grep -e "http.*atuin.*" **/*.nix
home-manager/modules/atuin.nix:  # go to https://github.com/atuinsh/atuin/issues/952
home-manager/modules/atuin.nix:    # https://github.com/atuinsh/atuin/issues/1199#issuecomment-1940931241
```
To get more context, we can extend the returned range with flag `-C` (context) and `-n` (number):

```bash
❯ grep -e "http.*atuin.*" **/*.nix -C 5 -n
home-manager/modules/atuin.nix-1-{ pkgs-unstable, ... }:
home-manager/modules/atuin.nix-2-{
home-manager/modules/atuin.nix-3-  # to address a slow startup sometimes
home-manager/modules/atuin.nix:4:  # go to https://github.com/atuinsh/atuin/issues/952
home-manager/modules/atuin.nix-5-  programs.atuin = {
home-manager/modules/atuin.nix-6-    enable = true;
home-manager/modules/atuin.nix-7-    enableZshIntegration = true;
home-manager/modules/atuin.nix-8-    package = pkgs-unstable.atuin;
home-manager/modules/atuin.nix:9:    # https://github.com/atuinsh/atuin/issues/1199#issuecomment-1940931241
home-manager/modules/atuin.nix-10-    settings = {
home-manager/modules/atuin.nix-11-      sync = {
home-manager/modules/atuin.nix-12-        records = true;
home-manager/modules/atuin.nix-13-      };
home-manager/modules/atuin.nix-14-      # use this to disable auto sync
```
## `ripgrep` - modern `grep` alternative

You might have noticed that I had to specify `**/*.nix` glob to specify which files grep should search for matches. Simple `grep atuin` didn't return for a couple of minutes in a simple project.
[`ripgrep`](https://github.com/BurntSushi/ripgrep) is a modern git alternative. It does several smart things to be less surprising for users:
- it checks .gitignore files to avoid crunching unnecessary files
- it uses smart directory traversal
- it uses author's regex matching engine

You can give [this](https://blog.burntsushi.net/ripgrep/) blog entry a read to find out more information from the author.

TLDR: with `ripgrep` you can now simply do `rg atuin`:

{{< figure src="/images/finding_stuff/rg_output.png" class="img-sm">}}

Notice the more sane defaults - line numbers, colored output, and grouped matches by file name or excellent features like sorting:

```bash
# zsh autocompletions after typing tab
# sorting switches into single threaded moment
❯ rg atuin --sort=accessed
accessed  -- sort by last accessed time
created   -- sort by creation time
modified  -- sort by last modified time
none      -- no sorting
path      -- sort by file path
```

`ripgrep` is multi-threaded, which helps search large file systems.

## Visualize structure with `rg` and `as-tree`

The hook of using a terminal is about joining small programs and creating more complex utilities.
Let's imagine trying to figure out the structure of files that relate to rust:

```bash
❯ rg rust -l | as-tree
.
├── README.md
├── configuration.nix
├── flake.lock
├── home-manager
│   ├── amd-pc.nix
│   ├── dell-xps.nix
│   └── modules
│       ├── i3.nix
│       ├── nvim
│       │   ├── config
│       │   │   ├── init.vim
│       │   │   ├── lsp-config.vim
│       │   │   └── rust-config.lua
│       │   └── neovim.nix
│       ├── rust.nix
│       └── zsh.nix
└── hosts
    ├── amd-pc
    │   ├── configuration.nix
    │   └── postgres.nix
    ├── dell-xps
    │   └── configuration.nix
    └── odroid
        ├── nextcloud.nix
        └── postgres.nix
```

Other use cases I tend to use:

- `fd`, [`proximity-sort`](https://github.com/jonhoo/proximity-sort) and [`fzf`](https://github.com/junegunn/fzf) to create [smart goto utility](https://github.com/FlakM/nix_dots/blob/66d7942bdbe1a7dcacfc5f6b2818f46c5da78987/home-manager/modules/nvim/config/init.vim#L216) for vim
- `curl` with pipe to `grep` to locate specific headers like: `curl "https://google.com" -i 2>/dev/null | grep cache`
- As suggested by [supafly1974](https://www.reddit.com/r/linux/comments/1bpvkuz/comment/kx0o6ne) `history | cut -c 8- | sort -u | fzf +m -e | tr -d '\\n' | xclip -selection c`

## Looking back in history with `git`

Let's say you are sure that a specific word like key or environemnt variable was previously changed but it's not present in working tree.
Provided that you are using `git` you may find it useful to look in `gits` history.

```bash
# -G (grep) Look for differences whose patch text contains added/removed lines that match
# -p (patch) generate patch
❯ git log -G _atuin_search_widget -p
```
{{< figure src="/images/finding_stuff/git_log.png" class="img-sm">}}

## Using modern git diff pager `delta`

To get even nicer output from diffs/logs using git you can install `delta` and configure git to use as a pager. The same command `git log -G` produces better output:

{{< figure src="/images/finding_stuff/git_log_delta.png" class="img-sm">}}

It will include syntax-highlighting and finer configurations like `n` and `N` moving between diffs.

## Looking for files by name, `find` and `fd`

Let's say we want to find a file in the current directory recursively. We can do it by using `find`:

```bash
❯ find . -name "*atuin*.nix"
./home-manager/modules/atuin.nix
```

We can use `find` to locate files modified today:
```bash
# -mmin -$((3 * 60)) for last 3 hours
# -type d for directories
❯ find . -type f -mtime 0
...
./.git/objects/97/042486eb5085b56007af556a99be1d640eb597
./.git/index
./flake.lock
./home-manager/modules/alacritty.nix
./home-manager/modules/git.nix
./home-manager/modules/zsh.nix
./home-manager/modules/common.nix
```

Find command can be built up using `-exec` or `xargs`

```bash
❯ find . -type f -mmin -$((3 * 60)) -exec du -h {} +
...
9.0K    ./.git/objects/97/042486eb5085b56007af556a99be1d640eb597
9.0K    ./.git/index
9.0K    ./home-manager/modules/alacritty.nix
9.0K    ./home-manager/modules/git.nix
```

There is also modern alternative [`fd`](https://github.com/sharkdp/fd) that is also simpler to use and has nicer output:

{{< figure src="/images/finding_stuff/fd.png" class="img-sm">}}

It doesn't show files that are ignored by `.gitignore` it has nice colors. It's quite handy to find versions of some library:

```bash
❯ fd "libc.so" /nix/store --type=f
/nix/store/gqghjch4p1s69sv4mcjksb2kb65rwqjy-glibc-2.38-23/lib/libc.so.6
/nix/store/gqghjch4p1s69sv4mcjksb2kb65rwqjy-glibc-2.38-23/lib/libc.so
/nix/store/xmprbk52mlcdsljz66m8yf7cf0xf36n1-glibc-2.38-44/lib/libc.so
/nix/store/xmprbk52mlcdsljz66m8yf7cf0xf36n1-glibc-2.38-44/lib/libc.so.6
/nix/store/wvgyhnd3rn6dhxzbr5r71gx2q9mhgshj-glibc-2.32-48/lib/libc.so
/nix/store/7jiqcrg061xi5clniy7z5pvkc4jiaqav-glibc-2.38-27/lib/libc.so
/nix/store/7jiqcrg061xi5clniy7z5pvkc4jiaqav-glibc-2.38-27/lib/libc.so.6
/nix/store/1zy01hjzwvvia6h9dq5xar88v77fgh9x-glibc-2.38-44/lib/libc.so
/nix/store/1zy01hjzwvvia6h9dq5xar88v77fgh9x-glibc-2.38-44/lib/libc.so.6
```
## Finding shell history items with atuin


I regret not knowing about `CTRL+R` and `history` for a couple of years of using linux. 
But in this case, there is also an amazing utility that can do it.
[`atuin`](https://github.com/atuinsh/atuin) is a must-use tool.

It stores your shell history in the local SQLite database and binds new widgets to `CTRL+R` and the up arrow.
It also supports end-to-end encrypted sync service that can be self-hosted and allows joining history from different machines.
It uses fuzzy logic to find relevant commands.

{{< figure src="/images/finding_stuff/atuin.png" class="img-sm">}}

## Changing the working directory with `zoxide`

[`zoxide`](https://github.com/ajeetdsouza/zoxide) is a small utility that helps with changing directory. It stores frequently visited directories to help with jumping without remembering the full path.

```bash
z ripgrep # changes CWD to highest ranked directory matching ripgrep, ie /home/me/code/ripgrep
```

## Finding what is eating up the disk space

Without resorting to tui tools like `ncdu` we can check it by using `du` chained with sort and head (as suggested by [BigHeadTonyT](https://www.reddit.com/r/linux/comments/1bpvkuz/comment/kwz2pze/)).

```bash
du -h ~ | sort -hr | head -n 10
```

But as always, there is a more modern alternative called dust that has a similar syntax:

```bash
dust ~ -n 10
```
Here is an [`asciinema`](https://asciinema.org/) recording comparing the output of those two tools:

{{< unsafe >}}
<script src="https://asciinema.org/a/ofH2tuxqWpSrmnPzIBBLZsspI.js" id="asciicast-649642" async="true"></script>
{{< /unsafe >}}

Notice how the `du` version took over 4 seconds on a single core, and the `dust` finished in 0.6 seconds and returned arguably more insightful information.

