mod commands;
mod model_store;
mod shell;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::backend_status,
            commands::get_app_state,
            commands::inspect_project,
            commands::complete,
            commands::check_command_risk,
            commands::execute_shell_command,
            commands::list_model_profiles,
            commands::save_model_profiles,
            commands::run_local_model,
            commands::create_ai_card
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AiSH desktop app");
}
