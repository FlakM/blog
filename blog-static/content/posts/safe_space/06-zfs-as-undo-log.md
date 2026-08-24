+++
title = "Safe space for coding agents: ZFS as an undo log"
date = 2026-06-03T09:50:00+02:00
draft = true
series = ["Safe space for coding agents"]
toc = true
categories = ["nixos", "security", "llm"]
tags = ["sandbox", "zfs", "rollback", "audit"]
+++

- Introduce ZFS as the host-side safety net for agent work.
- Use a dedicated dataset as the workspace.
- Snapshot before running the hostile harness or a coding-agent task.
- Run the task with full write access inside the workspace.
- Use `zfs diff` to inspect created, changed, and deleted files.
- Roll back with `zfs rollback`.
- Recover individual files through `.zfs/snapshot` if useful.
- Compare this with plain `git diff`: ZFS sees everything, not only tracked files.
- Explain why snapshot control should stay on the host, not inside the sandbox.
- Explain what this prevents: irreversible workspace damage and hidden filesystem changes.
- Explain what this still allows: full write access inside the workspace.
- Add `refquota` as disk blast-radius control.
- Connect this to ADR-02: real files on ZFS beat an opaque microVM image for this use case.
- End with the next step: combine filesystem, network, and resource evidence into an audit report.
