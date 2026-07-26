use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const RETRY_COUNT: usize = 120;
const RETRY_DELAY_MS: u64 = 250;
const PENDING_MAX_AGE_SECS: u64 = 10 * 60;

#[cfg(windows)]
const DETACHED_PROCESS_FLAGS: u32 = 0x08000008;

pub(super) fn handle_apply_args(args: &[String], current_version: &str) -> bool {
    let Some(target) = arg_value(args, "--apply-update") else {
        return false;
    };

    let expected_version =
        arg_value(args, "--expected-version").unwrap_or_else(|| current_version.to_string());
    let runtime_source = arg_value(args, "--apply-runtime-from").map(PathBuf::from);
    let runtime_target = arg_value(args, "--apply-runtime-to").map(PathBuf::from);
    let result = apply_windows_update(
        Path::new(&target),
        &expected_version,
        runtime_source.as_deref(),
        runtime_target.as_deref(),
    );
    clear_pending_update();

    let exit_code = match result {
        Ok(()) => {
            let message = format!("AiSH updated successfully to {expected_version}.");
            let _ = write_result(&success_path(), &message);
            0
        }
        Err(error) => {
            let message = format!("AiSH update to {expected_version} failed: {error}");
            let _ = write_result(&error_path(), &message);
            1
        }
    };

    std::process::exit(exit_code);
}

#[cfg(windows)]
pub(super) fn start_windows_replace(
    replacement: &Path,
    current: &Path,
    runtime_source: &Path,
    runtime_target: &Path,
    expected_version: &str,
) -> Result<(), String> {
    write_pending_update(expected_version)?;

    let mut command = Command::new(replacement);
    command
        .arg("--apply-update")
        .arg(current)
        .arg("--apply-runtime-from")
        .arg(runtime_source)
        .arg("--apply-runtime-to")
        .arg(runtime_target)
        .arg("--expected-version")
        .arg(expected_version)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.creation_flags(DETACHED_PROCESS_FLAGS);

    if let Err(error) = command.spawn() {
        clear_pending_update();
        return Err(format!(
            "failed to start Windows replacement helper: {error}"
        ));
    }

    Ok(())
}

#[cfg(not(windows))]
pub(super) fn start_windows_replace(
    _replacement: &Path,
    _current: &Path,
    _runtime_source: &Path,
    _runtime_target: &Path,
    _expected_version: &str,
) -> Result<(), String> {
    Err("Windows replacement helper is only available on Windows".to_string())
}

#[cfg(windows)]
fn apply_windows_update(
    target: &Path,
    expected_version: &str,
    runtime_source: Option<&Path>,
    runtime_target: Option<&Path>,
) -> Result<(), String> {
    let source =
        env::current_exe().map_err(|error| format!("could not locate update helper: {error}"))?;
    let mut copied = false;
    let mut last_error = String::new();

    for _ in 0..RETRY_COUNT {
        match fs::copy(&source, target) {
            Ok(_) => {
                copied = true;
                break;
            }
            Err(error) => {
                last_error = error.to_string();
                thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
            }
        }
    }

    if !copied {
        return Err(format!(
            "could not replace {} after {} attempts: {}",
            target.display(),
            RETRY_COUNT,
            last_error
        ));
    }

    let version_output = Command::new(target)
        .arg("--version")
        .env("AISH_SKIP_UPDATE_CHECK", "1")
        .output()
        .map_err(|error| format!("could not verify updated executable: {error}"))?;
    let version_text = String::from_utf8_lossy(&version_output.stdout);
    let installed_version = version_text.split_whitespace().last().unwrap_or_default();
    if !version_output.status.success()
        || normalize_version(installed_version) != normalize_version(expected_version)
    {
        return Err(format!(
            "replacement verification failed; expected {}, received {}",
            expected_version,
            version_text.trim()
        ));
    }

    match (runtime_source, runtime_target) {
        (Some(source), Some(target)) => replace_runtime_directory(source, target)?,
        (None, None) => {}
        _ => {
            return Err(
                "runtime update requires both source and target directory arguments".to_string(),
            )
        }
    }

    let repair_status = Command::new(target)
        .args(["--repair-install", "--quiet"])
        .env("AISH_SKIP_UPDATE_CHECK", "1")
        .status()
        .map_err(|error| format!("could not run installation repair: {error}"))?;
    if !repair_status.success() {
        return Err(format!(
            "updated executable was installed, but registration repair exited with {}",
            repair_status
        ));
    }

    Ok(())
}

