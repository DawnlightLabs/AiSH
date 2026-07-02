use crate::logging;
use aish_ai::ModelProfile;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_MODEL_URL: &str = "https://huggingface.co/bartowski/Qwen2.5-Coder-1.5B-Instruct-GGUF/resolve/main/Qwen2.5-Coder-1.5B-Instruct-Q4_K_M.gguf?download=true";

pub fn handle_setup_args() {
    if env::args().any(|arg| arg == "--download-model") {
        match download_model_if_missing(&default_model_path()) {
            Ok(()) => std::process::exit(0),
            Err(error) => {
                eprintln!("model download failed: {error}");
                std::process::exit(1);
            }
        }
    }

    if env::args().any(|arg| arg == "--setup") {
        run_setup_wizard(true);
    }
}

pub fn run_setup_wizard(exit_after: bool) {
    let install_dir = prompt_with_default(
        "Install location",
        default_install_dir().display().to_string(),
    );
    let install_dir = PathBuf::from(install_dir);
    let install_kind = prompt_with_default(
        "Install type: 1 provider shell only, 2 desktop app + provider shell",
        "2".to_string(),
    );
    let download_model = prompt_yes_no("Download/check the Qwen2.5 Coder model now", true);
    let add_to_path = prompt_yes_no("Add aish.exe to PATH", true);
    let set_model_env = prompt_yes_no("Set up local model path environment variable", true);
    let add_windows_terminal = prompt_yes_no("Add AiSH Provider Shell to Windows Terminal", true);
    let make_default_terminal =
        prompt_yes_no("Make AiSH the default Windows Terminal profile", false);
    let add_editor_profiles = prompt_yes_no(
        "Add AiSH terminal profile to VS Code/Cursor/Windsurf/VSCodium",
        true,
    );
    let command_log_policy = prompt_log_policy();
    let crash_log_sharing_opt_in =
        prompt_yes_no("Allow crash-log sharing prompts for Dawnlight Labs", false);

    let log_settings = logging::LogSettings {
        command_log_policy,
        crash_log_sharing_opt_in,
    };
    if let Err(error) = logging::write_settings(&log_settings) {
        eprintln!("setup warning: could not save log settings: {error}");
    } else {
        println!("saved log settings: {}", logging::settings_path().display());
        println!(
            "command logs path: {}",
            logging::command_log_path().display()
        );
        println!("logs stay local in this build; upload is not implemented.");
    }

    if let Err(error) = fs::create_dir_all(install_dir.join("bin")) {
        eprintln!("setup failed: could not create install directory: {error}");
        if exit_after {
            std::process::exit(1);
        }
        return;
    }

    let provider_target = match env::current_exe() {
        Ok(current) => {
            let name = if cfg!(target_os = "windows") {
                "aish.exe"
            } else {
                "aish"
            };
            let target = install_dir.join("bin").join(name);
            if let Err(error) = fs::copy(&current, &target) {
                eprintln!("setup warning: could not copy provider shell: {error}");
                None
            } else {
                println!("installed provider shell: {}", target.display());
                Some(target)
            }
        }
        Err(error) => {
            eprintln!("setup warning: could not locate current executable: {error}");
            None
        }
    };

    if install_kind.trim() == "2" {
        println!("desktop app install selected; package installer will place the desktop bundle for this OS.");
    } else {
        println!("provider shell only selected.");
    }

    if let Some(provider_path) = provider_target.as_deref() {
        if add_to_path {
            if let Err(error) = add_provider_to_path(&install_dir.join("bin")) {
                eprintln!("setup warning: could not update PATH: {error}");
            }
        }

        if set_model_env {
            if let Err(error) = persist_env_var(
                "AISH_MODEL_PATH",
                &default_model_path().display().to_string(),
            ) {
                eprintln!("setup warning: could not save AISH_MODEL_PATH: {error}");
            }
        }

        if add_windows_terminal {
            if let Err(error) = add_windows_terminal_profile(provider_path, make_default_terminal) {
                eprintln!("setup warning: could not update Windows Terminal profile: {error}");
            }
        }

        if add_editor_profiles {
            if let Err(error) = add_editor_terminal_profiles(provider_path) {
                eprintln!("setup warning: could not update editor terminal profiles: {error}");
            }
        }
    }

    println!("trusted app note: OS trust requires release signing/notarization. This setup prepares local install paths only.");

    if download_model {
        let model_path = default_model_path();
        if let Err(error) = download_model_if_missing(&model_path) {
            eprintln!("model download failed: {error}");
            eprintln!("set AISH_MODEL_PATH to an existing GGUF file, or retry setup later.");
        }
    }

    println!("setup complete");
    if exit_after {
        std::process::exit(0);
    }
}

