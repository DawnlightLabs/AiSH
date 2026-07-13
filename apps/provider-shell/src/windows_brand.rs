use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const PROFILE_GUID: &str = "{8f6d930e-7f49-4bd8-9d29-a15000000001}";
const WINDOWS_ICON_PNG: &[u8] =
    include_bytes!("../../../assets/png/aish-app-icon-dark-256x256.png");

pub fn refresh_windows_branding(
    provider_path: &Path,
    install_root: &Path,
    version: &str,
    make_default: bool,
) -> Result<(), String> {
    if !cfg!(target_os = "windows") {
        return Ok(());
    }

    let icon_path = write_versioned_icon(install_root, version)?;
    update_windows_terminal_profiles(provider_path, &icon_path, make_default)
}

fn write_versioned_icon(install_root: &Path, version: &str) -> Result<PathBuf, String> {
    let assets_dir = install_root.join("assets");
    fs::create_dir_all(&assets_dir)
        .map_err(|error| format!("failed to create {}: {error}", assets_dir.display()))?;

    let safe_version = version
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '.' || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let icon_path = assets_dir.join(format!("aish-{safe_version}.png"));
    fs::write(&icon_path, WINDOWS_ICON_PNG)
        .map_err(|error| format!("failed to write {}: {error}", icon_path.display()))?;

    if let Ok(entries) = fs::read_dir(&assets_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path == icon_path || !path.is_file() {
                continue;
            }
            let is_old_icon = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("aish-") && name.ends_with(".png"))
                .unwrap_or(false);
            if is_old_icon {
                let _ = fs::remove_file(path);
            }
        }
    }

    Ok(icon_path)
}

fn update_windows_terminal_profiles(
    provider_path: &Path,
    icon_path: &Path,
    make_default: bool,
) -> Result<(), String> {
    let commandline = provider_path.display().to_string();
    let icon = icon_path.display().to_string();

    for settings_path in windows_terminal_settings_paths() {
        if !settings_path.exists() {
            continue;
        }

        let text = fs::read_to_string(&settings_path)
            .map_err(|error| format!("failed to read {}: {error}", settings_path.display()))?;
        let mut json: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| format!("failed to parse {}: {error}", settings_path.display()))?;

        let profiles = json
            .get_mut("profiles")
            .and_then(|profiles| profiles.get_mut("list"))
            .and_then(|list| list.as_array_mut())
            .ok_or_else(|| format!("{} does not contain profiles.list", settings_path.display()))?;

        let mut found = false;
        for profile in profiles.iter_mut() {
            if profile.get("guid").and_then(|value| value.as_str()) == Some(PROFILE_GUID)
                || profile.get("name").and_then(|value| value.as_str()) == Some("AiSH")
            {
                profile["guid"] = serde_json::json!(PROFILE_GUID);
                profile["name"] = serde_json::json!("AiSH");
                profile["commandline"] = serde_json::json!(commandline);
                profile["startingDirectory"] = serde_json::json!("%USERPROFILE%");
                profile["icon"] = serde_json::json!(icon);
                profile["hidden"] = serde_json::json!(false);
                found = true;
            }
        }

        if !found {
            profiles.push(serde_json::json!({
                "guid": PROFILE_GUID,
                "name": "AiSH",
                "commandline": commandline,
                "startingDirectory": "%USERPROFILE%",
                "icon": icon,
                "hidden": false
            }));
        }

        if make_default {
            json["defaultProfile"] = serde_json::json!(PROFILE_GUID);
        }

        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&json).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("failed to write {}: {error}", settings_path.display()))?;
    }

    Ok(())
}

fn windows_terminal_settings_paths() -> Vec<PathBuf> {
    let local = env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join("AppData").join("Local"));

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
        local
            .join("Microsoft")
            .join("Windows Terminal")
            .join("settings.json"),
    ]
}

fn home_dir() -> PathBuf {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
