+++
title = "Safe space for coding agents: declarative devbox with NixOS nspawn"
date = 2026-06-03T10:10:00+02:00
draft = true
series = ["Safe space for coding agents"]
toc = true
categories = ["nixos", "security", "llm"]
tags = ["sandbox", "nixos", "nspawn", "systemd"]
+++

- Present NixOS nspawn as the integration point, not the first experiment.
- Add `hosts/amd-pc/devbox.nix` in `nix_dots`.
- Define `containers.devbox` as a full nested NixOS system.
- Use `privateNetwork = true`.
- Configure stable host and container addresses.
- Add SSH as the primary entry point.
- Add basic development packages.
- Share the host `/nix/store` read-only through normal NixOS container behavior.
- Mount the ZFS-backed work dataset into the container.
- Use an ephemeral root so the OS resets cleanly.
- Apply cgroup limits at the container service level.
- Apply NAT, host deny, LAN deny, and egress logging on the host.
- Re-run the hostile harness inside `devbox`.
- Explain what this prevents: ad-hoc drift and one-off sandbox setup.
- Explain what this still allows: a fast full NixOS dev environment with shared store efficiency.
- Compare briefly with the existing `clawd` microVM pattern and explain why it remains the high-isolation tier.
- End with the next step: make the sandbox pleasant enough to use every day.
