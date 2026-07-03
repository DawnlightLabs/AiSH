# Release Checklist

Use this checklist before publishing AiSH release artifacts.

## Pre-release

- Confirm the default branch is green.
- Run `cargo check --workspace`.
- Build the provider shell with `cargo build --release -p aish-provider-shell`.
- Build the website with `npm run site:build`.
- Confirm install instructions in `README.md` and the website match the release outputs.
- Check that release workflows use a supported Node.js version.
- Confirm no deprecated or unavailable runner target is required.
- Confirm artifacts do not include secrets, local paths, model binaries, or debug caches.

## Platform artifacts

### Windows

- MSI or setup bundle.
- Provider shell executable.
- PATH/profile setup behavior tested.
- Windows Terminal and VS Code-compatible profile behavior tested when relevant.

### macOS

- Apple Silicon build when supported.
- Intel x64 build only when a valid runner or cross-build path is available.
- DMG/pkg/tarball behavior tested when implemented.
- Shell profile setup tested for zsh.

### Linux

- `.deb` where supported.
- `.rpm` where supported.
- AppImage or tarball where supported.
- Shell profile setup tested for bash/zsh where implemented.

## Release notes

Include:

- Summary of user-facing changes.
- Install or upgrade instructions.
- Known limitations.
- Breaking changes.
- Checksums when generated.

## Post-release

- Verify download links.
- Verify install scripts point to the intended version.
- Smoke test a clean install.
- Update docs if artifact names or install commands changed.
