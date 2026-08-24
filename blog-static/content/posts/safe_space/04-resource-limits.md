+++
title = "Safe space for coding agents: resource limits"
date = 2026-06-03T09:30:00+02:00
draft = true
series = ["Safe space for coding agents"]
toc = true
categories = ["nixos", "security", "llm"]
tags = ["sandbox", "systemd", "cgroups", "limits"]
+++

- Frame resource limits as containment, not only stability tuning.
- Use `systemd-run` to run the hostile harness in a constrained cgroup.
- Test `MemoryMax` with the memory allocation attempt.
- Test `CPUQuota` with a CPU burner.
- Test `TasksMax` with the fork attempt.
- Test `RuntimeMaxSec` for long-running runaway jobs.
- Capture behavior when limits are hit: killed, throttled, or failed.
- Observe limits with `systemctl show`.
- Observe live pressure with `systemd-cgtop`.
- Add the storage side of resource control later with ZFS `refquota`.
- Explain what this prevents: host memory exhaustion, fork bombs, CPU starvation, and eventually disk fill.
- Explain what this still allows: normal builds and package installs within budget.
- Explain that failed agent tasks are acceptable if the host remains healthy.
- Connect this to the final `containers.devbox` service-level limits.
- End with the next step: bound where the agent can send data.