pub fn ensure_model(profile: &ModelProfile) {
    let path = PathBuf::from(&profile.model_path);
    if is_valid_gguf(&path) {
        return;
    }

    println!("model missing or invalid: {}", path.display());
    if !prompt_yes_no("Download it now", true) {
        return;
    }

    if let Err(error) = download_model_if_missing(&path) {
        eprintln!("model download failed: {error}");
    }
}

fn download_model_if_missing(path: &Path) -> Result<(), String> {
    if is_valid_gguf(path) {
        println!("model already exists: {}", path.display());
        return Ok(());
    }

    let url = env::var("AISH_MODEL_URL").unwrap_or_else(|_| DEFAULT_MODEL_URL.to_string());
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    println!("downloading model to {}", path.display());
    println!("source: {url}");

    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(60))
        .call()
        .map_err(|error| error.to_string())?;
    let total = response
        .header("content-length")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let mut reader = response.into_reader();
    let partial_path = path.with_extension("gguf.part");
    let _ = fs::remove_file(&partial_path);
    let mut file = File::create(&partial_path)
        .map_err(|error| format!("failed to create {}: {error}", partial_path.display()))?;
    let mut buf = [0u8; 1024 * 128];
    let mut written = 0u64;

    loop {
        let n = reader.read(&mut buf).map_err(|error| error.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|error| error.to_string())?;
        written += n as u64;
        if total > 0 {
            let pct = (written as f64 / total as f64) * 100.0;
            print!("\r{pct:.1}%");
            let _ = io::stdout().flush();
        }
    }

    file.flush().map_err(|error| error.to_string())?;
    drop(file);
    if total > 0 && written != total {
        let _ = fs::remove_file(&partial_path);
        return Err(format!(
            "incomplete download: expected {total} bytes, received {written}"
        ));
    }
    if !is_valid_gguf(&partial_path) {
        let _ = fs::remove_file(&partial_path);
        return Err("downloaded file is not a valid GGUF model".to_string());
    }
    fs::rename(&partial_path, path)
        .map_err(|error| format!("failed to install {}: {error}", path.display()))?;
    if total > 0 {
        println!();
    }
    println!("model ready: {}", path.display());
    Ok(())
}

fn is_valid_gguf(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if metadata.len() < 16 * 1024 * 1024 {
        return false;
    }

    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic).is_ok() && &magic == b"GGUF"
}

fn add_provider_to_path(bin_dir: &Path) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        let bin = bin_dir.display().to_string();
        let current = env::var("PATH").unwrap_or_default();
        if current
            .split(';')
            .any(|entry| entry.trim_matches('"').eq_ignore_ascii_case(&bin))
        {
            println!("[✓] aish.exe already on PATH");
            return Ok(());
        }

        let next = if current.trim().is_empty() {
            bin.clone()
        } else {
            format!("{current};{bin}")
        };

        let status = Command::new("setx")
            .args(["PATH", &next])
            .status()
            .map_err(|error| error.to_string())?;

        if !status.success() {
            return Err("setx PATH failed".to_string());
        }

        println!("[✓] Add aish.exe to PATH");
        println!("    Open a new terminal window for PATH changes to take effect.");
        Ok(())
    } else {
        let shell_profile = shell_profile_path();
        let line = format!("export PATH=\"{}:$PATH\"", bin_dir.display());
        append_line_if_missing(&shell_profile, &line)?;
        println!(
            "[✓] Added AiSH provider shell to PATH via {}",
            shell_profile.display()
        );
        Ok(())
    }
}

