pub use aish_logging::{CommandLogEntry, LogSettings};

#[tauri::command]
pub fn get_log_settings() -> Result<LogSettings, String> {
    Ok(aish_logging::read_settings())
}

#[tauri::command]
pub fn save_log_settings(settings: LogSettings) -> Result<LogSettings, String> {
    aish_logging::write_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn record_command_log(entry: CommandLogEntry) -> Result<(), String> {
    aish_logging::record_command(entry)
}
