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
    pub surface: String,
    pub intent: Option<String>,
    pub command: Option<String>,
    pub status: String,
    pub risk: Option<String>,
    pub reason: Option<String>,
    pub error: Option<String>,
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

pub fn read_settings() -> LogSettings {
    let path = settings_path();
    if !path.exists() {
        let settings = LogSettings::default();
        let _ = write_settings(&settings);
        return settings;
    }

    fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn write_settings(settings: &LogSettings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(&path, text).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub fn set_policy(policy: CommandLogPolicy) -> Result<LogSettings, String> {
    let mut settings = read_settings();
    settings.command_log_policy = policy;
    write_settings(&settings)?;
    Ok(settings)
}

pub fn set_crash_log_sharing(value: bool) -> Result<LogSettings, String> {
    let mut settings = read_settings();
    settings.crash_log_sharing_opt_in = value;
    write_settings(&settings)?;
    Ok(settings)
}

pub fn record_command(entry: CommandLogEntry) -> Result<(), String> {
    let settings = read_settings();
    if !should_log(&settings.command_log_policy, &entry.status) {
        return Ok(());
    }

    append_entry(&StoredCommandLogEntry {
        timestamp_ms: timestamp_ms(),
        surface: entry.surface,
        intent: entry.intent,
        command: entry.command,
        status: entry.status,
        risk: entry.risk,
        reason: entry.reason,
        error: entry.error,
    })
}

pub fn record_command_parts(
    surface: &str,
    intent: Option<&str>,
    command: Option<&str>,
    status: &str,
    risk: Option<&str>,
    reason: Option<&str>,
    error: Option<&str>,
) -> Result<(), String> {
    record_command(CommandLogEntry {
        surface: surface.to_string(),
        intent: intent.map(str::to_string),
        command: command.map(str::to_string),
        status: status.to_string(),
        risk: risk.map(str::to_string),
        reason: reason.map(str::to_string),
        error: error.map(str::to_string),
    })
}

pub fn describe_policy(policy: &CommandLogPolicy) -> &'static str {
    match policy {
        CommandLogPolicy::Off => "off",
        CommandLogPolicy::FailedOnly => "failed_only",
        CommandLogPolicy::All => "all",
    }
}

pub fn parse_policy(value: &str) -> Option<CommandLogPolicy> {
    match value.to_lowercase().as_str() {
        "off" | "none" => Some(CommandLogPolicy::Off),
        "failed" | "failed_only" | "failures" | "errors" => Some(CommandLogPolicy::FailedOnly),
        "all" => Some(CommandLogPolicy::All),
        _ => None,
    }
}

pub fn should_log(policy: &CommandLogPolicy, status: &str) -> bool {
    match policy {
        CommandLogPolicy::Off => false,
        CommandLogPolicy::All => true,
        CommandLogPolicy::FailedOnly => {
            let value = status.to_lowercase();
            value.contains("fail") || value.contains("error")
        }
    }
}

pub fn settings_path() -> PathBuf {
    app_data_dir().join("logging-settings.json")
}

pub fn command_log_path() -> PathBuf {
    app_data_dir().join("command-log.jsonl")
}

pub fn app_data_dir() -> PathBuf {
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
