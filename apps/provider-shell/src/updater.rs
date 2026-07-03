use std::env;
use std::io::{self, Write};
use std::process::Command;
use std::time::Duration;

const REPO: &str = "DawnlightLabs/AiSH";
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/DawnlightLabs/AiSH/releases/latest";
const WINDOWS_INSTALLER_URL: &str = "https://aish.dawnlightlabs.com/install.ps1";
const UNIX_INSTALLER_URL: &str = "https://aish.dawnlightlabs.com/install";

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

    if args.iter().any(|arg| arg == "--update") {
        let assume_yes = args.iter().any(|arg| arg == "--yes" || arg == "-y");
        let should_exit = run_update(assume_yes);
        if should_exit {
            std::process::exit(0);
        }
        return true;
    }

    false
}

pub fn run_update_flow() -> bool {
    run_update(false)
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
    if env::consts::OS == "windows" {
        start_windows_update(tag)?;
        println!("AiSH update started in a detached PowerShell process.");
        println!("This provider shell will exit so Windows can replace aish.exe.");
        return Ok(true);
    }

    run_unix_update(tag)?;
    println!("update complete. Restart AiSH to use the new binary.");
    Ok(false)
}

fn start_windows_update(tag: &str) -> Result<(), String> {
    let tag = tag.replace(''', "''");
    let command = format!(
        "$ErrorActionPreference = 'Stop'; \
         Start-Sleep -Seconds 1; \
         $script = irm {WINDOWS_INSTALLER_URL}; \
         & ([scriptblock]::Create($script)) -Headless -SkipModel -Version '{tag}'"
    );

    Command::new("powershell.exe")
        .args(["-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &command])
        .spawn()
        .map_err(|error| format!("failed to start PowerShell updater: {error}"))?;

    Ok(())
}

fn run_unix_update(tag: &str) -> Result<(), String> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let status = Command::new(shell)
        .env("AISH_VERSION", tag)
        .env("AISH_HEADLESS", "1")
        .env("AISH_SKIP_MODEL", "1")
        .args(["-lc", &format!("curl -fsSL {UNIX_INSTALLER_URL} | bash")])
        .status()
        .map_err(|error| format!("failed to start Unix updater: {error}"))?;

    if !status.success() {
        return Err(format!("installer exited with status {status}"));
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

fn normalize_version(version: &str) -> String {
    version.trim().trim_start_matches('v').trim_start_matches('V').to_string()
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
