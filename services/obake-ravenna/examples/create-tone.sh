#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
# SPDX-License-Identifier: MIT
#
# Create a lingering audiotestsrc tone and optionally link it into an RTP
# sink (default: obake-out-1).
#
# Prerequisites on the board:
#   - PipeWire running as the same user
#   - audiotestsrc SPA available (image ships it; if create-node fails with
#     "factory not found", install a spa-libs drop-in):
#       mkdir -p ~/.config/pipewire/pipewire.conf.d
#       cat > ~/.config/pipewire/pipewire.conf.d/10-audiotestsrc.conf <<'EOF'
#       context.spa-libs = {
#         audiotestsrc = audiotestsrc/libspa-audiotestsrc
#       }
#       EOF
#       systemctl --user restart pipewire wireplumber
#
# Do NOT set node.group to the RAVENNA PTP group, and use live=false.
# The PTP support.node.driver owns that group; joining it (or running
# audiotestsrc live against that clock) leaves the tone suspended or
# fails Start with EINVAL. Linking into the RTP sink is enough.
#
# Usage:
#   ./create-tone.sh
#   ./create-tone.sh --no-link
#   SINK=obake-out-1 FREQ=1000 ./create-tone.sh

set -euo pipefail

NAME="${NAME:-tone}"
RATE="${RATE:-48000}"
CHANNELS="${CHANNELS:-2}"
FREQ="${FREQ:-440}"
SINK="${SINK:-obake-out-1}"
LINK=1

for arg in "$@"; do
  case "$arg" in
    --no-link) LINK=0 ;;
    -h|--help)
      sed -n '2,30p' "$0"
      exit 0
      ;;
    *)
      echo "unknown argument: $arg" >&2
      exit 1
      ;;
  esac
done

if ! command -v pw-cli >/dev/null; then
  echo "pw-cli not found" >&2
  exit 1
fi

# Every Node id whose node.name matches (pw-cli prints "id N," not "id: N").
node_ids_by_name() {
  local n="$1"
  pw-cli ls Node 2>/dev/null | sed -n \
    "/id [0-9]/{s/.*id //;s/,.*//;h}; /node.name = \"${n}\"/{g;p}"
}

# Destroy every previous tone (linger leaves orphans if destroy was skipped).
while read -r id; do
  [[ -z "$id" ]] && continue
  echo "destroying existing node id=$id ($NAME)"
  pw-cli destroy "$id" >/dev/null || true
done < <(node_ids_by_name "$NAME")
sleep 0.2

# Use adapter so format/channel convert can negotiate with the RTP sink.
# object.linger keeps the node after this pw-cli process exits.
echo "creating $NAME (${CHANNELS}ch @ ${RATE} Hz, ${FREQ} Hz) -> ${SINK}"
pw-cli create-node adapter "{
  factory.name     = audiotestsrc
  node.name        = \"${NAME}\"
  node.description = \"${NAME}\"
  media.class      = Audio/Source
  object.linger    = true
  audio.rate       = ${RATE}
  audio.channels   = ${CHANNELS}
  node.param.Props = {
    live = false
    frequency = ${FREQ}
  }
}"

if [[ "$LINK" -eq 0 ]]; then
  echo "created $NAME (not linked; use: pw-link ${NAME}:capture_FL ${SINK}:send_1)"
  exit 0
fi

if ! command -v pw-link >/dev/null; then
  echo "pw-link not found; node created but not linked" >&2
  exit 1
fi

# Wait briefly for ports to appear.
for _ in $(seq 1 20); do
  if pw-link -o 2>/dev/null | grep -q "^${NAME}:"; then
    break
  fi
  sleep 0.1
done

echo "linking ${NAME} -> ${SINK}"
# Unique capture_* outs only (ignore monitor_*); sink send_* in order.
mapfile -t outs < <(pw-link -o 2>/dev/null | grep "^${NAME}:capture_" | awk '{print $1}' | awk '!seen[$0]++')
mapfile -t ins < <(pw-link -i 2>/dev/null | grep "^${SINK}:send_" | awk '{print $1}' | awk '!seen[$0]++')

if [[ ${#outs[@]} -eq 0 ]]; then
  echo "no capture output ports on ${NAME}" >&2
  exit 1
fi
if [[ ${#ins[@]} -eq 0 ]]; then
  echo "no send input ports on ${SINK} (is the RTP sink up?)" >&2
  exit 1
fi

n=${#outs[@]}
if [[ ${#ins[@]} -lt $n ]]; then
  n=${#ins[@]}
fi
for ((i = 0; i < n; i++)); do
  pw-link -d "${outs[$i]}" "${ins[$i]}" 2>/dev/null || true
  pw-link -w "${outs[$i]}" "${ins[$i]}"
  echo "  ${outs[$i]} -> ${ins[$i]}"
done

id=$(node_ids_by_name "$NAME" | head -n1 || true)
if [[ -n "${id:-}" ]]; then
  state=$(pw-cli info "$id" 2>/dev/null | awk -F'"' '/state:/{print $2; exit}')
  echo "tone state=${state:-unknown} id=$id"
  if [[ "${state:-}" != "running" ]]; then
    echo "warning: tone is not running (check journalctl --user -u pipewire)" >&2
  fi
fi

echo "done"
