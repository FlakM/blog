+++
title = "You shall not pass: threat modeling coding agents"
date = 2026-06-03T09:00:00+02:00
draft = true
series = ["Safe space for coding agents"]
toc = true
categories = ["nixos", "security", "llm"]
tags = ["sandbox", "llm", "security", "threat-model"]
+++

## Development in the age of coding agents

I feel so many things about the recent changes caused by the advent of ai. Sadness about parting way with the long hours crafting code in the editor.
Jelousy about the speed at which the new tools are jumping around the codebase and workflow, even after sharpening my tools and skills for year their grep-foo in unparalleled.
Excitement about the ease at which I can now learn new technologies - debug complex problems that used to be a nightmare.
But most of all I feel fleeting discomfort about the speed and the lack of old safety mechanisms in our systems.
The tools are getting built faster in non deterministic ways, and they are getting more powerful and more integrated into the workflow.
It would be only natural to give them less trust than in the past - but somehow I find myself giving them more and more trust, even when I know I shouldn't. I want to fix that.

The new bread of tools in my workflow consist of mix of cli tools with very wide capabilities, gh for doing github operations, gws for google related operations, jira cli, and of course coding agents that have mcps wired in.
Some of those tools are themselves built rapidly with a lot of ai assistance - with no clear security best practices in mind.
In the past the element that made it all more secure was the slow pace of development and high oversight - the fact that I was the only one doing those things on my machine.

I'd prefer to put that genie in the box and return to the familiar world of the past - but the reality is that employers and clients expect proficiency in those areas.
I detest the fact that we are giving the means of production to the companies like anthropic, openai and others but I'm optimistic about the local models and open source alternatives that are popping up.
Hopefully albait naively maybe the open source will as usually take over the market over time.

In the meantime, let's try to do something about it.

## The new guals

Let's start with the goals of this series. It's the documentation for my journey to build a safer framework for working with those tools.

- Deterministism - I want to control the tools available for the agents, version them and have a clear understanding
- Isolation - I want to be able to say with confidence that a tool cannot do X, Y, Z things on my machine, and have technical controls to back that up
- Observability - I want to know what commands ai agent ran and what they did to the system, so I can learn from it and improve the sandbox over time
- Recovery - I want to be able to easily undo the changes if something goes wrong, and have a clear audit trail of what happened
- Usability - I want the sandbox to be easy enough to use that it becomes a natural part of my workflow, not a chore that I avoid

## Threat modelling

1. Agents doing stuff to my host machine
    - file system access
    - controling security settings
    - leaking host secrets by accident
2. Development tooling created for the agents that goes rouge
    - malicious dependency (see bitwarden cli incident)
    - supply chain attack on a trusted tool (see npm install incidents)
3. Prompt injection attack that turns a coding agent into a data exfiltration tool on host
    - someone able to inject an instruction to read a file and send it to an external server

What are non goals:

1. Preventing all possible bad outcomes. The goal is not to make a perfect sandbox that can prevent every possible attack, but to make a sandbox that can mitigate the most likely and most damaging attacks.
2. Preventing ai from doing dumb shit
3. Protecting from commiting bad code

## The sandbox mindset

Let's start with the threat of coding agents and their tools doing something bad on the host machine. Traditionally, the developer machine has been a place of implicit trust.
I want to start to treat it as a more hostile environment - kind of like the hypervisor in the cloud do. From now on I assume that it's shared with a bunch of trolls that want to mess with me.

Normally this was the task of either virtualization or containers. Realistically I don't want to guard from kernel vulnerabilities - that's a different threat model from mine. Containers seem like a better fit for the job - they are lighter since they reuse the host kernel, and they can still provide a good level of isolation if configured correctly. I'm using nixos as a host so using docker or podman seems like doing the same work twice - all of the layers will be stored in `/nix/store` anyway so I've decided to go with `systemd-nspawn` as the container technology for this series.
It's a native systemd technology that nixos supports well, and it gives me a good level of isolation without the overhead of a full VM.

## About nspawn

Nspawn according to `man systemd-nspawn` can spawn a command or OS in a lightwight container. It does the following things:
    - virtulizes the file system
    - process tree
    - ipc subsystems
    - host and domain names
    - restricts kernel interfaces to read only (`/sys`, `/proc/sys`...) and limits capabilities for modyfing the host netwwork interfaces or system clock
It can be run using command line interface or by defining a system service in the background. `machinectl` is the management tool for nspawn containers.


