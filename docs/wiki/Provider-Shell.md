# Provider Shell

The provider shell is the active AiSH product surface on `main`.

## What it does

AiSH helps users turn terminal intent into shell-aware command plans while keeping execution inside real terminal workflows.

Core behavior:

- Understand user intent.
- Inspect relevant shell and project context.
- Produce command cards or plan cards.
- Classify risk.
- Ask for approval before destructive or system-impacting actions.
- Execute through the platform shell where appropriate.

## Modes

AiSH may support different usage modes over time:

- normal shell flow
- history-assisted completion
- AI-generated command planning

The exact UI and keybindings should be documented as they stabilize.

## Safety expectation

The provider shell must not silently edit shell profiles, change PATH, delete files, run installers, or execute destructive commands without clear user approval.

## Platform notes

### Windows

Primary targets include PowerShell, Windows Terminal, and VS Code-compatible terminal profiles.

### macOS

Primary target is zsh, with Terminal.app and compatible terminals as install targets where supported.

### Linux

Primary targets include bash and zsh, with package support depending on release artifacts.
