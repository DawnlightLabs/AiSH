# Security Model

AiSH is security-sensitive because it operates in terminals and can help produce commands that affect the user's system.

## Core rule

Generated commands should be visible, classifiable, and approval-gated before execution.

## Risk levels

### Low risk

Examples:

- read-only filesystem inspection
- `git status`
- package script listing
- environment inspection without secrets

### Medium risk

Examples:

- package installation
- file writes
- project configuration edits
- commands that alter a workspace

### High risk

Examples:

- deletion
- privilege escalation
- registry edits
- shell profile edits
- PATH mutation
- remote script execution
- installer execution

## Required controls

- Preview generated commands.
- Explain why a command is needed.
- Require approval for destructive or system-impacting work.
- Avoid silent profile edits.
- Keep logs free of secrets.
- Keep model binaries and generated artifacts out of the repository.

## Reporting vulnerabilities

Follow `SECURITY.md`. Do not open public issues for vulnerabilities.
