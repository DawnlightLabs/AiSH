# AiSH release checklist

## Before stable release

- [ ] Normalize all package, Cargo, Tauri, installer, and release versions to the intended release tag.
- [ ] Verify Windows desktop launch does not show a blank page when the backend is unavailable or a Tauri invoke fails.
- [ ] Verify provider shell launch path from the installer and document the exact executable name.
- [ ] Confirm local command logging defaults and copy in desktop settings and provider shell setup.
- [ ] Add an explicit opt-in flow before any crash log or command log upload to Dawnlight Labs.
- [ ] Add a privacy notice explaining what is collected, where it is stored, and how to delete it.
- [ ] Add a local export/delete logs action in both desktop settings and provider shell slash commands.
- [ ] Ensure no logs are uploaded silently. Local-only must remain the default until the opt-in flow ships.

## Logging policy notes

Current implementation stores command logs locally only. The saved crash-log sharing preference is not an upload mechanism. It is a placeholder preference for a later release where the user can explicitly opt in before anything is sent to Dawnlight Labs.
