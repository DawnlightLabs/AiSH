# AiSH

AiSH is a minimalist intelligent terminal system with two surfaces:

```text
1. Standalone desktop terminal app
2. Shell/provider layer for native terminals
```

The app is the primary product. The provider layer lets AiSH features appear inside existing shells when possible.

AiSH should feel closer to a clean Warp-style terminal with a PowerShell-native command experience than to a chatbot CLI. It should not interrupt normal terminal use. It should offer completion, context, and AI only when useful.

## Product Shape

```text
AiSH = terminal app + shell/provider layer + local context engine + optional local AI
```

### 1. Standalone App

The standalone app owns the full terminal interface:

```text
- terminal tabs and panes
- command input
- ghost suggestions
- dropdown suggestions
- command cards
- mode switching
- context controls
- cache controls
- safety prompts
- AI panel / inline AI suggestions
```

The app runs real shells through a PTY backend. Commands execute in PowerShell, cmd, Git Bash, Bash, Zsh, Fish, or other configured shells.

### 2. Shell / Provider Layer

The provider layer integrates AiSH into existing terminal environments:

```text
- PowerShell provider / module
- Zsh integration
- Bash integration
- Fish integration
- cmd shim where possible
```

Provider integrations should be thinner than the app. Their job is to expose completions, context collection, history capture, and explicit AI actions without trying to replace the native shell UI.

## Design Direction

Visual direction:

```text
- minimalist
- dark-first
- calm contrast
- clean blocks like Warp
- PowerShell-friendly command language
- low visual noise
- fast keyboard-first interactions
```

The terminal should still feel like a terminal. AiSH UI elements should sit around the command line, not turn the terminal into a chat app.

## Modes

AiSH has three primary modes.

### 1. Normal Mode

Plain shell behavior.

```text
- no ghost text
- no AI
- no ranking
- no extra suggestions unless manually opened
```

Use this when trust and predictability matter.

### 2. History Mode

Local suggestions from user behavior and project context.

```text
Inputs:
- typed prefix
- shell history
- cwd
- project type
- package scripts
- git branches
- recent successful commands
- frequency and recency
```

This is the default mode for the first stable release.

### 3. AI Mode

AI-assisted command help. AI Mode has two submodes.

#### AI Suggest

Inline or dropdown command suggestions.

```text
User intent -> command card -> safety check -> suggestion
```

The model may generate a command, plan, fallback, or explanation card. It never directly executes commands.

#### AI Ask

A side panel / command palette flow for longer help.

```text
- explain a command
- suggest alternatives
- debug an error
- turn natural language into a command
- summarize project context
```

AI Ask is explicit. It should not run on every keystroke.

## Context Controls

AiSH must make context visible and controllable.

```text
Context toggle: on/off
Context scope:
  - none
  - cwd only
  - project files
  - git state
  - package metadata
  - recent terminal output
  - selected text only
```

The user should be able to see what context is being used before AI runs.

## Cache Controls

AiSH should cache local data for speed, but expose controls.

```text
Cache types:
- command history index
- project detection cache
- package script cache
- git branch cache
- AI response cache
- model metadata cache
```

Cache rules:

```text
- local-first
- clearable
- scoped per project/user
- no hidden cloud sync
- no model call on every keystroke
```

## Safety

AiSH must never silently execute risky suggestions.

High-risk examples:

```text
rm -rf
del /s /q
git reset --hard
docker system prune
kubectl delete
npm publish
terraform apply
format
chmod -R 777
```

Risky actions require confirmation and a clear explanation.

## Target Platforms

```text
Windows:
  - standalone app with PowerShell first
  - cmd support
  - Git Bash support
  - PowerShell provider/module

macOS:
  - standalone app
  - Zsh provider
  - Bash provider

Linux:
  - standalone app
  - Bash provider
  - Zsh provider
  - Fish provider
```

## Recommended Repo Shape

```text
aish/
├── apps/
│   └── desktop/              # Tauri + React app
├── crates/
│   ├── aish-core/            # shared types, mode router, config
│   ├── aish-pty/             # PTY and shell sessions
│   ├── aish-context/         # cwd/project/git/package context
│   ├── aish-history/         # SQLite history and events
│   ├── aish-completion/      # deterministic candidates and ranker hooks
│   ├── aish-ai/              # local model integration and command cards
│   ├── aish-safety/          # deterministic risk classifier
│   └── aish-provider/        # provider protocol shared by shells
├── providers/
│   ├── powershell/
│   ├── bash/
│   ├── zsh/
│   ├── fish/
│   └── cmd/
├── models/
├── docs/
└── README.md
```

## First Build Target

Build in this order:

```text
1. Desktop app shell with terminal view
2. Windows PowerShell PTY backend
3. Normal / History / AI mode switcher
4. Local history store
5. Deterministic completion engine
6. Provider protocol
7. PowerShell provider
8. AI command-card integration
9. Model cache and context controls
```

Ken is useful, but AiSH should not wait for Ken to be perfect. The product should work first with deterministic history/project completions, then plug in local AI as an optional layer.
