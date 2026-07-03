# AiSH security and system design review

## Current system shape

AiSH is now a provider-shell-first CLI distribution. The main runtime is `apps/provider-shell`, backed by shared crates for provider planning, safety checks, context, logging, AI prompt/runtime handling, completion, and core card types.

## Strengths

- Desktop/Tauri build path has been removed from mainline.
- Provider shell is the single runtime surface.
- Command risk classification exists before execution.
- Local log settings are explicit and stored locally.
- Release workflow builds platform-specific artifacts.
- Install scripts are static site assets and can be reviewed.

## High-priority risks and mitigations

### 1. Installer integrity

Risk: shell install scripts download release assets over HTTPS but historically did not verify artifact checksums.

Mitigation in this cleanup:
- Release workflow now emits `.sha256` files for each binary archive.
- Install scripts attempt checksum verification when checksum files exist.

Remaining hardening:
- Add signature verification later, e.g. minisign/cosign.
- Publish a manual install path in docs for users who avoid pipe-to-shell installs.

### 2. Windows PATH mutation

Risk: `setx PATH` can truncate or accidentally rewrite the user PATH with expanded process PATH content.

Mitigation in this cleanup:
- Replaced direct `setx PATH` usage with PowerShell user environment persistence via `[Environment]::SetEnvironmentVariable`.

Remaining hardening:
- Add backup/restore for previous user PATH.
- Add `aish --doctor` PATH diagnostics.

### 3. Windows Terminal profile GUID

Risk: the previous profile GUID contained non-hex text and was invalid.

Mitigation in this cleanup:
- Replaced with a valid GUID.
- Added unpackaged Windows Terminal settings path.

### 4. Model supply chain

Risk: `AISH_MODEL_URL` can point to arbitrary GGUF content. Current validation checks size and GGUF magic, but not model hash or signer.

Remaining hardening:
- Add optional `AISH_MODEL_SHA256` verification.
- Pin default model checksum in release notes/config.
- Consider making model download opt-in with a clear source prompt.

### 5. Shell command execution

Risk: AiSH intentionally executes shell commands. The highest security requirement is preventing accidental destructive execution.

Existing controls:
- Risk levels.
- Approval-required actions.
- Prompt rules for shell-specific command generation.

Remaining hardening:
- Add a pre-execution command normalizer per shell.
- Add denylist tests for PowerShell/cmd syntax confusion.
- Add property tests around destructive alias detection.

## System design recommendations

1. Keep provider shell as the only mainline runtime.
2. Split install/setup logic into smaller modules next:
   - `install/args.rs`
   - `install/model.rs`
   - `install/path.rs`
   - `install/windows_terminal.rs`
   - `install/editors.rs`
3. Add `aish --doctor` to inspect PATH, model path, terminal profile, editor profiles, logging, and release version.
4. Use Spec Kit style docs for feature work:
   - `spec.md` for intent and acceptance criteria.
   - `plan.md` for architecture.
   - `tasks.md` for implementation steps.
5. Keep public site assets under `apps/site/public`, and source theme files under `apps/site/src/styles`.

## Final release checklist

- `cargo fmt`
- `cargo check --workspace`
- `cargo build --release -p aish-provider-shell`
- `npm install`
- `npm run site:build`
- Verify latest release workflow with `v0.2.0`
- Verify install commands after release:
  - Windows: PowerShell install
  - macOS/Linux: curl install
