use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const WINDOWS_UNINSTALLER: &str = include_str!("../windows_uninstall.ps1");
const WINDOWS_REGISTER_SCRIPT: &str = include_str!("../windows_register.ps1");

pub fn handle_args(args: &[String], version: &str) -> bool {
    if args.iter().any(|arg| arg == "--repair-install") {
        let quiet = args.iter().any(|arg| arg == "--quiet");
        if let Err(error) = repair_windows_app_registration(version, quiet) {
            eprintln!("install registration repair failed: {error}");
            std::process::exit(1);
        }
        return true;
    }

    if args.iter().any(|arg| arg == "--uninstall") {
        let assume_yes = args.iter().any(|arg| arg == "--yes" || arg == "-y");
        match run_uninstall(assume_yes) {
            Ok(_) => return true,
            Err(error) => {
                eprintln!("uninstall failed: {error}");
                std::process::exit(1);
            }
        }
    }

    false
}

pub fn ensure_windows_app_registration(version: &str) {
    if env::consts::OS != "windows" || !is_managed_windows_install() {
        return;
    }

    if windows_registration_present() {
        return;
    }

    if let Err(error) = repair_windows_app_registration(version, true) {
        if env::var("AISH_UPDATE_DEBUG").ok().as_deref() == Some("1") {
            eprintln!("AiSH app registration repair failed: {error}");
        }
    }
}

fn repair_windows_app_registration(version: &str, quiet: bool) -> Result<(), String> {
    if env::consts::OS != "windows" {
        if !quiet {
            println!("Windows app registration is only available on Windows.");
        }
        return Ok(());
    }

    let current = env::current_exe().map_err(|error| error.to_string())?;
    let install_root = windows_install_root();
    fs::create_dir_all(&install_root).map_err(|error| error.to_string())?;
    let uninstaller = install_root.join("uninstall.ps1");
    fs::write(&uninstaller, WINDOWS_UNINSTALLER).map_err(|error| error.to_string())?;

    let exe = current.display().to_string();
    let root = install_root.display().to_string();
    let status = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            WINDOWS_REGISTER_SCRIPT,
            &exe,
            &root,
            version,
        ])
        .status()
        .map_err(|error| error.to_string())?;

    if !status.success() {
        return Err("PowerShell failed to register AiSH as a Windows application".to_string());
    }

    if !quiet {
        println!("AiSH is registered in the Start menu and Installed apps.");
    }
    Ok(())
}

fn run_uninstall(assume_yes: bool) -> Result<bool, String> {
    if !assume_yes && !prompt_yes_no("Uninstall AiSH from this user account", false) {
        println!("uninstall cancelled");
        return Ok(false);
    }

    if env::consts::OS == "windows" {
        let install_root = windows_install_root();
        fs::create_dir_all(&install_root).map_err(|error| error.to_string())?;
        let uninstaller = install_root.join("uninstall.ps1");
        if !uninstaller.exists() {
            fs::write(&uninstaller, WINDOWS_UNINSTALLER).map_err(|error| error.to_string())?;
        }

        let uninstaller_text = uninstaller.display().to_string();
        let status = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &uninstaller_text,
            ])
            .status()
            .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err("AiSH uninstaller exited unsuccessfully".to_string());
        }
        return Ok(true);
    }

    uninstall_unix()?;
    println!("AiSH has been uninstalled. Open a new shell to refresh PATH.");
    Ok(true)
}

fn uninstall_unix() -> Result<(), String> {
    let install_root = if env::consts::OS == "macos" {
        home_dir().join("Applications").join("AiSH")
    } else {
        home_dir().join(".local").join("aish")
    };
    let bin_dir = install_root.join("bin");
    let export_line = format!("export PATH=\"{}:$PATH\"", bin_dir.display());

    for profile in [
        home_dir().join(".bashrc"),
        home_dir().join(".zshrc"),
        home_dir().join(".config").join("fish").join("config.fish"),
    ] {
        remove_line_if_present(&profile, &export_line)?;
    }

    if install_root.exists() {
        fs::remove_dir_all(&install_root).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn remove_line_if_present(path: &Path, line: &str) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let current = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let next = current
        .lines()
        .filter(|existing| existing.trim() != line.trim())
        .collect::<Vec<_>>()
        .join("\n");
    let next = if next.is_empty() {
        next
    } else {
        format!("{next}\n")
    };
    fs::write(path, next).map_err(|error| error.to_string())
}

fn windows_install_root() -> PathBuf {
    env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir())
        .join("AiSH")
}

fn is_managed_windows_install() -> bool {
    let Ok(current) = env::current_exe() else {
        return false;
    };
    let expected = windows_install_root().join("bin").join("aish.exe");
    current
        .display()
        .to_string()
        .eq_ignore_ascii_case(&expected.display().to_string())
}

fn windows_registration_present() -> bool {
    let shortcut = env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join("AppData").join("Roaming"))
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("AiSH.lnk");
    let uninstaller = windows_install_root().join("uninstall.ps1");
    let registry = Command::new("reg.exe")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\AiSH",
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    shortcut.exists() && uninstaller.exists() && registry
}

fn home_dir() -> PathBuf {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
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
