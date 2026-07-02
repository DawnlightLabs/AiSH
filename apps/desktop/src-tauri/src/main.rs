mod commands;
mod logging;
mod model_store;
mod shell;
mod terminal;

fn main() {
    tauri::Builder::default()
        .manage(terminal::TerminalState::default())
        .invoke_handler(tauri::generate_handler![
            commands::backend_status,
            commands::get_app_state,
            commands::inspect_project,
            commands::complete,
            commands::check_command_risk,
            commands::list_model_profiles,
            commands::save_model_profiles,
            commands::run_local_model,
            commands::create_ai_card,
            logging::get_log_settings,
            logging::save_log_settings,
            logging::record_command_log,
            terminal::terminal_open
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AiSH desktop app");
}
