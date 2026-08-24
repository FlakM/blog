+++
title = "Safe space for coding agents: making the sandbox pleasant enough to use"
date = 2026-06-03T10:20:00+02:00
draft = true
series = ["Safe space for coding agents"]
toc = true
categories = ["nixos", "security", "llm"]
tags = ["sandbox", "ux", "ssh", "waypipe", "credentials"]
+++

- Open with the principle: ergonomics is a security property.
- Make `devbox` the path of least resistance.
- Add a red-themed kitty launcher for sandbox terminals.
- Use a distinct kitty class such as `devbox-term`.
- Add Hyprland rules for a dedicated sandbox workspace and red border.
- Add red tmux status inside the container.
- Add an unmistakable starship prompt inside the container.
- Use SSH as the entry point because it carries agent forwarding and supports waypipe.
- Forward only the SSH agent for Yubikey-backed git.
- Do not copy raw private keys into the container.
- Do not forward GPG signing or raw USB by default.
- Use `waypipe ssh devbox <app>` for GUI apps.
- Use sandbox Firefox for untrusted content and `xdg-open` flows.
- Avoid opening sandbox-originated files in the credentialed host browser.
- Add explicit port exposure through `ssh -L` first.
- Consider `tailscale serve` only for persistent or remote preview needs.
- Explain what this prevents: bypassing the sandbox, confusing host and sandbox windows, raw key leakage, and browser credential bleed.
- Explain what this still allows: normal daily development, git over forwarded SSH agent, GUI apps, and webapp previews.
- Connect this to ADR-03 and ADR-04.
- End with the future direction: `devbox snapshot`, `devbox diff`, `devbox rollback`, `devbox audit`, and remote attach over Tailscale.
