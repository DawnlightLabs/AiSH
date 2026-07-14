mod lifecycle;

use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const LATEST_RELEASE_API: &str = "https://api.github.com/repos/DawnlightLabs/AiSH/releases/latest";
const RELEASE_DOWNLOAD_BASE: &str = "https://github.com/DawnlightLabs/AiSH/releases/download";
const UPDATE_CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

#[derive(Debug, Clone)]
struct ReleaseInfo {
    tag_name: String,
    name: Option<String>,
    html_url: Option<String>,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn print_version() {
    println!("AiSH {}", current_version());
}

pub fn handle_update_args() -> bool {
    let args = env::args().collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        print_version();
        return true;
    }

    if lifecycle::handle_args(&args, current_version()) {
        return true;
    }

    if args.iter().any(|arg| arg == "--update") {
        let assume_yes = args.iter().any(|arg| arg == "--yes" || arg == "-y");
        let should_exit = run_update(assume_yes);
        if should_exit {
            std::process::exit(0);
        }
        return true;
    }

    lifecycle::ensure_windows_app_registration(current_version());
    maybe_prompt_for_update()
}

pub fn run_update_flow() -> bool {
    run_update(false)
}

fn maybe_prompt_for_update() -> bool {
    if env::var("AISH_SKIP_UPDATE_CHECK").ok().as_deref() == Some("1") {
        return false;
    }

    if !update_check_due() {
        return false;
    }

    let release = match fetch_latest_release() {
        Ok(release) => {
            record_update_check();
            release
        }
        Err(error) => {
            record_update_check();
            if env::var("AISH_UPDATE_DEBUG").ok().as_deref() == Some("1") {
                eprintln!("automatic update check failed: {error}");
            }
            return false;
        }
    };

    let current = current_version();
    let latest = normalize_version(&release.tag_name);
    if !is_newer_version(&latest, current) {
        return false;
    }

    println!();
    println!(
        "AiSH {} is available. You are running {}.",
        release.tag_name, current
    );
    if let Some(name) = release.name.as_deref() {
        if !name.trim().is_empty() && name.trim() != release.tag_name {
            println!("release: {name}");
        }
    }
    if let Some(url) = release.html_url.as_deref() {
        println!("release notes: {url}");
    }

    if !prompt_yes_no("Install this update now", true) {
        println!("update deferred; AiSH will ask again after the next daily check.");
        return false;
    }

    match install_release(&release.tag_name) {
        Ok(_) => true,
        Err(error) => {
            eprintln!("update failed: {error}");
            false
        }
    }
}

fn run_update(assume_yes: bool) -> bool {
    println!("checking for AiSH updates...");

    let release = match fetch_latest_release() {
        Ok(release) => release,
        Err(error) => {
            eprintln!("update check failed: {error}");
            return false;
        }
    };
    record_update_check();

    let current = current_version();
    let latest = normalize_version(&release.tag_name);

    println!("current version: {current}");
    println!("latest release: {}", release.tag_name);
    if let Some(name) = release.name.as_deref() {
        if !name.trim().is_empty() && name.trim() != release.tag_name {
            println!("release name: {name}");
        }
    }

    if !is_newer_version(&latest, current) {
        println!("AiSH is already on the latest release.");
        return false;
    }

    if let Some(url) = release.html_url.as_deref() {
        println!("release notes: {url}");
    }

    if !assume_yes && !prompt_yes_no("Install this update now", true) {
        println!("update cancelled");
        return false;
    }

    match install_release(&release.tag_name) {
        Ok(should_exit) => should_exit,
        Err(error) => {
            eprintln!("update failed: {error}");
            false
        }
    }
}