## Experinment one - our first container

Let's dive in - first let's understand how to create a container and poke it a bit. First we need some operating system tree for the container

```bash
CID=$(docker create ubuntu)
sudo mkdir -p /var/lib/machines/ubuntu
docker export "$CID" | sudo tar -x -C /var/lib/machines/ubuntu
docker rm "$CID"
sudo systemd-nspawn -M ubuntu \
    --setenv=PATH=/usr/bin:/bin:/usr/sbin:/sbin \
░ Spawning container ubuntu on /var/lib/machines/ubuntu.
░ Press Ctrl-] three times within 1s to kill container; two times followed by r
░ to reboot container; two times followed by p to poweroff container.
root@ubuntu:/# apt-get update && apt-get install -y systemd dbus
root@ubuntu:/# exit
```


We have exited the shell and we can now start the machine again with `machinectl start ubuntu` and `machinectl shell ubuntu` to get back in. We can also stop it with `machinectl stop ubuntu` and check the logs with `journalctl -M ubuntu`.
And now inspect the status:

```bash
❯ sudo machinectl status ubuntu
ubuntu(919ea52223b24a8295faf7e0fb0b7642)
           Since: Fri 2026-06-05 15:31:17 CEST; 1min 34s ago
          Leader: 383023 (systemd)
         Superv.: 383020 (systemd-nspawn)
         Service: systemd-nspawn; class container
            Root: /var/lib/machines/ubuntu
           Iface: ve-ubuntu
              OS: Ubuntu 26.04 LTS
        ID Shift: 238288896
            Unit: systemd-nspawn@ubuntu.service
        Subgroup: payload
                  ├─init.scope
                  │ └─383023 /usr/lib/systemd/systemd
                  └─system.slice
                    ├─console-getty.service
                    │ └─383183 /usr/sbin/agetty --noreset --noclear --issue-file=/etc/issue:/etc/issue.d:/run/issue.d:/usr/lib/issue.d --keep-baud 115200,57600,38400,9600 - dumb
                    ├─dbus.service
                    │ └─383173 @dbus-daemon --system --address=systemd: --nofork --nopidfile --systemd-activation --syslog-only
                    ├─system-container\x2dshell.slice
                    │ └─container-shell@1.service
                    │   ├─386565 /bin/bash -l
                    │   └─386567 "(sd-"
                    ├─systemd-journald.service
                    │ └─383147 /usr/lib/systemd/systemd-journald
                    ├─systemd-logind.service
                    │ └─383174 /usr/lib/systemd/systemd-logind
                    └─systemd-resolved.service
                      └─383158 /usr/lib/systemd/systemd-resolved
```

Now let's mount a ZFS dataset into a fresh, unshifted container rootfs so my work
lives on its own snapshot-able, quota-capped dataset — and ends up owned by *me*
on the host, not by some mapped uid.

```bash
sudo zfs create -p -o compression=zstd -o refquota=2G \
  -o mountpoint=/srv/ubuntu-work rpool/nixos/containers/ubuntu-work

# make sure the dataset is owned by my user so it shows up as owned inside the container with idmap
sudo chown 1000:100 /srv/ubuntu-work

# bind the dataset with idmap; auto keeps the rootfs unshifted when possible
sudo mkdir -p /run/systemd/system/systemd-nspawn@ubuntu.service.d
sudo tee /run/systemd/system/systemd-nspawn@ubuntu.service.d/override.conf >/dev/null <<EOF
[Service]
ExecStart=
ExecStart=systemd-nspawn --quiet --keep-unit --boot --link-journal=try-guest --network-veth --private-users=1048576:65536 --private-users-ownership=auto --bind=/srv/ubuntu-work:/work:rootidmap --machine=%i
EOF
sudo systemctl daemon-reload
sudo machinectl start ubuntu
```

Create a uid-1000 user in the container, then write a file as that user:

```bash
sudo machinectl shell root@ubuntu /usr/bin/bash -lc \
  'id -u ubuntu >/dev/null 2>&1 || useradd -m -u 1000 -s /bin/bash ubuntu'
sudo machinectl shell ubuntu@ubuntu /usr/bin/bash -lc 'echo hi > /work/hello'
# on the host /srv/ubuntu-work/hello is now owned by my user (uid 1000)
```

Three pieces make the ownership line up:

