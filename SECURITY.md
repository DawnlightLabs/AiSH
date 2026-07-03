# Security Policy

AiSH is a terminal-facing tool. Security reports are taken seriously because the project handles shell commands, profile setup, installers, PATH changes, model paths, and release artifacts.

## Supported versions

Security support applies to the default branch and the latest published release.

| Version | Supported |
| --- | --- |
| Latest release | Yes |
| `main` | Best effort |
| Older releases | No, unless the issue is critical |

## Reporting a vulnerability

Do not open a public GitHub issue for security vulnerabilities.

Use GitHub's private vulnerability reporting flow if it is enabled on the repository. If it is not enabled, contact a repository maintainer directly and include enough detail to reproduce the issue.

Include:

- Affected platform: Windows, macOS, Linux, or all.
- Affected component: provider shell, installer, shell profile setup, release workflow, website, model path handling, or another area.
- Reproduction steps.
- Expected behavior and actual behavior.
- Impact assessment.
- Logs or screenshots with secrets removed.

## Sensitive areas

Please treat the following as security-sensitive:

- Command planning and execution.
- Approval gates for destructive or system-impacting commands.
- Shell profile editing for PowerShell, Windows Terminal, VS Code, bash, zsh, and related shells.
- Installer scripts and package metadata.
- Release workflow artifacts and signing/notarization paths.
- PATH and environment variable changes.
- Local model file discovery and loading.
- Any telemetry, logging, crash report, or analytics code.

## Disclosure expectations

Give maintainers reasonable time to investigate and patch before public disclosure. Do not publish exploit details, proof-of-concept payloads, or bypass techniques until a fix is available.

## Security design principles

AiSH should default to:

- Explicit user approval before destructive actions.
- Clear command previews.
- Minimal privileges.
- Local-first operation where possible.
- No silent shell profile changes.
- No bundled secrets.
- Reproducible release behavior.
