#!/usr/bin/env bash
set -euo pipefail

if [[ "$(id -u)" == "0" ]]; then
  mkdir -p /models /home/aish
  chown -R aish:aish /models /home/aish
  exec gosu aish "$0" "$@"
fi

export DISPLAY="${DISPLAY:-:99}"

display_number="${DISPLAY#:}"
display_number="${display_number%%.*}"
rm -f "/tmp/.X${display_number}-lock" "/tmp/.X11-unix/X${display_number}"

Xvfb "$DISPLAY" -screen 0 1440x900x24 -ac -nolisten tcp &
for _ in $(seq 1 50); do
  [[ -S "/tmp/.X11-unix/X${display_number}" ]] && break
  sleep 0.1
done

openbox --sm-disable &
x11vnc \
  -display "$DISPLAY" \
  -forever \
  -shared \
  -nopw \
  -listen 0.0.0.0 \
  -rfbport 5900 &
websockify --web=/usr/share/novnc 6080 localhost:5900 &

exec dbus-run-session -- aish-desktop