#[cfg(not(windows))]
fn apply_windows_update(
    _target: &Path,
    _expected_version: &str,
    _runtime_source: Option<&Path>,
    _runtime_target: Option<&Path>,
) -> Result<(), String> {
    Err("Windows update application is only available on Windows".to_string())
}

pub(super) fn replace_runtime_directory(source: &Path, target: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!(
            "updated runtime directory is missing: {}",
            source.display()
        ));
    }
    let parent = target
        .parent()
        .ok_or_else(|| "runtime target has no parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let staged = parent.join("runtime.new");
    let backup = parent.join("runtime.old");
    if staged.exists() {
        fs::remove_dir_all(&staged).map_err(|error| error.to_string())?;
    }
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|error| error.to_string())?;
    }
    copy_directory(source, &staged)?;
    if target.exists() {
        fs::rename(target, &backup).map_err(|error| {
            format!("failed to stage existing runtime for replacement: {error}")
        })?;
    }
    if let Err(error) = fs::rename(&staged, target) {
        if backup.exists() {
            let _ = fs::rename(&backup, target);
        }
        return Err(format!("failed to activate updated runtime: {error}"));
    }
    if backup.exists() {
        fs::remove_dir_all(backup).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn copy_directory(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub(super) fn active_pending_update() -> Option<String> {
    if env::consts::OS != "windows" {
        return None;
    }

    let path = pending_path();
    let text = fs::read_to_string(&path).ok()?;
    let mut lines = text.lines();
    let version = lines.next()?.trim().to_string();
    let created = lines.next()?.trim().parse::<u64>().ok()?;

    if now_unix_secs().saturating_sub(created) > PENDING_MAX_AGE_SECS {
        let _ = fs::remove_file(path);
        return None;
    }

    Some(version)
}

pub(super) fn show_result_once() {
    if env::consts::OS != "windows" {
        return;
    }

    let success = success_path();
    if let Ok(message) = fs::read_to_string(&success) {
        println!("{}", message.trim());
        let _ = fs::remove_file(success);
    }

    let error = error_path();
    if let Ok(message) = fs::read_to_string(&error) {
        eprintln!("{}", message.trim());
        eprintln!("Run the latest installer if automatic replacement cannot complete.");
        let _ = fs::remove_file(error);
    }
}

fn write_pending_update(expected_version: &str) -> Result<(), String> {
    fs::create_dir_all(state_dir()).map_err(|error| error.to_string())?;
    fs::write(
        pending_path(),
        format!(
            "{}\n{}",
            normalize_version(expected_version),
            now_unix_secs()
        ),
    )
    .map_err(|error| error.to_string())
}

fn clear_pending_update() {
    let _ = fs::remove_file(pending_path());
}

fn write_result(path: &Path, message: &str) -> Result<(), String> {
    fs::create_dir_all(state_dir()).map_err(|error| error.to_string())?;
    fs::write(path, message).map_err(|error| error.to_string())
}

fn state_dir() -> PathBuf {
    windows_install_root().join("state")
}

fn pending_path() -> PathBuf {
    state_dir().join("update-pending")
}

fn success_path() -> PathBuf {
    state_dir().join("update-success")
}

fn error_path() -> PathBuf {
    state_dir().join("update-error")
}

fn windows_install_root() -> PathBuf {
    env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir())
        .join("AiSH")
}

fn home_dir() -> PathBuf {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn normalize_version(version: &str) -> String {
    version
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
}

#[cfg(test)]
mod tests {
    use super::{normalize_version, replace_runtime_directory};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn normalizes_release_versions() {
        assert_eq!(normalize_version("v0.4.3"), "0.4.3");
        assert_eq!(normalize_version(" 0.4.3 "), "0.4.3");
    }

    #[test]
    fn replaces_the_complete_runtime_directory() {
        let root = std::env::temp_dir().join(format!(
            "aish-runtime-update-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = root.join("source");
        let target = root.join("installed").join("runtime");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(source.join("llama-cli.fixture"), "new").unwrap();
        fs::write(source.join("required-library.fixture"), "new").unwrap();
        fs::write(target.join("stale-library.fixture"), "old").unwrap();

        replace_runtime_directory(&source, &target).unwrap();

        assert!(target.join("llama-cli.fixture").is_file());
        assert!(target.join("required-library.fixture").is_file());
        assert!(!target.join("stale-library.fixture").exists());
        assert!(!root.join("installed").join("runtime.old").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
