# shellcheck shell=bash
# agent-msg - talk to the human over the self-hosted ntfy topic.
#
# One publish reaches every device subscribed to the topic: ntfy fans the
# message out itself, there is nothing per-device to do here.
#
# Agent messages are tagged `robot`; the human's replies (sent from the ntfy
# app or web UI) are untagged, which is how `wait` tells the two apart.
set -euo pipefail

SERVER=${NTFY_SERVER:-https://ntfy.house.flakm.com}
TOPIC=${NTFY_TOPIC:-agent-chat}
TOKEN_FILE=${NTFY_TOKEN_FILE:-/run/secrets/ntfy_agent_token}

die() {
  printf 'agent-msg: %s\n' "$*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
agent-msg - message the human on every device (ntfy topic `agent-chat`)

Usage:
  agent-msg send [options] <message>     publish a message (`-` reads stdin)
  agent-msg inbox [options]              recent conversation, oldest first
  agent-msg wait [options]               block until the human replies
  agent-msg help

send options:
  -t, --title TEXT       notification title
  -p, --priority LEVEL   min|low|default|high|urgent (or 1-5)
  -T, --tag TAG          extra tag/emoji (repeatable; `robot` is always added)
  -c, --code CODE        short correlation code for concurrent conversations
  -u, --click URL        URL opened when the notification is tapped
  -a, --attach FILE      attach a file; the message is the caption
  -m, --markdown         render the message as Markdown
  -q, --quiet            print nothing on success

inbox options:
  -s, --since TIME       how far back (default 24h, `all` for everything)
  -n, --limit N          show at most N messages (default 20)
      --json             print raw JSON lines

wait options:
  -s, --since TIME       start from `now` (default), `all`, or an ntfy time
      --timeout SECONDS  give up after N seconds (default 300)
  -c, --code CODE        only accept replies that mention this code
      --json             print the raw message object

Environment:
  NTFY_SERVER            default https://ntfy.house.flakm.com
  NTFY_TOPIC             default agent-chat
  NTFY_TOKEN_FILE        default /run/secrets/ntfy_agent_token
  NTFY_TOKEN             inline bearer token, overrides the file
EOF
}

token() {
  if [[ -n ${NTFY_TOKEN:-} ]]; then
    printf '%s' "$NTFY_TOKEN"
    return 0
  fi
  [[ -r $TOKEN_FILE ]] ||
    die "cannot read token file $TOKEN_FILE (sops secret ntfy_agent_token)"
  tr -d '\r\n' <"$TOKEN_FILE"
}

uri() {
  jq -rn --arg v "$1" '$v | @uri'
}

payload_id() {
  jq -r '"sent \(.id) to \(.topic)"' <<<"$1" 2>/dev/null || printf 'sent\n'
}

cmd_send() {
  local title='' priority='default' click='' attach='' code='' quiet=0 markdown=0
  local -a tags=('robot')
  local -a words=()

  while (($#)); do
    case $1 in
      -t | --title) title=${2-}; shift 2 || die "$1 needs a value" ;;
      -p | --priority) priority=${2-}; shift 2 || die "$1 needs a value" ;;
      -T | --tag) tags+=("${2-}"); shift 2 || die "$1 needs a value" ;;
      -c | --code) code=${2-}; shift 2 || die "$1 needs a value" ;;
      -u | --click) click=${2-}; shift 2 || die "$1 needs a value" ;;
      -a | --attach) attach=${2-}; shift 2 || die "$1 needs a value" ;;
      -m | --markdown) markdown=1; shift ;;
      -q | --quiet) quiet=1; shift ;;
      -h | --help) usage; exit 0 ;;
      --) shift; words=("$@"); break ;;
      -) words=("$@"); break ;;
      -*) die "unknown option: $1 (try agent-msg help)" ;;
      *) words=("$@"); break ;;
    esac
  done

  local message="${words[*]-}"
  if [[ $message == '-' ]]; then
    message=$(cat)
  fi
  [[ -n $message || -n $attach ]] || die 'empty message'

  case $priority in
    min | 1) priority=1 ;;
    low | 2) priority=2 ;;
    default | normal | 3) priority=3 ;;
    high | 4) priority=4 ;;
    max | urgent | 5) priority=5 ;;
    *) die "invalid priority: $priority" ;;
  esac

  if [[ -n $code ]]; then
    tags+=("code-$code")
    if [[ -n $title ]]; then
      title="[$code] $title"
    else
      title="[$code]"
    fi
  fi

  local tok
  tok=$(token)
  local resp

  if [[ -n $attach ]]; then
    [[ -f $attach ]] || die "no such file: $attach"
    local taglist url
    taglist=$(IFS=,; printf '%s' "${tags[*]}")
    url="$SERVER/$TOPIC?filename=$(uri "$(basename "$attach")")&priority=$priority&tags=$(uri "$taglist")"
    [[ -n $title ]] && url+="&title=$(uri "$title")"
    [[ -n $message ]] && url+="&message=$(uri "$message")"
    [[ -n $click ]] && url+="&click=$(uri "$click")"
    resp=$(curl -fsS -m 60 -H "Authorization: Bearer $tok" \
      -H "Filename: $(basename "$attach")" -T "$attach" "$url") ||
      die "attachment upload failed"
  else
    local tags_json payload
    tags_json=$(jq -cn '$ARGS.positional' --args "${tags[@]}")
    payload=$(jq -cn \
      --arg topic "$TOPIC" \
      --arg message "$message" \
      --arg title "$title" \
      --arg click "$click" \
      --argjson priority "$priority" \
      --argjson tags "$tags_json" \
      --argjson markdown "$markdown" \
      '{topic: $topic, message: $message, priority: $priority, tags: $tags}
       + (if $title == "" then {} else {title: $title} end)
       + (if $click == "" then {} else {click: $click} end)
       + (if $markdown == 1 then {markdown: true} else {} end)')
    resp=$(curl -fsS -m 20 -H "Authorization: Bearer $tok" \
      -H 'Content-Type: application/json' -d "$payload" "$SERVER/") ||
      die "publish failed"
  fi

  ((quiet)) || payload_id "$resp"
}

