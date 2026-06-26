# AiSH Shell / Provider Layer

The provider layer brings AiSH into existing terminals without replacing the terminal UI.

The standalone app gives the full experience. Providers give native-shell access to the same completion, context, history, safety, and AI services.

## Goals

```text
- work inside native shells
- collect local context safely
- provide fast completions
- trigger AI explicitly
- log accepted/rejected suggestions locally
- share one provider protocol across shells
```

## Non-Goals

```text
- do not reimplement every terminal UI feature
- do not force AI on every keystroke
- do not auto-run commands
- do not require the desktop app window to be open if a local daemon exists
```

## Provider Types

### PowerShell Provider / Module

Priority provider.

Capabilities:

```text
- register argument completers
- expose AiSH commands
- capture cwd and typed prefix
- integrate with PSReadLine where possible
- show completions through native PowerShell UI first
```

Example commands:

```powershell
Invoke-AiSHSuggest
Invoke-AiSHAsk
Get-AiSHMode
Set-AiSHMode History
Set-AiSHContext Project
Clear-AiSHCache -Project
```

### Zsh Plugin

Capabilities:

```text
- zle widgets
- completion functions
- prefix capture
- explicit AI trigger binding
```

### Bash Integration

Capabilities:

```text
- programmable completion
- readline bindings where possible
- explicit AI command wrapper
```

### Fish Integration

Capabilities:

```text
- native fish completions
- abbreviations where useful
- explicit AI function
```

### cmd Shim

cmd support is limited. It should provide explicit commands and history capture where possible, not advanced inline UI.

## Local Service

Providers should call a local AiSH service or binary.

```text
provider -> aish local service -> shared core crates -> response
```

Transport options:

```text
MVP:
  stdio command invocation

Later:
  local named pipe / Unix socket

Optional:
  background daemon for lower latency
```

## Request Types

```text
complete
rank
ai_suggest
ai_ask
record_event
get_mode
set_mode
get_context_policy
set_context_policy
clear_cache
```

## Completion Request

```json
{
  "request_type": "complete",
  "surface": "powershell-provider",
  "shell": "powershell",
  "os": "windows",
  "mode": "history",
  "prefix": "npm",
  "cwd": "C:/projects/app",
  "context_level": "project",
  "cache_policy": "use_project_cache"
}
```

## Completion Response

```json
{
  "items": [
    {
      "kind": "command",
      "command": "npm run dev",
      "display": "npm run dev",
      "description": "Start the project dev server",
      "source": "package_scripts",
      "score": 0.92,
      "risk": "low",
      "needs_confirmation": false
    }
  ]
}
```

## AI Suggest Request

```json
{
  "request_type": "ai_suggest",
  "surface": "powershell-provider",
  "shell": "powershell",
  "os": "windows",
  "intent": "find process using port 3000",
  "cwd": "C:/projects/app",
  "context_level": "project",
  "ai_mode": "suggest"
}
```

## Event Logging

Providers should report local events.

```json
{
  "request_type": "record_event",
  "event": {
    "shell": "powershell",
    "os": "windows",
    "cwd_hash": "...",
    "typed_prefix": "npm",
    "command": "npm run dev",
    "source": "history_suggestion",
    "accepted": true,
    "exit_code": 0,
    "duration_ms": 1200
  }
}
```

## Provider MVP

PowerShell first:

```text
1. installable PowerShell module
2. explicit Invoke-AiSHSuggest command
3. native completion candidates for simple prefixes
4. local context packet generation
5. record accepted suggestions
6. mode/context/cache setters
```

After that:

```text
- Zsh provider
- Bash provider
- Fish provider
- cmd shim
```