fn persist_env_var(name: &str, value: &str) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        let status = Command::new("setx")
            .args([name, value])
            .status()
            .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err(format!("setx {name} failed"));
        }
    } else {
        let shell_profile = shell_profile_path();
        let line = format!("export {name}=\"{value}\"");
        append_line_if_missing(&shell_profile, &line)?;
    }

    println!("[✓] Set up local model path");
    Ok(())
}

fn add_windows_terminal_profile(provider_path: &Path, make_default: bool) -> Result<(), String> {
    if !cfg!(target_os = "windows") {
        return Ok(());
    }

    let mut updated_any = false;
    for settings_path in windows_terminal_settings_paths() {
        if !settings_path.exists() {
            continue;
        }

        let text = fs::read_to_string(&settings_path)
            .map_err(|error| format!("failed to read {}: {error}", settings_path.display()))?;
        let mut json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| format!("failed to parse {}: {error}", settings_path.display()))?;

        let profile_guid = "{8f6d930e-7f49-4bd8-9d29-aish00000001}";
        let commandline = provider_path.display().to_string();

        let profiles = json
            .get_mut("profiles")
            .and_then(|profiles| profiles.get_mut("list"))
            .and_then(|list| list.as_array_mut())
            .ok_or_else(|| format!("{} does not contain profiles.list", settings_path.display()))?;

        let mut found = false;
        for profile in profiles.iter_mut() {
            if profile.get("guid").and_then(|value| value.as_str()) == Some(profile_guid)
                || profile.get("name").and_then(|value| value.as_str()) == Some("AiSH")
            {
                profile["guid"] = serde_json::json!(profile_guid);
                profile["name"] = serde_json::json!("AiSH");
                profile["commandline"] = serde_json::json!(commandline);
                profile["startingDirectory"] = serde_json::json!("%USERPROFILE%");
                found = true;
            }
        }

        if !found {
            profiles.push(serde_json::json!({
                "guid": profile_guid,
                "name": "AiSH",
                "commandline": commandline,
                "startingDirectory": "%USERPROFILE%",
                "hidden": false
            }));
        }

        if make_default {
            json["defaultProfile"] = serde_json::json!(profile_guid);
        }

        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&json).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("failed to write {}: {error}", settings_path.display()))?;

        updated_any = true;
    }

    if updated_any {
        println!("[✓] Add AiSH Provider Shell to Windows Terminal");
        if make_default {
            println!("[✓] Make AiSH the default Windows Terminal profile");
        }
    } else {
        println!("[ ] Windows Terminal settings not found; skipping profile update");
    }

    Ok(())
}

fn add_editor_terminal_profiles(provider_path: &Path) -> Result<(), String> {
    if !cfg!(target_os = "windows") {
        println!("[ ] Editor profile auto-setup is currently implemented for Windows settings paths only");
        return Ok(());
    }

    let mut updated_any = false;
    for settings_path in editor_settings_paths() {
        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }

        let mut json = if settings_path.exists() {
            let text = fs::read_to_string(&settings_path)
                .map_err(|error| format!("failed to read {}: {error}", settings_path.display()))?;
            serde_json::from_str::<serde_json::Value>(&text)
                .unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        if !json.is_object() {
            json = serde_json::json!({});
        }

        json["terminal.integrated.profiles.windows"]["AiSH"] = serde_json::json!({
            "path": provider_path.display().to_string(),
            "args": [],
            "icon": "terminal"
        });

        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&json).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("failed to write {}: {error}", settings_path.display()))?;
        updated_any = true;
    }

    if updated_any {
        println!("[✓] Added AiSH terminal profile to VS Code-compatible editors");
        println!("    Codex, Claude Code, and similar CLI tools can run inside this AiSH terminal profile.");
        println!("    JetBrains IDEs do not expose one stable cross-IDE JSON terminal profile; set Shell path to aish manually.");
    }

    Ok(())
}

