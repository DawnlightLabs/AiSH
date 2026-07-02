pub use aish_logging::{CommandLogPolicy, LogSettings};
use std::path::PathBuf;

pub fn read_settings() -> LogSettings {
    aish_logging::read_settings()
}

pub fn write_settings(settings: &LogSettings) -> Result<(), String> {
    aish_logging::write_settings(settings)
}

pub fn set_policy(policy: CommandLogPolicy) -> Result<LogSettings, String> {
    aish_logging::set_policy(policy)
}

pub fn set_crash_log_sharing(value: bool) -> Result<LogSettings, String> {
    aish_logging::set_crash_log_sharing(value)
}

pub fn record_command(
    intent: Option<&str>,
    command: Option<&str>,
    status: &str,
    risk: Option<&str>,
    reason: Option<&str>,
    error: Option<&str>,
) {
    let _ = aish_logging::record_command_parts(
        "provider_shell",
        intent,
        command,
        status,
        risk,
        reason,
        error,
    );
}

pub fn describe_policy(policy: &CommandLogPolicy) -> &'static str {
    aish_logging::describe_policy(policy)
}

pub fn parse_policy(value: &str) -> Option<CommandLogPolicy> {
    aish_logging::parse_policy(value)
}

pub fn settings_path() -> PathBuf {
    aish_logging::settings_path()
}

pub fn command_log_path() -> PathBuf {
    aish_logging::command_log_path()
}
