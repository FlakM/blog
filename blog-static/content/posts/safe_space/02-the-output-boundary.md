+++
title = "Safe space for coding agents: the output boundary"
date = 2026-06-03T09:10:00+02:00
draft = true
series = ["Safe space for coding agents"]
toc = true
categories = ["nixos", "security", "llm"]
tags = ["sandbox", "llm", "git", "review"]
+++

- Introduce the distinction between generated output and trusted output.
- Frame the output boundary as the first safety layer.
- Use a disposable workspace before using containers.
- Try `git worktree` as the lightweight path.
- Try a throwaway clone under `/tmp` as the simplest path.
- Run the hostile harness in the disposable workspace.
- Run a small real coding-agent task in the disposable workspace.
- Review every change with `git diff`.
- Promote only selected changes back to the real workspace.
- Delete the disposable workspace as the rollback mechanism.
- Explain what this prevents: accidental trust, unreviewed merge, and pollution of the real checkout.
- Explain what this still allows: unrestricted edits inside the disposable workspace, full host network, and full host resources.
- Show that this does not protect host secrets or prevent resource abuse.
- Treat this as the output-review boundary from the ADRs.
- End with the next step: restrict what the agent process can see.
