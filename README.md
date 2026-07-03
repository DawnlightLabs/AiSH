<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="apps/site/public/brand/aish-full-horizontal-white.svg">
    <img alt="AiSH" src="apps/site/public/brand/aish-full-horizontal-graphite.svg" width="360">
  </picture>
</p>

# AiSH

AiSH is an AI-native provider shell from Dawnlight Labs.

It turns natural-language intent into shell-aware command plans, keeps execution inside real terminals, and uses approval gates before destructive or system-impacting actions.

## Install

Windows PowerShell:

```powershell
irm https://aish.dawnlightlabs.com/install.ps1 | iex
```

macOS / Linux:

```bash
curl -fsSL https://aish.dawnlightlabs.com/install | bash
```

Backup downloads are published on GitHub Releases.

## Current shape

```text
AiSH = provider shell + context engine + CLI knowledge layer + optional local Ken model
```

The old Tauri desktop app has been archived on the `app-provider-archive` branch. Main now ships the provider shell only.

## Setup

```bash
aish --install
```

Headless install:

```bash
aish --install-headless --add-path --set-model-path --editor-profiles --model-check
```

Legacy setup remains available:

```bash
aish --setup
```

## Features

- AI Run mode for shell-aware command planning
- Local-first model path support
- Read-only commands can run quickly
- Destructive and system-impacting commands require approval
- Windows Terminal and VS Code-compatible terminal profile setup
- macOS/Linux shell profile setup

## Development

```bash
cargo check --workspace
cargo build --release -p aish-provider-shell
npm install
npm run site:build
```

## Brand

AiSH is a Dawnlight Labs pilot project.
