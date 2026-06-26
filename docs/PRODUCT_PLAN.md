# AiSH Product Plan

AiSH has two product surfaces that share one local intelligence layer.

```text
1. Standalone app
2. Shell/provider layer
```

The standalone app is the main experience. The provider layer brings AiSH into existing shells without replacing them.

## Product Principles

```text
- terminal first, not chat first
- local first, no hidden cloud dependency
- fast enough for normal typing
- AI is optional and explicit
- risky commands are never silently accepted
- provider integrations are thin and predictable
- the app can work without a model
```

## Standalone App

The standalone app is a full terminal application.

Responsibilities:

```text
- own terminal UI
- run real shells through PTY
- render ghost text and suggestions
- provide command cards
- manage tabs, panes, and sessions
- collect local context
- store history/events
- run deterministic completion engine
- call local AI only when needed
```

Initial shell priority:

```text
1. Windows PowerShell
2. cmd
3. Git Bash
4. macOS Zsh
5. Linux Bash/Zsh/Fish
```

## Shell / Provider Layer

The provider layer exposes AiSH features inside existing shells.

Responsibilities:

```text
- capture typed prefix when supported
- expose completion candidates
- send cwd/shell/project context to local AiSH service
- record accepted/rejected suggestions
- provide explicit AI action commands
- avoid owning the whole terminal UI
```

Provider priority:

```text
1. PowerShell module/provider
2. Zsh plugin
3. Bash integration
4. Fish integration
5. cmd shim
```

## Modes

### Normal Mode

No automation. AiSH behaves like a plain terminal.

### History Mode

Default useful mode. Uses local history and project context.

Sources:

```text
- typed prefix
- previous successful commands
- current directory
- package.json scripts
- Makefile targets
- Docker Compose files
- Git branches
- pyproject.toml
- Cargo.toml
```

### AI Mode

AI Mode has two submodes.

#### AI Suggest

Small, inline command generation.

```text
input -> context packet -> model/runtime planner -> command card -> safety -> suggestion
```

#### AI Ask

Explicit side-panel or command-palette interaction.

Use cases:

```text
- explain this command
- debug this terminal error
- generate a command from natural language
- suggest command alternatives
- summarize project state
```

## Context Toggle

Context must be visible and controllable.

```text
Context Off:
  AI only sees typed text and selected shell name.

Context Minimal:
  cwd, shell, OS, detected project type.

Context Project:
  package metadata, git state, scripts, docker files, pyproject, cargo metadata.

Context Terminal:
  recent terminal output and last command result.

Context Selected:
  only selected text or manually attached context.
```

## Cache Toggle

Cache improves speed but must be explicit.

Cache entries:

```text
- command history index
- project detection results
- package scripts
- git branches
- provider capabilities
- model metadata
- AI responses for repeated prompts
```

Controls:

```text
- clear project cache
- clear all local cache
- disable AI response cache
- disable history learning
```

## MVP Definition

MVP is not Ken integration. MVP is a usable terminal app.

```text
MVP 0:
  app boots, opens PowerShell, sends input, displays output

MVP 1:
  mode switcher, local history, deterministic suggestions

MVP 2:
  provider protocol and PowerShell provider

MVP 3:
  AI command-card integration with local model/runtime planner
```

## Non-Goals For First Build

```text
- no cloud account system
- no remote sync
- no auto-running AI on every keystroke
- no fully autonomous agent
- no command execution without user acceptance
- no model dependency for basic completions
```
