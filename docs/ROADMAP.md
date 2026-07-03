# Roadmap

This roadmap is directional and can change as AiSH moves toward stable releases.

## Now

- Provider shell as the active product surface.
- Installer and profile setup for supported shells and terminals.
- Website and downloads page for release distribution.
- Command planning with approval gates.
- Local model path support.

## Next

- Harden release automation across Windows, macOS, and Linux.
- Improve installer rollback and repair behavior.
- Expand terminal profile support.
- Improve command trace readability.
- Add more deterministic CLI knowledge before relying on model output.
- Improve docs for clean install, upgrade, and uninstall flows.

## Later

- More local model options.
- Better ranking for command suggestions.
- Richer project context detection.
- More package formats and installation channels.
- Optional signed artifacts and stronger supply-chain checks.

## Non-goals for the current main branch

- Rebuilding the old Tauri desktop app as the main product surface.
- Running destructive commands without explicit approval.
- Bundling large model binaries directly into the repository.