fn windows_terminal_settings_paths() -> Vec<PathBuf> {
    let local = env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir());

    vec![
        local
            .join("Packages")
            .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
            .join("LocalState")
            .join("settings.json"),
        local
            .join("Packages")
            .join("Microsoft.WindowsTerminalPreview_8wekyb3d8bbwe")
            .join("LocalState")
            .join("settings.json"),
    ]
}

fn editor_settings_paths() -> Vec<PathBuf> {
    let roaming = env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join("AppData").join("Roaming"));

    vec![
        roaming.join("Code").join("User").join("settings.json"),
        roaming.join("Cursor").join("User").join("settings.json"),
        roaming.join("Windsurf").join("User").join("settings.json"),
        roaming.join("VSCodium").join("User").join("settings.json"),
    ]
}

fn shell_profile_path() -> PathBuf {
    if cfg!(target_os = "macos") {
        home_dir().join(".zshrc")
    } else {
        env::var("SHELL")
            .ok()
            .and_then(|shell| {
                Path::new(&shell)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
            })
            .map(|name| {
                if name.contains("zsh") {
                    home_dir().join(".zshrc")
                } else if name.contains("fish") {
                    home_dir().join(".config").join("fish").join("config.fish")
                } else {
                    home_dir().join(".bashrc")
                }
            })
            .unwrap_or_else(|| home_dir().join(".bashrc"))
    }
}

fn append_line_if_missing(path: &Path, line: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    let current = fs::read_to_string(path).unwrap_or_default();
    if current
        .lines()
        .any(|existing| existing.trim() == line.trim())
    {
        return Ok(());
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;

    if !current.ends_with('\n') && !current.is_empty() {
        writeln!(file).map_err(|error| error.to_string())?;
    }
    writeln!(file, "{line}").map_err(|error| error.to_string())
}

fn prompt_with_default(label: &str, default_value: String) -> String {
    print!("{label} [{default_value}]: ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return default_value;
    }
    let trimmed = input.trim();
    if trimmed.is_empty() {
        default_value
    } else {
        trimmed.to_string()
    }
}

fn prompt_yes_no(label: &str, default_yes: bool) -> bool {
    let suffix = if default_yes { "Y/n" } else { "y/N" };
    print!("{label} [{suffix}]: ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return default_yes;
    }
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default_yes,
    }
}

fn prompt_log_policy() -> logging::CommandLogPolicy {
    println!("Local command logs are stored on this machine only.");
    println!("  1. Off");
    println!("  2. Failed commands only");
    println!("  3. All AiSH commands");
    let choice = prompt_with_default("Local command log policy", "2".to_string());
    match choice.trim().to_lowercase().as_str() {
        "1" | "off" | "none" => logging::CommandLogPolicy::Off,
        "3" | "all" => logging::CommandLogPolicy::All,
        _ => logging::CommandLogPolicy::FailedOnly,
    }
}

fn default_install_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir())
            .join("AiSH")
    } else if cfg!(target_os = "macos") {
        home_dir().join("Applications").join("AiSH")
    } else {
        home_dir().join(".local").join("aish")
    }
}

fn default_model_path() -> PathBuf {
    if let Ok(path) = env::var("AISH_MODEL_PATH") {
        return PathBuf::from(path);
    }
    home_dir()
        .join("Downloads")
        .join("aish-model")
        .join("models")
        .join("Qwen2.5-Coder-1.5B-Instruct-Q4_K_M.gguf")
}

fn home_dir() -> PathBuf {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
