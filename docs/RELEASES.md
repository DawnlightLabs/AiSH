# Release Checklist

Use this checklist before publishing AiSH release artifacts.

## Version bump workflow

Use the `Version Bump` workflow for normal releases.

Inputs:

- `version`: semantic version without a leading `v`, for example `1.2.2`.
- `publish_release`: when `true`, the workflow pushes `vVERSION` after committing the version bump.

The version bump updates `apps/provider-shell/Cargo.toml`, refreshes `Cargo.lock`, commits the bump to `main`, and optionally pushes the release tag. Pushing the tag triggers `Provider Release`, which publishes the platform artifacts.

The installed provider shell uses its compiled package version for `/version`, `/status`, and `/update` comparisons. Always bump the package version before publishing a release that users should receive through `/update`.

## Runtime update flow

Installed users can run:

```text
/update
```

AiSH checks the latest GitHub release, compares it with the installed version, shows the release information, asks for approval, then installs the matching platform artifact.

Non-interactive update check/install is available with:

```bash
aish --update --yes
```

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
- Linux x64 release artifacts should be built on Ubuntu 22.04 to keep the glibc baseline compatible with Ubuntu 22.04 LTS and newer distributions.

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
- Run `/update` from an older installed build and confirm it detects the new release.
- Update docs if artifact names or install commands changed.
