use aish_completion::demo_suggestions;
use aish_context::inspect_current_project;
use aish_core::{AppMode, AppState, CachePolicy, ContextLevel};
use aish_safety::classify_risk;

#[tauri::command]
fn backend_status() -> String {
    "backend ready · PTY next".to_string()
}

#[tauri::command]
fn get_app_state() -> AppState {
    AppState {
        mode: AppMode::History,
        ai_submode: "suggest".to_string(),
        context_level: ContextLevel::Project,
        cache_policy: CachePolicy::ProjectOnly,
        shell: "powershell".to_string(),
        cwd: std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| ".".to_string()),
    }
}

#[tauri::command]
fn inspect_project() -> serde_json::Value {
    serde_json::to_value(inspect_current_project()).unwrap_or_else(|_| serde_json::json!({}))
}

#[tauri::command]
fn complete(prefix: String) -> serde_json::Value {
    serde_json::to_value(demo_suggestions(&prefix)).unwrap_or_else(|_| serde_json::json!([]))
}

#[tauri::command]
fn check_command_risk(command: String) -> serde_json::Value {
    serde_json::to_value(classify_risk(&command)).unwrap_or_else(|_| serde_json::json!({ "risk": "medium" }))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            backend_status,
            get_app_state,
            inspect_project,
            complete,
            check_command_risk
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AiSH desktop app");
}
