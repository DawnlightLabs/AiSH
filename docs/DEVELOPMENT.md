# Development Guide

This guide covers the basic local workflow for AiSH contributors.

## Requirements

- Rust toolchain.
- Node.js for the website and supporting scripts.
- npm.
- Git.
- Platform shell for manual testing: PowerShell on Windows, zsh/bash on macOS or Linux.

Use the active Node.js LTS line rather than deprecated runtime versions in new automation.

## Clone

```bash
git clone https://github.com/amaansyed27/aish.git
cd aish
```

## Check the Rust workspace

```bash
cargo check --workspace
```

## Build the provider shell

```bash
cargo build --release -p aish-provider-shell
```

## Build the site

```bash
npm install
npm run site:build
```

## Manual testing

When changing shell behavior, test at least one real terminal flow.

### Windows

- PowerShell.
- Windows Terminal profile setup.
- VS Code-compatible terminal profile setup when relevant.
- PATH changes and rollback behavior.

### macOS

- zsh profile setup.
- Terminal.app or the user's preferred terminal.
- Release package behavior when relevant.

### Linux

- bash and zsh profile setup where supported.
- `.deb`, `.rpm`, AppImage, or tarball install paths when relevant.

## Safety-sensitive changes

Require extra review for:

- generated command execution
- destructive command classification
- shell profile edits
- PATH edits
- installer scripts
- release workflows
- model loading paths

## Large-file policy

Do not commit:

- model binaries
- generated installers
- release archives
- local caches
- logs containing private data
- `.env` files or secrets

Use GitHub Releases for distribution artifacts.