- `--private-users=1048576:65536` — user namespacing on; container root maps to
  an unprivileged host uid (1048576) that owns nothing. The base must be a
  multiple of 65536.
- `--private-users-ownership=auto` — nspawn uses an idmapped mount for the
  rootfs when the filesystem supports it, and falls back to chowning when it
  cannot. On my ZFS-backed rootfs it kept the files owned by root on disk.
- `--bind=…:idmap` + running as uid 1000 — `idmap` maps container uid *z* to host
  uid *z*, so a uid-1000 process inside writes files owned by uid 1000 (me).

### What `auto` does not solve

`auto` only controls the container rootfs ownership. It does not make a regular
bind mount writable by uid 1000 inside the container. Without `:idmap`, the work
directory still shows up as a host-owned directory whose owner is outside the
container's uid range, and writes fail with `EACCES`.

This is adjacent to, but not the same as,
[systemd#36470](https://github.com/systemd/systemd/issues/36470). That issue is
about nspawn entering the user namespace before setting up some bind mounts, so
the bind source can become inaccessible before it is mounted. I did not reproduce
that failure here: `--private-users-ownership=auto` worked for the rootfs, and
`--bind=…:idmap` worked for `/work`.

I also tried `rootidmap` and `owneridmap` for the work bind. On this stack they
made `/work` appear owned by root inside the container, but writes still failed.
The reliable version was plain `idmap` plus running the payload as uid 1000.


- Apply the same thinking to coding agents and vibe-coded tooling: assume that at some point something will delete the wrong files, leak the wrong thing, or run the wrong command.
- The question becomes how small the blast radius is and how quickly I can recover.
- Different contexts require different confinement.
- A quick throwaway experiment can tolerate less scrutiny.
- Work with credentials, private repositories, customer data, or deployment access needs more care and attention.
- Running a random dependency install is a different risk than asking an agent to rename a local variable.
- The goal is not one perfect sandbox mode for every task.
- The goal is a set of boundaries that can be tightened depending on context.
- All contexts can still benefit from better security, better observability, and more rigid boundaries.
- In this series, go over the concrete threats I want to mitigate.
- Threat 1: tooling with unlimited host access.
- Coding agents, package managers, shell scripts, editor plugins, CLIs, and generated scripts often run with access that is effectively god-like on the host.
- Even without literal `sudo`, a tool running as my user can delete projects, read SSH config, read tokens, alter shell config, edit dotfiles, modify git remotes, and poison future sessions.
- With `sudo`, the blast radius becomes the whole machine.
- The uncomfortable part is that this level of access is normal in a developer workstation.
- The mitigation direction is to stop treating the host as the default place where tools run.
- The sandbox should give tools enough access to do the task, not unlimited access to the workstation.
- Threat 2: lack of borders between the AI process, its tools, and the host system.
- The AI chat, tool runner, shell, package manager, editor, git credentials, browser, and host filesystem can collapse into one large implicit trust zone.
- A prompt can become a shell command.
- A shell command can invoke a package manager.
- A package manager can run install scripts.
- Install scripts can touch the same home directory, credentials, shell startup files, and project tree as everything else.
- The danger is not only what the model decides to do, but what any tool in the chain is allowed to do after the model starts it.
- The mitigation direction is to introduce explicit borders: process boundaries, filesystem boundaries, network boundaries, credential boundaries, and output-review boundaries.
- Threat 3: no useful audit trail.
- If a run goes wrong, I may not know which prompt, command, package install, tool call, or generated script caused the problem.
- The lack of a prompt/tool/action trail makes it hard to debug incidents after the fact.
- It also makes it harder to improve the sandbox because there is no evidence of what actually happened.
- The mitigation direction is to preserve enough context: prompts or task descriptions, commands executed, filesystem changes, network attempts, resource peaks, and final diff.
- Threat 4: no network segregation.
- A normal developer shell can often reach the host, the LAN, private services, package registries, SaaS APIs, and random internet destinations from the same environment.
- That means prompt injection or a malicious dependency can turn readable files into exfiltrated files.
- The mitigation direction is to separate network spaces, block host and LAN access by default, log denied attempts, and expose only explicit ports outward.
- Threat 5: no resource limits.
- A runaway agent, test, dependency install, or generated script can consume CPU, memory, process slots, or disk until the host becomes unusable.
- This can be malicious, but it can also be a normal bug or an accidental infinite loop.
- The mitigation direction is to apply cgroup limits and storage quotas so failure is bounded.
- Threat 6: shared secrets without good visibility or rollback scenarios.
- Developer machines often accumulate secrets from many contexts: SSH agents, Bitwarden sessions, cloud CLIs, kubeconfigs, `.env` files, SaaS tokens, and private package registry credentials.
- These secrets are shared across projects because they live in the same user session.
- It is often unclear which tool accessed which secret and when.
- If a secret is touched or leaked, rotating it can be painful because the dependency graph is informal and spread across dotfiles, CLIs, and shells.
- The mitigation direction is to avoid staging broad secrets into the sandbox, prefer scoped and short-lived credentials, forward only the minimum required socket, and keep enough audit trail to know what must be rotated.
- Threat 7: supply-chain attacks through trusted-looking CLIs and dependencies.
- The risk is not only random `npm install` packages.
- Trusted operational tools like `bitwarden`, cloud CLIs, GitHub CLI, package managers, language build tools, and helper scripts can become part of the attack surface.
- A malicious or compromised dependency can run during install, build, code generation, test setup, or plugin loading.
- A compromised credential helper is especially dangerous because it is expected to touch secrets.
- The mitigation direction is to run supply-chain-heavy workflows inside the sandbox, limit secrets available to those tools, and make network/file effects visible.
- Threat 8: cross-project contamination.
- A single host user account has access to personal projects, work projects, private repositories, notes, dotfiles, caches, and credentials at the same time.
- A tool started for one project can accidentally or maliciously read another project's files.
- Generated scripts can use broad paths like `~/programming`, `~/.config`, or `~/.ssh` without realizing they crossed a context boundary.
- Cross-project contamination is especially risky when mixing personal work, employer code, client code, and experiments.
- The mitigation direction is to make project/context boundaries explicit: separate work datasets, separate sandboxes where needed, and deliberate copy-in/copy-out.
- Explain how the rest of the series will address these threats.
- The plan is to learn the pieces in isolation first instead of jumping straight to a finished sandbox.
- Start with process isolation to understand what a tool can and cannot see.
- Move to resource controls so runaway tools have bounded CPU, memory, process, and disk impact.
- Add network isolation so the sandbox can reach the public internet where needed but not the host or LAN by default.
- Add storage snapshots so a coding-agent run becomes reversible: snapshot, run, inspect, keep or roll back.
- Add audit and monitoring so the question "what happened?" has an answer after a bad run.
- Then combine the pieces in `systemd-nspawn`, because it is a native NixOS way to run a full Linux userspace with a real service boundary and shared host `/nix/store` efficiency.
- Build a dedicated `devbox` container rather than using the host as the default development environment.
- Define the container declaratively in the `nix_dots` flake so the environment is reproducible, reviewable, and not a pile of shell history.
- Put the mutable workspace on a dedicated ZFS dataset so it can be snapshotted, diffed, rolled back, quota-limited, and eventually replicated.
- Keep the container root disposable or ephemeral so system-level mess does not accumulate.
- Keep credentials staged deliberately, ideally with SSH-agent forwarding or short-lived scoped secrets instead of copying broad host secrets into the container.
- Treat port exposure as explicit: the container should not join the tailnet or LAN directly; dev servers are exposed outward only when requested.
- Make the end goal a bounded workstation: useful enough for daily coding, but with clearer borders than the host.

## The non-NixOS pieces I need to understand first

- Keep this section minimal and practical: what the primitive is, why it matters, and where the official docs live.
- For now, learn these tools directly on Linux before wrapping them in NixOS declarations.
- Later, NixOS should make the setup reproducible; it should not hide the concepts from me.
- [`systemd-nspawn`](https://www.freedesktop.org/software/systemd/man/latest/systemd-nspawn.html) is the lightweight container runner I want to understand.
- What: it runs a full Linux userspace in a container using Linux namespaces and systemd integration.
- Why: it is closer to a small machine than a single-process sandbox, but lighter than a VM.
- Why for this series: it gives me a place to run the agent and its tools without making the host the default execution environment.
- [`machinectl`](https://www.freedesktop.org/software/systemd/man/latest/machinectl.html) is the management surface for systemd machines and containers.
- What: it lists, enters, starts, stops, and inspects containers registered with systemd.
- Why: it gives me operational visibility into whether the sandbox is actually a separate machine-like context.
- [Linux namespaces](https://man7.org/linux/man-pages/man7/namespaces.7.html) are the isolation mechanism underneath the container story.
- What: they separate views of processes, mounts, users, hostnames, IPC, and networking.
- Why: the first boundary I care about is that the agent should not see the same world as the host process.
- [Network namespaces](https://man7.org/linux/man-pages/man7/network_namespaces.7.html) are especially important for the egress story.
- What: they give a container its own network devices, routes, and firewall context.
- Why: prompt injection becomes much less scary if the process cannot freely reach the host, LAN, or arbitrary internal services.
- [`cgroups v2`](https://docs.kernel.org/admin-guide/cgroup-v2.html) are the resource control mechanism.
- What: they account for and limit CPU, memory, process count, IO, and related resource usage.
- Why: a runaway agent should become a bounded failed job, not an unusable workstation.
- [`systemd.resource-control`](https://www.freedesktop.org/software/systemd/man/latest/systemd.resource-control.html) is how systemd exposes cgroup controls.
- What: options like `MemoryMax`, `CPUQuota`, `TasksMax`, and IO controls can be applied to services/scopes.
- Why: before building the final container, I can test resource boundaries with `systemd-run` and prove the limits work.
- [`systemd-run`](https://www.freedesktop.org/software/systemd/man/latest/systemd-run.html) is useful for experiments before the container exists.
- What: it starts commands in transient systemd services or scopes.
- Why: it lets me test cgroup limits and hardening without writing a full service or NixOS module.
- [`systemd.exec`](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html) contains many process hardening knobs.
- What: it documents filesystem protection, capability restrictions, private tmp, user isolation, and syscall filtering options.
- Why: some hardening can be tested on a single command before moving it into the container design.
- [Linux capabilities](https://man7.org/linux/man-pages/man7/capabilities.7.html) are the split-up version of root privileges.
- What: instead of one all-powerful root bit, privileged operations are divided into capabilities.
- Why: the sandbox should not keep capabilities it does not need.
- [`nftables`](https://wiki.nftables.org/wiki-nftables/index.php/Main_Page) is the firewall and packet filtering layer I need for egress control.
- What: it can NAT container traffic, block host/LAN destinations, and log denied attempts.
- Why: network isolation without logging leaves me blind; network isolation without deny rules leaves the host and LAN exposed.
- [`ip netns`](https://man7.org/linux/man-pages/man8/ip-netns.8.html) and [`ip-link`](https://man7.org/linux/man-pages/man8/ip-link.8.html) are useful for manually learning the networking pieces.
- What: they let me create namespaces and veth pairs by hand.
- Why: building the network once manually should make the later `nspawn` and NixOS setup less magical.
- [ZFS snapshots](https://openzfs.github.io/openzfs-docs/man/master/8/zfs-snapshot.8.html), [rollback](https://openzfs.github.io/openzfs-docs/man/master/8/zfs-rollback.8.html), and [diff](https://openzfs.github.io/openzfs-docs/man/master/8/zfs-diff.8.html) are the recovery and audit tools.
- What: snapshots freeze workspace state, diffs show file-level changes, rollback reverts the dataset.
- Why: the agent can make a mess inside the workspace if I can cheaply inspect and undo it.
- [ZFS quotas and reservations](https://openzfs.github.io/openzfs-docs/man/master/8/zfsprops.8.html) matter for disk blast radius.
- What: properties like `refquota` cap how much a dataset can consume.
- Why: disk exhaustion is a resource problem too, and it should be bounded like CPU and memory.
- DNS deserves its own tracking story.
- What: DNS is the map of where tools try to go, and it often reveals intent before the actual connection does.
- Why: if an agent or dependency tries to resolve `evil-example.test`, an internal service name, or a paste/exfiltration endpoint, I want that visible.
- The simple DNS control model: point the guest at a host-controlled resolver, log every query there, and block direct DNS to anywhere else.
- Candidate resolvers: [`dnsmasq`](https://thekelleys.org.uk/dnsmasq/docs/dnsmasq-man.html) with query logging, [`Unbound`](https://nlnetlabs.nl/documentation/unbound/unbound.conf/) with logging, or [`CoreDNS`](https://coredns.io/plugins/log/) with the log plugin.
- The host can run a resolver bound only to the container bridge/veth address.
- The guest gets `/etc/resolv.conf` pointing at that resolver.
- nftables blocks outbound UDP/TCP port `53` from the guest except to the host resolver.
- nftables also blocks DNS-over-TLS port `853` unless explicitly allowed.
- DNS-over-HTTPS is harder because it hides inside normal HTTPS on port `443`.
- DoH mitigation options: do not install browsers with DoH enabled, disable DoH in sandbox Firefox, block known DoH providers, or move toward an allowlist/proxy model for egress.
- Be honest in the post: DNS logging is strong for normal resolver traffic, but it does not magically reveal encrypted DoH queries.
- DNS policy can also limit capability, not only observe it.
- The sandbox resolver should avoid internal split-horizon DNS so names like homelab services do not resolve in the first place.
- The network firewall should still block RFC1918 even if a name resolves to a private address.
- The guest should not have `CAP_NET_ADMIN`, so tools cannot rewrite routes, add interfaces, or bypass DNS policy by changing network setup.
- Keep `/etc/resolv.conf` generated or read-only where practical so the sandbox has one intended DNS path.
- Host/guest auditing needs a layered story.
- Host-side audit is the source of truth for boundaries the guest should not control.
- Host-side signals: `systemd-nspawn` unit logs, `machinectl` state, cgroup accounting, nftables logs/counters, conntrack, resolver query logs, ZFS snapshots/diffs, and host-side port exposure state.
- Guest-side audit explains what happened inside the sandbox.
- Guest-side signals: shell history, tmux logs or transcripts, guest journald, package manager logs, build logs, git diff, and application logs.
- [`journalctl -M`](https://www.freedesktop.org/software/systemd/man/latest/journalctl.html) can inspect logs for a registered machine/container when the setup supports it.
- [`systemd-nspawn --link-journal`](https://www.freedesktop.org/software/systemd/man/latest/systemd-nspawn.html) is worth learning because it controls how guest journals relate to the host journal.
- Linux audit via [`auditd`](https://man7.org/linux/man-pages/man8/auditd.8.html) is mostly a host-kernel-level tool, so treat it as host-side infrastructure rather than something the agent can administer.
- For early experiments, avoid overbuilding auditd rules; start with logs that answer practical questions.
- Practical audit questions: which prompt/task started the run, which commands ran, which files changed, which DNS names resolved, which IPs were contacted, which connections were denied, which resources peaked, and whether rollback happened.
- Server-layer auditing should separate control-plane events from guest activity.
- Control-plane events: container start/stop/restart, snapshot/rollback, port expose/unexpose, credential staging, and policy changes.
- Guest activity: commands, package installs, file changes, network attempts, and application logs.
- The future `devbox audit` command should merge these views instead of pretending one log source is enough.
- The story thread: namespaces define borders, cgroups define budgets, nftables defines egress, ZFS defines recovery, and systemd-nspawn ties these into a machine-like container.
- The first half of the series should learn these pieces directly; the second half should make them declarative and ergonomic.
- Make clear this is not an anti-AI-coding rant.
- Say that AI-assisted coding can absolutely produce better code than working without it.
- The problem is that it is too easy to fall into the trust trap.
- Use the `Thinking, Fast and Slow` idea: what you see is all there is.
- Once an agent does one impressive task well, it is tempting to let that success stand in for evidence about everything else.
- The agent being good at refactoring one function does not prove it is safe with credentials, package installs, network access, or destructive commands.
- Tie this to trust calibration: the sandbox is not about distrusting every agent action, but about making trust earned and observable.
- Then introduce the real goal: if a coding agent breaks the dev machine, it should not be a big deal.
- Avoid starting with tools like NixOS, nspawn, or microVMs.
- Define the concrete risks: accidental deletion, prompt-injection exfiltration, runaway dependencies, malicious install scripts, resource abuse, and unreviewed output.
- Build a hostile toy workload that acts like a reckless agent.
- Include attempts to delete files inside and outside the workspace.
- Include an attempt to read a fake host secret outside the workspace.
- Include an attempt to spawn too many processes.
- Include an attempt to allocate too much memory.
- Include an attempt to write a huge file.
- Include public internet access checks.
- Include host IP and LAN IP access checks.
- Include a local dev server check.
- Turn the harness into a result table used by every later post.
- Make the expected final behavior explicit: host secret denied, host writes denied, LAN denied, GitHub allowed, resources bounded, disk bounded, workspace changes auditable.
- Explain that the harness prevents nothing by itself.
- Explain that the harness makes vague security claims testable.
- End with the next step: create an output boundary before adding any sandbox technology.
