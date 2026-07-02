#!/usr/bin/env sh
set -eu

find_provider() {
  for candidate in \
    "/usr/bin/aish" \
    "/usr/local/bin/aish" \
    "/opt/AiSH/aish" \
    "/usr/lib/aish/aish" \
    "/usr/lib/AiSH/aish" \
    "/usr/share/aish/aish"
  do
    if [ -x "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  if command -v aish >/dev/null 2>&1; then
    command -v aish
    return 0
  fi

  return 1
}

if provider="$(find_provider)"; then
  "$provider" --setup-non-interactive --add-path --set-model-path --editor-profiles --model-check || true
else
  echo "AiSH provider setup skipped: provider binary not found" >&2
fi