cmd_inbox() {
  local since=24h limit=20 json=0
  while (($#)); do
    case $1 in
      -s | --since) since=${2-}; shift 2 || die "$1 needs a value" ;;
      -n | --limit) limit=${2-}; shift 2 || die "$1 needs a value" ;;
      --json) json=1; shift ;;
      -h | --help) usage; exit 0 ;;
      *) die "unknown option: $1 (try agent-msg help)" ;;
    esac
  done

  local tok out
  tok=$(token)
  out=$(curl -fsS -m 20 -H "Authorization: Bearer $tok" \
    "$SERVER/$TOPIC/json?poll=1&since=$since") || die "poll failed"

  if ((json)); then
    printf '%s\n' "$out" | sed '/^$/d'
    return 0
  fi

  printf '%s\n' "$out" | jq -rs --argjson limit "$limit" '
    map(select(.event == "message")) | .[-$limit:][]
    | "\(.time | localtime | strftime("%Y-%m-%d %H:%M"))  \(if ((.tags // []) | index("robot")) then "agent" else "you  " end)  \(.title // "")"
      + (if (.message // "") == "" then ""
         else "\n  " + (.message | gsub("\n"; "\n  ")) end)
  '
}

cmd_wait() {
  local since=now timeout=300 code='' json=0
  while (($#)); do
    case $1 in
      -s | --since) since=${2-}; shift 2 || die "$1 needs a value" ;;
      --timeout) timeout=${2-}; shift 2 || die "$1 needs a value" ;;
      -c | --code) code=${2-}; shift 2 || die "$1 needs a value" ;;
      --json) json=1; shift ;;
      -h | --help) usage; exit 0 ;;
      *) die "unknown option: $1 (try agent-msg help)" ;;
    esac
  done

  local start
  if [[ $since == now ]]; then
    start=$(date +%s)
  else
    start=$since
  fi

  local tok deadline msg
  tok=$(token)
  deadline=$(($(date +%s) + timeout))

  while :; do
    local out
    out=$(curl -fsS -m 15 -H "Authorization: Bearer $tok" \
      "$SERVER/$TOPIC/json?poll=1&since=$start") ||
      die "poll failed (is $SERVER reachable?)"

    msg=$(printf '%s\n' "$out" | jq -c --arg code "$code" '
      select(.event == "message")
      | select(((.tags // []) | index("robot")) | not)
      | (if $code == "" then . else select(
          ((.title // "") | contains($code))
          or ((.message // "") | contains($code))
          or (((.tags // []) | index("code-" + $code)) != null)
        ) end)
    ' | head -n 1)

    if [[ -n $msg ]]; then
      if ((json)); then
        printf '%s\n' "$msg"
      else
        printf '%s\n' "$msg" | jq -r '
          (if (.title // "") == "" then "" else "\(.title)\n" end)
          + (.message // "")'
      fi
      return 0
    fi

    (($(date +%s) >= deadline)) && break
    sleep 2
  done

  printf 'agent-msg: no reply within %ss\n' "$timeout" >&2
  return 1
}

command=${1:-help}
shift || true

case $command in
  send) cmd_send "$@" ;;
  inbox) cmd_inbox "$@" ;;
  wait) cmd_wait "$@" ;;
  help | -h | --help) usage ;;
  *)
    printf 'agent-msg: unknown command: %s\n\n' "$command" >&2
    usage >&2
    exit 2
    ;;
esac
