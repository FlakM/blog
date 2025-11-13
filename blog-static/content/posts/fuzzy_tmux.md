---
title: "Keeping all the balls on fire: tmux-fzf to the rescue 🔥"
date: 2025-11-13T08:42:48+01:00
draft: false
---

## Speed isn’t the problem - stopping is

I’ve got three lovely kids, a job I enjoy, and a life that’s basically a juggling act where all the balls might also be on fire.

Weirdly enough, going fast isn't the issue. Slowing down is. Focusing on one thing, ignoring distractions, avoiding the tax of constant context switching… that’s the real challenge.

So I try to keep workflows that help me remember which ball is currently on fire. One of those frictions used to be switching between tasks. I open a new tmux session for every thread of work I’m on - a project - and each session gets a name that helps future-me remember what present-me was doing.

Every session has several windows: usually the editor first, the terminal second, and then whatever other chaos the project demands.

### How I summon a new session

1. `ctrl-d` to quit the current session  
2. `ctrl-r` to bring up the atuin prompt  
3. Type `tmux` to find my beloved `tmux new-session -c ~ -s blog` in history  
4. `ciw` to rename it using zsh’s vim mode  
5. Profit  

Creating sessions wasn’t the problem. Switching them was.

I had to open the native tmux TUI (`ctrl-b s`), scan the list, match numbers, and press Enter. My brain already *knew* what it wanted - no need to read a list of sessions like I’m shopping for produce.

And honestly, with all the other metaphorical balls on fire, I don’t need reminders of how many there are. I just need to jump back to the… most fire-y one.

## `tmux-fzf` love at first bind 💘

After way too much searching, I finally found a ridiculously elegant solution: [tmux-fzf](https://github.com/sainnhe/tmux-fzf).

I added this bind to my [tmux.nix#L63](https://github.com/FlakM/nix_dots/blob/e7d8fbd60fe4b3987306b83d0a65c2e6d6aee347/home-manager/modules/tmux.nix#L63):

```tmux
bind-key -n "C-l" run-shell -b "${pkgs.tmuxPlugins.tmux-fzf}/share/tmux-plugins/tmux-fzf/scripts/session.sh switch"
```

Now I can press `ctrl-l` from anywhere, fuzzy-search the session I want, and hit Enter. Fast. Effortless. Elegant.

{{< unsafe >}}
<script src="https://asciinema.org/a/755622.js" id="asciicast-755622" async="true"></script>
{{< /unsafe >}}

## The automation math has changed

I used to avoid automating small annoyances because the setup time outweighed the benefit. But LLMs flipped that math. Tasks that used to take an hour now take minutes.

And especially with text-based configurations like dotfiles or Nix, LLMs are excellent for the tiny tweaks that quietly transform your workflow.

All these micro-optimizations stack up. They let me stay focused a little longer - which, in a house with three kids, is basically a superpower.
