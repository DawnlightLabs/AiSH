# AiSH Build Roadmap

This roadmap builds the product while Ken continues training separately.

## Track A: Standalone App

### A0: Scaffold

```text
- Tauri + React desktop app
- Rust workspace
- xterm.js terminal component
- empty mode/status bar
- app settings skeleton
```

### A1: Real Shell Session

```text
- Windows PowerShell PTY through ConPTY
- input/output stream bridge
- terminal resize
- clean process shutdown
- shell selector foundation
```

### A2: Mode System

```text
- Normal Mode
- History Mode
- AI Mode
- AI Suggest submode
- AI Ask submode
- keyboard shortcuts
```

### A3: Local History

```text
- SQLite command_events table
- command capture
- cwd hash
- shell/os
- source
- accepted/rejected
- exit code
- duration
```

### A4: Deterministic Completion

```text
- prefix matching
- recent commands
- frequent commands
- package.json scripts
- git branches
- docker compose
- python project commands
- cargo commands
```

### A5: Suggestion UI

```text
- ghost text
- dropdown list
- accept/dismiss
- source labels
- risk labels
```

### A6: Context / Cache Controls

```text
- context toggle
- context level selector
- cache toggle
- clear project cache
- clear AI cache
```

### A7: AI Integration

```text
- command-card parser
- local runtime bridge
- Ken/GGUF path setting
- AI Suggest
- AI Ask
- safety gate before display
```

## Track B: Provider Layer

### B0: Protocol

```text
- shared JSON protocol
- complete request/response
- ai_suggest request/response
- record_event request
- mode/context/cache setters
```

### B1: Local CLI / Service

```text
- aish complete
- aish ai suggest
- aish mode get/set
- aish context get/set
- aish cache clear
```

### B2: PowerShell Provider

```text
- PowerShell module skeleton
- Invoke-AiSHSuggest
- Invoke-AiSHAsk
- Set-AiSHMode
- Set-AiSHContext
- Register-ArgumentCompleter integration
```

### B3: Bash/Zsh/Fish Providers

```text
- shell-specific completion wrappers
- explicit AI trigger functions
- event logging hooks where possible
```

## Track C: Shared Intelligence

### C0: Context Engine

```text
- detect package.json
- detect lockfile/package manager
- read package scripts
- detect .git
- read current branch
- detect docker compose
- detect pyproject/requirements
- detect Cargo.toml
```

### C1: Safety Engine

```text
- destructive command patterns
- confirmation policy
- risk labels
- admin requirement hints
```

### C2: Completion Engine

```text
- candidate generation
- simple scoring
- source attribution
- project-aware commands
```

### C3: AI Runtime Bridge

```text
- command-card schema
- local model invocation
- planner fallback
- structural validation
- no semantic autocorrect in safety layer
```

## Immediate Next Milestones

```text
1. Add workspace and app scaffold
2. Add placeholder UI with Warp-like minimalist layout
3. Add Rust crate layout
4. Add PTY spike for PowerShell
5. Add provider protocol types
6. Add PowerShell provider skeleton
```

## Definition of Done For First Usable Build

```text
- app opens
- PowerShell runs inside app
- user can type commands
- output streams correctly
- Normal/History/AI mode state is visible
- command history is stored locally
- deterministic suggestions work for npm/git/docker/python
- risky suggestions are labeled before acceptance
```
