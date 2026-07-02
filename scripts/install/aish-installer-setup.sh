#!/usr/bin/env sh
set -eu

PROVIDER_PATH="${AISH_PROVIDER_PATH:-}"
INSTALL_APP="${AISH_INSTALL_APP:-1}"
SKIP_MODEL="${AISH_SKIP_MODEL:-0}"
ADD_PATH="${AISH_ADD_PATH:-1}"
SET_MODEL_PATH="${AISH_SET_MODEL_PATH:-1}"
EDITOR_PROFILES="${AISH_EDITOR_PROFILES:-1}"

find_provider() {
  if [ -n "$PROVIDER_PATH" ] && [ -x "$PROVIDER_PATH" ]; then
    printf '%s\n' "$PROVIDER_PATH"
    return 0
  fi

  for candidate in \
    "$(dirname "$0")/aish" \
    "$(dirname "$0")/../aish" \
    "$(dirname "$0")/../Resources/aish" \
    "$HOME/.local/aish/bin/aish" \
    "/usr/local/bin/aish" \
    "/usr/bin/aish"
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

  echo "Could not locate AiSH provider shell binary." >&2
  return 1
}

provider="$(find_provider)"
args="--setup-non-interactive"

if [ "$ADD_PATH" = "1" ]; then
  args="$args --add-path"
fi
if [ "$SET_MODEL_PATH" = "1" ]; then
  args="$args --set-model-path"
fi
if [ "$EDITOR_PROFILES" = "1" ]; then
  args="$args --editor-profiles"
fi
if [ "$SKIP_MODEL" != "1" ]; then
  args="$args --model-check"
fi

# shellcheck disable=SC2086
"$provider" $args

echo "AiSH installer setup complete."
