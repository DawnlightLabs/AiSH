# AiSH Architecture

AiSH is an AI-native provider shell from Dawnlight Labs. The current `main` branch focuses on the provider shell, context engine, CLI knowledge layer, approval gates, and optional local Ken model support.

The old Tauri desktop app is archived on the `app-provider-archive` branch and is not the active architecture for `main`.

```text
User intent
  -> AiSH provider shell
  -> context engine
  -> CLI knowledge layer
  -> planner / optional local Ken model
  -> command card
  -> safety classifier
  -> approval gate
  -> real terminal execution
```

## Goals

- Keep users inside real terminal workflows.
- Convert natural-language intent into shell-aware command plans.
- Preserve explicit approval before destructive or system-impacting actions.
- Support Windows, macOS, and Linux.
- Keep local-first operation possible through model-path configuration.
- Keep installers and shell-profile changes readable and reversible.

## Major areas

### Provider shell

The provider shell is the main runtime. It should stay predictable, fast to start, and easy to install globally.

Responsibilities:

- Accept normal shell workflows and AiSH-assisted flows.
- Build command plans from user intent.
- Surface previews and approval prompts.
- Execute through the user's real platform shell where appropriate.
- Support profile setup for Windows Terminal, VS Code-compatible terminals, and Unix shells where implemented.

### Context engine

The context layer provides shell-relevant information without collecting unnecessary user data.

Responsibilities:

- OS and shell detection.
- Current working directory context.
- Project type detection.
- Package manager and script detection.
- Git branch and working-tree context.
- Local model path configuration.
- Installed CLI availability.

### CLI knowledge layer

The CLI knowledge layer gives the planner compact tool context for common developer commands.

Target areas include:

- package managers: npm, pnpm, yarn, bun
- version control: git
- containers: docker, docker compose
- cloud/deploy CLIs: vercel, firebase, netlify, wrangler, AWS, gcloud, az
- language tools: cargo, dotnet, go, Java/Maven/Gradle, Python/pip/uv

### Planner and cards

Generated work should be represented as structured cards before execution.

```json
{
  "action_type": "command",
  "os": "windows",
  "shell": "powershell",
  "command": "git status --short",
  "risk": "low",
  "category": "git",
  "requires_admin": false,
  "modifies_system": false,
  "needs_confirmation": false,
  "reason": "Shows concise Git working tree status."
}
```

For multi-step work, use a plan card with individual step risk classifications.

### Approval and safety layer

No generated candidate should bypass safety classification.

Examples that should require approval:

- File deletion, overwrite, or mass modification.
- Package installs and uninstalls.
- Registry, profile, PATH, or shell configuration edits.
- Network downloads and installer execution.
- Privileged commands.
- Commands that alter system state.

Read-only, low-risk commands can run more quickly when the command card has been validated.

### Command trace

AI Run should preserve an execution trace that can explain what happened.

Trace fields can include:

- detected intent
- context used
- card type
- commands or scripts run
- plan steps
- exit code
- duration
- output excerpts
- safety decision

### Website and downloads

The site explains AiSH, hosts install instructions, and points users to release artifacts.

### Release system

Release automation should produce platform-specific artifacts where supported:

- Windows MSI / setup artifacts.
- macOS packages or app bundles where available.
- Linux packages such as `.deb`, `.rpm`, AppImage, or tarballs where implemented.
- CLI install scripts.

## Design constraints

- Do not silently mutate user shell profiles.
- Do not hide generated commands from users.
- Do not bypass approval gates to improve perceived speed.
- Keep installer scripts readable and reviewable.
- Keep release workflows reproducible.
- Keep platform-specific logic isolated.