fn fetch_latest_release() -> Result<ReleaseInfo, String> {
    let response = ureq::get(LATEST_RELEASE_API)
        .set("User-Agent", concat!("AiSH/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(20))
        .call()
        .map_err(|error| error.to_string())?;

    let text = response.into_string().map_err(|error| error.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;

    let tag_name = json
        .get("tag_name")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "latest release did not include tag_name".to_string())?
        .to_string();

    Ok(ReleaseInfo {
        tag_name,
        name: json
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        html_url: json
            .get("html_url")
            .and_then(|value| value.as_str())
            .map(str::to_string),
    })
}

fn install_release(tag: &str) -> Result<bool, String> {
    let asset = platform_asset()?;
    let url = format!("{RELEASE_DOWNLOAD_BASE}/{tag}/{asset}");
    let work_dir = env::temp_dir().join(format!("aish-update-{}", sanitize_tag(tag)));
    let extract_dir = work_dir.join("extract");
    let archive_path = work_dir.join(asset);

    if work_dir.exists() {
        let _ = fs::remove_dir_all(&work_dir);
    }
    fs::create_dir_all(&extract_dir).map_err(|error| error.to_string())?;

    println!("downloading {url}");
    download_to_file(&url, &archive_path)?;
    extract_archive(asset, &archive_path, &extract_dir)?;

    let executable_name = if env::consts::OS == "windows" {
        "aish.exe"
    } else {
        "aish"
    };
    let downloaded = find_file(&extract_dir, executable_name)
        .ok_or_else(|| format!("release archive did not contain {executable_name}"))?;
    let current = env::current_exe().map_err(|error| error.to_string())?;

    if env::consts::OS == "windows" {
        let replacement = work_dir.join("aish-update.exe");
        fs::copy(&downloaded, &replacement).map_err(|error| error.to_string())?;
        start_windows_replace(&replacement, &current)?;
        println!("AiSH update prepared. This shell will exit so Windows can replace aish.exe.");
        return Ok(true);
    }

    let replacement = work_dir.join("aish-update");
    fs::copy(&downloaded, &replacement).map_err(|error| error.to_string())?;
    make_executable(&replacement)?;
    fs::rename(&replacement, &current).map_err(|error| {
        format!(
            "failed to replace {} with update: {error}",
            current.display()
        )
    })?;

    println!("update complete. Restart AiSH to use {tag}.");
    Ok(false)
}

fn platform_asset() -> Result<&'static str, String> {
    match (env::consts::OS, env::consts::ARCH) {
        ("windows", "x86_64") => Ok("aish-windows-x64.zip"),
        ("windows", "aarch64") => Ok("aish-windows-arm64.zip"),
        ("macos", "aarch64") => Ok("aish-macos-arm64.tar.gz"),
        ("linux", "x86_64") => Ok("aish-linux-x64.tar.gz"),
        ("linux", "aarch64") => Ok("aish-linux-arm64.tar.gz"),
        (os, arch) => Err(format!("updates are not available for {os}-{arch}")),
    }
}

fn download_to_file(url: &str, path: &Path) -> Result<(), String> {
    let response = ureq::get(url)
        .set("User-Agent", concat!("AiSH/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(60))
        .call()
        .map_err(|error| error.to_string())?;
    let mut reader = response.into_reader();
    let mut file = File::create(path).map_err(|error| error.to_string())?;
    io::copy(&mut reader, &mut file).map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    Ok(())
}

fn extract_archive(asset: &str, archive_path: &Path, extract_dir: &Path) -> Result<(), String> {
    let status = if asset.ends_with(".zip") {
        Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-Command",
                "Expand-Archive -LiteralPath $env:AISH_UPDATE_ARCHIVE -DestinationPath $env:AISH_UPDATE_EXTRACT_DIR -Force",
            ])
            .env("AISH_UPDATE_ARCHIVE", archive_path)
            .env("AISH_UPDATE_EXTRACT_DIR", extract_dir)
            .status()
    } else {
        Command::new("tar")
            .args([
                "-xzf",
                &archive_path.display().to_string(),
                "-C",
                &extract_dir.display().to_string(),
            ])
            .status()
    }
    .map_err(|error| error.to_string())?;

    if !status.success() {
        return Err(format!("failed to extract {asset}"));
    }
    Ok(())
}

fn start_windows_replace(replacement: &Path, current: &Path) -> Result<(), String> {
    let command = format!(
        "timeout /t 1 /nobreak > nul && copy /Y \"{}\" \"{}\" > nul && \"{}\" --repair-install --quiet",
        replacement.display(),
        current.display(),
        current.display()
    );
    Command::new("cmd.exe")
        .args([
            "/D",
            "/C",
            "start",
            "AiSH Updater",
            "/MIN",
            "cmd.exe",
            "/D",
            "/C",
            &command,
        ])
        .spawn()
        .map_err(|error| format!("failed to start Windows replacement process: {error}"))?;
    Ok(())
}

fn update_check_due() -> bool {
    let path = update_check_path();
    let Ok(text) = fs::read_to_string(path) else {
        return true;
    };
    let Ok(last) = text.trim().parse::<u64>() else {
        return true;
    };
    now_unix_secs().saturating_sub(last) >= update_check_interval_secs()
}

fn record_update_check() {
    let path = update_check_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, now_unix_secs().to_string());
}

fn update_check_interval_secs() -> u64 {
    env::var("AISH_UPDATE_CHECK_HOURS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map(|hours| hours.saturating_mul(60 * 60))
        .filter(|seconds| *seconds > 0)
        .unwrap_or(UPDATE_CHECK_INTERVAL_SECS)
}

fn update_check_path() -> PathBuf {
    if env::consts::OS == "windows" {
        windows_install_root()
            .join("state")
            .join("last-update-check")
    } else {
        env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir().join(".config"))
            .join("aish")
            .join("last-update-check")
    }
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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

fn find_file(root: &Path, filename: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|value| value.to_str()) == Some(filename) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file(&path, filename) {
                return Some(found);
            }
        }
    }
    None
}

fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .map_err(|error| error.to_string())?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn prompt_yes_no(prompt: &str, default_yes: bool) -> bool {
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    print!("{prompt} {suffix}: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return default_yes;
    }

    match input.trim().to_lowercase().as_str() {
        "" => default_yes,
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default_yes,
    }
}

fn sanitize_tag(tag: &str) -> String {
    tag.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn normalize_version(version: &str) -> String {
    version
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let latest = parse_version_parts(latest);
    let current = parse_version_parts(current);
    latest > current
}

fn parse_version_parts(version: &str) -> [u64; 3] {
    let normalized = normalize_version(version);
    let mut parts = [0_u64; 3];

    for (index, chunk) in normalized.split(['.', '-', '+']).take(3).enumerate() {
        let digits = chunk
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        parts[index] = digits.parse::<u64>().unwrap_or(0);
    }

    parts
}
