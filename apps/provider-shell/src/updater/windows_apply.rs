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
    let result = apply_windows_update(Path::new(&target), &expected_version);
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
    expected_version: &str,
) -> Result<(), String> {
    write_pending_update(expected_version)?;

    let mut command = Command::new(replacement);
    command
        .arg("--apply-update")
        .arg(current)
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
    _expected_version: &str,
) -> Result<(), String> {
    Err("Windows replacement helper is only available on Windows".to_string())
}

#[cfg(windows)]
fn apply_windows_update(target: &Path, expected_version: &str) -> Result<(), String> {
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
fn apply_windows_update(_target: &Path, _expected_version: &str) -> Result<(), String> {
    Err("Windows update application is only available on Windows".to_string())
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
    use super::normalize_version;

    #[test]
    fn normalizes_release_versions() {
        assert_eq!(normalize_version("v0.4.3"), "0.4.3");
        assert_eq!(normalize_version(" 0.4.3 "), "0.4.3");
    }
}
