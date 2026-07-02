use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandLogPolicy {
    Off,
    FailedOnly,
    All,
}

impl Default for CommandLogPolicy {
    fn default() -> Self {
        Self::FailedOnly
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSettings {
    pub command_log_policy: CommandLogPolicy,
    pub crash_log_sharing_opt_in: bool,
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            command_log_policy: CommandLogPolicy::FailedOnly,
            crash_log_sharing_opt_in: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandLogEntry {
    pub intent: Option<String>,
    pub command: Option<String>,
    pub status: String,
    pub risk: Option<String>,
    pub reason: Option<String>,
    pub error: Option<String>,
    pub surface: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredCommandLogEntry {
    timestamp_ms: u128,
    surface: String,
    intent: Option<String>,
    command: Option<String>,
    status: String,
    risk: Option<String>,
    reason: Option<String>,
    error: Option<String>,
}

#[tauri::command]
pub fn get_log_settings() -> Result<LogSettings, String> {
    read_settings()
}

#[tauri::command]
pub fn save_log_settings(settings: LogSettings) -> Result<LogSettings, String> {
    write_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn record_command_log(entry: CommandLogEntry) -> Result<(), String> {
    let settings = read_settings()?;
    if !should_log(&settings.command_log_policy, &entry.status) {
        return Ok(());
    }

    let stored = StoredCommandLogEntry {
        timestamp_ms: timestamp_ms(),
        surface: entry.surface.unwrap_or_else(|| "desktop".to_string()),
        intent: entry.intent,
        command: entry.command,
        status: entry.status,
        risk: entry.risk,
        reason: entry.reason,
        error: entry.error,
    };

    append_entry(&stored)
}

fn should_log(policy: &CommandLogPolicy, status: &str) -> bool {
    match policy {
        CommandLogPolicy::Off => false,
        CommandLogPolicy::All => true,
        CommandLogPolicy::FailedOnly => {
            let value = status.to_lowercase();
            value.contains("fail") || value.contains("error")
        }
    }
}

fn read_settings() -> Result<LogSettings, String> {
    let path = settings_path();
    if !path.exists() {
        let settings = LogSettings::default();
        write_settings(&settings)?;
        return Ok(settings);
    }

    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_settings(settings: &LogSettings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(&path, text).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn append_entry(entry: &StoredCommandLogEntry) -> Result<(), String> {
    let path = command_log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    let line = serde_json::to_string(entry).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    writeln!(file, "{line}").map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn settings_path() -> PathBuf {
    app_data_dir().join("logging-settings.json")
}

fn command_log_path() -> PathBuf {
    app_data_dir().join("command-log.jsonl")
}

fn app_data_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir())
            .join("AiSH")
    } else if cfg!(target_os = "macos") {
        home_dir().join("Library").join("Application Support").join("AiSH")
    } else {
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir().join(".local").join("share"))
            .join("aish")
    }
}

fn home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
