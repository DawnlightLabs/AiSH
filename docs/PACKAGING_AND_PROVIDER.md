# AiSH Provider Shell and Packaging

## Current provider shell shape

AiSH now has a first-pass cross-platform provider shell binary named `aish` from the `aish-provider-shell` package.

It is intentionally one mode only: AI Run.

The user can type natural language directly into the provider prompt. Direct terminal commands still pass through for common shell commands such as `cd`, `dir`, `ls`, `git status`, and `npm -v`.

## Slash commands

Slash commands control AiSH itself and are not sent to the model.

Supported first-pass commands:

```text
/model                 show current model
/model list            list enabled models
/model use <id>        reserved; current build keeps Qwen2.5 Coder only
/status                show OS, shell, model, model path, and llama path
/reasoning on|off      toggle full working trace
/working on|off        alias for reasoning trace
/approve               approve pending risky command
/cancel                cancel pending risky command
/help                  show provider help
/exit                  exit provider shell
//text                 send a literal slash-prefixed line
```

## Enabled model policy

For now the desktop selector and provider shell keep only:

```text
Qwen2.5 Coder 1.5B Instruct Q4_K_M
```

Later settings should support additional local models and providers.

## Runtime configuration

Default Windows local paths match the current development layout:

```text
%USERPROFILE%/Downloads/aish-model/models/Qwen2.5-Coder-1.5B-Instruct-Q4_K_M.gguf
%USERPROFILE%/Downloads/llama.cpp/build/bin/Release/llama-cli.exe
```

Portable overrides:

```text
AISH_MODEL_PATH=/path/to/model.gguf
AISH_LLAMA_CLI=/path/to/llama-cli
AISH_TARGET_OS=windows|macos|linux
AISH_TARGET_SHELL=powershell|pwsh|zsh|bash|fish
```

## Build commands

Provider shell:

```powershell
cargo build --release -p aish-provider-shell
```

Desktop app:

```powershell
npm run desktop:build
```

Both:

```powershell
npm run package:all
```

## Cross-platform packaging

The workflow `.github/workflows/package.yml` builds on:

```text
windows-latest
macos-latest
ubuntu-22.04
```

Artifacts include:

```text
provider shell binary: aish / aish.exe
desktop bundles from Tauri
```

## Provider install targets

Windows Terminal profile should launch the provider shell binary directly once packaged:

```text
aish.exe
```

macOS/Linux terminal profiles should launch:

```text
aish
```

Shell integration wrappers can be thin launchers later. The provider shell itself is the cross-platform product boundary.
