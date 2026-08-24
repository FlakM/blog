+++
title = "Safe space for coding agents: watching the agent"
date = 2026-06-03T10:00:00+02:00
draft = true
series = ["Safe space for coding agents"]
toc = true
categories = ["nixos", "security", "llm"]
tags = ["sandbox", "monitoring", "audit", "observability"]
+++

- Start with the question: what did the agent do?
- Combine existing tools before building custom tooling.
- Use `zfs diff` for filesystem changes.
- Use nftables logs or counters for network attempts.
- Use systemd cgroup data for CPU, memory, task count, and runtime.
- Use shell history or session transcript for command context.
- Optionally add Grafana later for live container dashboards.
- Produce a manual per-run audit report.
- Include changed files, deleted files, outbound attempts, denied network attempts, resource peaks, and rollback decision.
- Explain what this prevents: invisible behavior and trust drift.
- Explain what this still allows: real work while trust is calibrated.
- Make the case that observability improves the sandbox over time.
- Connect this to ADR-04: `devbox status` and `devbox audit` are thin wrappers over existing engines.
- End with the next step: encode the proven layers declaratively in NixOS.
