# AiSH Constitution

## Core principles

1. Provider shell first. Mainline AiSH ships as a native provider shell; desktop wrappers belong on archive or experimental branches.
2. Local-first execution. User commands run in the user's real shell, with no hidden cloud execution path.
3. Approval before mutation. Delete, overwrite, install, deploy, publish, registry, permission, process, service, and cloud/system state changes require explicit approval.
4. Spec before large changes. Major features need a short spec, implementation plan, and task list before code changes.
5. Security by construction. Release artifacts, install scripts, model downloads, shell profile edits, and generated commands must have reviewable safety controls.
6. Reproducible releases. CLI releases must publish checksums and commit lockfiles for application builds.

## Required gates

- `cargo fmt`
- `cargo check --workspace`
- `cargo build --release -p aish-provider-shell`
- `npm run site:build`
- Manual install-script dry run for any release workflow changes

## Spec Kit mapping

AiSH uses a lightweight Spec Kit style:
- constitution: this file
- specs: `specs/<feature>/spec.md`
- plans: `specs/<feature>/plan.md`
- tasks: `specs/<feature>/tasks.md`
