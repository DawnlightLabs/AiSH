from pathlib import Path

updater = Path("apps/provider-shell/src/updater.rs")
text = updater.read_text()

text = text.replace("mod lifecycle;\n", "mod lifecycle;\nmod windows_apply;\n", 1)

old_handle_start = '''pub fn handle_update_args() -> bool {
    let args = env::args().collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
'''
new_handle_start = '''pub fn handle_update_args() -> bool {
    let args = env::args().collect::<Vec<_>>();

    if windows_apply::handle_apply_args(&args, current_version()) {
        return true;
    }

    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
'''
if old_handle_start not in text:
    raise SystemExit("handle_update_args start not found")
text = text.replace(old_handle_start, new_handle_start, 1)

old_after_lifecycle = '''    if lifecycle::handle_args(&args, current_version()) {
        return true;
    }

    if args.iter().any(|arg| arg == "--update") {
'''
new_after_lifecycle = '''    if lifecycle::handle_args(&args, current_version()) {
        return true;
    }

    if let Some(version) = windows_apply::active_pending_update() {
        println!(
            "AiSH update to {version} is still being applied. Close this window and reopen AiSH in a few seconds."
        );
        return true;
    }

    windows_apply::show_result_once();

    if args.iter().any(|arg| arg == "--update") {
'''
if old_after_lifecycle not in text:
    raise SystemExit("lifecycle insertion point not found")
text = text.replace(old_after_lifecycle, new_after_lifecycle, 1)

old_call = "        start_windows_replace(&replacement, &current)?;\n"
new_call = '''        windows_apply::start_windows_replace(
            &replacement,
            &current,
            &normalize_version(tag),
        )?;
'''
if old_call not in text:
    raise SystemExit("Windows replacement call not found")
text = text.replace(old_call, new_call, 1)

start = text.index("fn start_windows_replace(")
end = text.index("\nfn update_check_due()", start)
text = text[:start] + text[end + 1 :]

updater.write_text(text)

Path("docs/releases/v0.4.3.md").write_text(
    """# AiSH v0.4.3

AiSH 0.4.3 replaces the fragile Windows `cmd.exe start` updater chain with an internal detached update helper.

## Fixed

- Waits for the running AiSH executable to unlock and retries replacement safely.
- Prevents a newly opened AiSH shell from re-locking the executable while an update is pending.
- Verifies the installed executable version before completing the update.
- Runs Windows app and terminal registration repair after replacement.
- Surfaces update success or failure on the next AiSH launch instead of losing background errors.

Users on 0.4.1 or 0.4.2 should install 0.4.3 once using the direct installer because those versions contain the old replacement process.
"""
)
