# Feature Spec: Provider-only AiSH distribution

## User goal

A user can install AiSH from one shell command on Windows, macOS, or Linux and receive a working provider shell with PATH, model path, shell profile, and editor profile setup.

## Requirements

- Windows install command downloads the matching release zip and runs interactive AiSH install.
- macOS/Linux install command downloads the matching tarball and runs interactive AiSH install.
- Install scripts verify checksums when release checksum files are available.
- Releases publish platform artifacts and matching `.sha256` files.
- The website presents provider-shell-first installation only.
- The archived desktop app remains available on `app-provider-archive`.

## Non-goals

- MSI/DMG/deb/rpm wizard UI in mainline.
- Cloud model execution.
- Silent destructive shell actions.

## Acceptance checks

- `npm run site:build`
- `cargo check --workspace`
- `npm run provider:build`
- GitHub Actions Provider Release can publish `v0.2.0` manually.
