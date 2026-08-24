+++
title = "Safe space for coding agents: process isolation"
date = 2026-06-03T09:20:00+02:00
draft = true
series = ["Safe space for coding agents"]
toc = true
categories = ["nixos", "security", "llm"]
tags = ["sandbox", "bubblewrap", "unshare", "systemd", "process-isolation"]
+++

- Start below NixOS containers to understand the smallest useful boundary.
- Compare `bubblewrap`, `unshare`, and `systemd-run --user` as process-level isolation tools.
- Expose only the disposable workspace to the process.
- Hide the fake host secret from the isolated process.
- Make the host filesystem read-only or invisible where possible.
- Run the hostile harness inside the process sandbox.
- Check whether writes outside the workspace fail.
- Check whether reading the fake secret fails.
- Check whether the process can still reach the internet.
- Check whether the process can still consume all memory or CPU.
- Explain what this prevents: many accidental filesystem mistakes and some simple host reads.
- Explain what this still allows: network access, resource abuse, and shared-kernel risk.
- Make clear that process isolation is a useful layer but not the final answer.
- Connect this to rejected ADR alternatives: useful complement, not primary sandbox.
- End with the next step: add resource controls to bound runaway behavior.
