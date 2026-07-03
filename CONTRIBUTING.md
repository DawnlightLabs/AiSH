# Contributing to AiSH

Thanks for helping improve AiSH.

AiSH is a Dawnlight Labs pilot project focused on a provider-shell experience for terminal AI. Contributions should protect the core design goals: local-first behavior where possible, clear approval gates, predictable shell behavior, and cross-platform reliability.

## Ways to contribute

- Report bugs with clear reproduction steps.
- Propose small, focused features that improve the shell workflow.
- Improve install, release, and platform documentation.
- Add tests or manual verification notes for Windows, macOS, and Linux.
- Review security-sensitive behavior around command execution, shell profiles, installers, and model paths.

## Development setup

```bash
git clone https://github.com/amaansyed27/aish.git
cd aish
cargo check --workspace
npm install
npm run site:build
```

Build the provider shell:

```bash
cargo build --release -p aish-provider-shell
```

## Branches and commits

Use short, descriptive branch names:

```text
fix/windows-terminal-profile
feat/headless-installer-flags
docs/release-checklist
```

Write commits in a direct style:

```text
Fix Windows profile detection
Add release checklist
Update installer metadata
```

## Pull request checklist

Before opening a pull request:

- Run `cargo check --workspace`.
- Run relevant package or site checks, including `npm run site:build` when touching the website.
- Confirm no secrets, tokens, model binaries, generated installers, or local paths are committed.
- Add screenshots for visual changes.
- Document manual tests for affected platforms.
- Keep the PR focused; split unrelated changes.

## Code standards

- Prefer small modules over large files.
- Keep platform-specific logic isolated.
- Make destructive or system-impacting command paths explicit and reviewable.
- Do not bypass approval gates.
- Treat shell profile editing, PATH changes, installers, and release scripts as security-sensitive code.

## Documentation standards

Update docs when changing install behavior, command flags, release outputs, configuration, or platform support.

Relevant docs live in:

- `README.md`
- `docs/`
- `.github/ISSUE_TEMPLATE/`
- `SECURITY.md`
- `SUPPORT.md`

## Reporting security issues

Do not open public issues for vulnerabilities. Follow `SECURITY.md` instead.
