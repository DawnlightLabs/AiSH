#!/usr/bin/env bash
set -euo pipefail

if [[ "$(id -u)" == "0" ]]; then
  mkdir -p /models /home/aish
  chown -R aish:aish /models /home/aish
  exec gosu aish "$0" "$@"
fi

exec /usr/local/bin/aish "$@"

