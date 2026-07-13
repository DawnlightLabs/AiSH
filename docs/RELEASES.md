# Release Checklist

Use this checklist before publishing AiSH release artifacts.

## Version bump workflow

Use the `Version Bump` workflow for normal releases.

Inputs:

- `version`: semantic version without a leading `v`, for example `0.3.0`.
- `publish_release`: when `true`, the workflow pushes `vVERSION` after committing the version bump.

The version bump updates `apps/provider-shell/Cargo.toml`, refreshes `Cargo.lock`, commits the bump to `main`, and optionally pushes the release tag. Pushing the tag triggers `Provider Release`, which publishes the platform artifacts.

The installed provider shell uses its compiled package version for `/version`, `/status`, startup update prompts, and `/update` comparisons. Always bump the package version before publishing a release users should receive.

## Runtime update flow

AiSH checks GitHub Releases at most once every 24 hours when the provider shell starts. If a newer stable release exists, it shows the installed and available versions and asks before downloading or replacing anything.

Manual checks remain available:

```text
/update
```

```bash
aish --update
```

Non-interactive update installation is available with:

```bash
aish --update --yes
```

On Windows the replacement process starts after the active shell exits, then refreshes Start menu and Installed apps registration using the new binary.

## Uninstall flow

Interactive uninstall:

```bash
aish --uninstall
```

Unattended uninstall:

```bash
aish --uninstall --yes
```

Windows uninstall must remove the user PATH entry, Start menu shortcut, Installed apps entry, App Paths entry, Windows Terminal profile, supported editor profiles, and installed files. Models stored outside the AiSH install directory are preserved.

## Pre-release

- Confirm the default branch is green.
- Run `cargo check --workspace` on Linux and Windows.
- Parse all shipped PowerShell scripts before release.
- Build the provider shell with `cargo build --release -p aish-provider-shell`.
- Build the website with `npm run site:build`.
- Confirm install instructions in `README.md` and the website match the release outputs.
- Check that release workflows use a supported Node.js version.
- Confirm no deprecated or unavailable runner target is required.
- Confirm artifacts do not include secrets, local paths, model binaries, or debug caches.

## Platform artifacts

### Windows

- PowerShell installer and provider shell executable.
- Start menu shortcut and user-level App Paths registration.
- Installed apps entry with publisher, version, icon, and uninstall command.
- Automatic registration repair after updates.
- PATH/profile setup and complete uninstall behavior tested.
- Windows Terminal and VS Code-compatible profile behavior tested when relevant.
- MSI or setup bundle when implemented.

### macOS

- Apple Silicon build when supported.
- Intel x64 build only when a valid runner or cross-build path is available.
- DMG/pkg/tarball behavior tested when implemented.
- Shell profile setup and built-in uninstall tested for zsh.

### Linux

- `.deb` where supported.
- `.rpm` where supported.
- AppImage or tarball where supported.
- Shell profile setup and built-in uninstall tested for bash/zsh where implemented.
- Linux x64 release artifacts should be built on Ubuntu 22.04 to keep the glibc baseline compatible with Ubuntu 22.04 LTS and newer distributions.

## Release notes

Add curated notes at `docs/releases/vVERSION.md` before publishing. `Provider Release` uses that file as the GitHub release body. The downloads page reads the same body from the GitHub Releases API and displays it as the public changelog.

Include:

- Summary of user-facing changes.
- Install or upgrade instructions.
- Known limitations.
- Breaking changes.
- Checksums when generated.

## Post-release

- Verify download links and checksums.
- Verify install scripts point to the intended version.
- Smoke test a clean Windows install and confirm AiSH appears in Start menu search and Installed apps.
- Upgrade an older Windows build with `/update`, restart, and confirm app registration repairs automatically.
- Launch an older build after a newer release exists and confirm the automatic update prompt appears.
- Test `aish --uninstall` and Windows Settings uninstall; confirm PATH, profiles, registration, and installed files are removed.
- Confirm the downloads-page changelog matches the curated release notes.
- Update docs if artifact names or install commands changed.
