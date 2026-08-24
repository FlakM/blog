+++
title = "Safe space for coding agents: network isolation"
date = 2026-06-03T09:40:00+02:00
draft = true
series = ["Safe space for coding agents"]
toc = true
categories = ["nixos", "security", "llm"]
tags = ["sandbox", "network", "nftables", "egress", "veth"]
+++

- Present egress as the AI-specific security boundary.
- Explain that prompt injection becomes exfiltration only if the agent can reach a useful destination.
- Build a manual network namespace before encoding anything in NixOS.
- Create a veth pair for the sandbox namespace.
- Add NAT so public internet works.
- Block traffic to the host address.
- Block RFC1918 LAN ranges.
- Log or count denied outbound attempts with nftables.
- Run the hostile harness network checks.
- Verify that GitHub or another public endpoint works.
- Verify that the host IP fails.
- Verify that LAN IPs fail.
- Decide whether the first version uses free public internet or a stricter allowlist.
- Explain what this prevents: host access, homelab access, simple LAN exfiltration, and accidental coupling to internal services.
- Explain what this still allows: public internet, registries, Nix caches, Anthropic API, GitHub.
- Connect this to the ADR decision: container stays off the tailnet and host proxies only explicit ports.
- End with the next step: make file changes reversible and auditable.
