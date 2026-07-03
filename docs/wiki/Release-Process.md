# Release Process

AiSH releases should be reproducible, platform-aware, and easy to verify.

## Before cutting a release

- Confirm CI passes.
- Run Rust workspace checks.
- Build the provider shell.
- Build the website.
- Confirm install scripts and release notes are in sync.
- Confirm workflows use a supported Node.js runtime.
- Confirm unavailable runner targets are not required.

## Artifact expectations

Release artifacts may include:

- Windows installer bundles.
- macOS artifacts where runner support exists.
- Linux packages or tarballs.
- Checksums.
- Install scripts.

## macOS x64 note

Do not block releases on an unavailable x64 macOS runner. Either provide a valid build path or mark that artifact as unsupported for the release.

## After publishing

- Verify links.
- Smoke test clean install.
- Check that website/download instructions point to the new release.
- Triage installer issues first.
