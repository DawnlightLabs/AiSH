use crate::model_store;
use crate::shell;
use aish_ai::{run_gguf_model, ModelProfile, ModelRunRequest, ModelRunResult};
use aish_completion::demo_suggestions;
use aish_context::inspect_current_project;
use aish_core::{AppMode, AppState, CachePolicy, CommandTrace, ContextLevel};
use aish_provider::{
    parse_provider_mode, plan_provider_input, ProviderInputMode, ProviderPlan, ProviderPlanRequest,
};
use aish_safety::classify_risk;

#[tauri::command]
pub fn backend_status() -> String {
    "backend ready".to_string()
}

#[tauri::command]
pub fn get_app_state() -> AppState {
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
pub fn inspect_project() -> serde_json::Value {
    serde_json::to_value(inspect_current_project()).unwrap_or_else(|_| serde_json::json!({}))
}

#[tauri::command]
pub fn complete(prefix: String) -> serde_json::Value {
    serde_json::to_value(demo_suggestions(&prefix)).unwrap_or_else(|_| serde_json::json!([]))
}

#[tauri::command]
pub fn check_command_risk(command: String) -> serde_json::Value {
    serde_json::to_value(classify_risk(&command))
        .unwrap_or_else(|_| serde_json::json!({ "risk": "medium" }))
}

#[tauri::command]
pub fn execute_shell_command(
    command: String,
    allow_medium_risk: bool,
) -> Result<CommandTrace, String> {
    shell::run_shell_command(command, allow_medium_risk)
}

#[tauri::command]
pub fn list_model_profiles() -> Result<Vec<ModelProfile>, String> {
    model_store::list_profiles()
}

#[tauri::command]
pub fn save_model_profiles(profiles: Vec<ModelProfile>) -> Result<Vec<ModelProfile>, String> {
    model_store::save_profiles(profiles)
}

#[tauri::command]
pub async fn run_local_model(profile_id: String, prompt: String) -> Result<ModelRunResult, String> {
    let profile = model_store::find_profile(&profile_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        run_gguf_model(ModelRunRequest { profile, prompt })
    })
    .await
    .map_err(|error| format!("Model task failed: {error}"))?
}

#[tauri::command]
pub async fn create_ai_card(profile_id: String, intent: String) -> Result<ModelRunResult, String> {
    let profile = model_store::find_profile(&profile_id)?;
    let context =
        serde_json::to_value(inspect_current_project()).unwrap_or_else(|_| serde_json::json!({}));
    let prompt = aish_ai::build_command_card_prompt(&intent, &context);

    tauri::async_runtime::spawn_blocking(move || {
        run_gguf_model(ModelRunRequest { profile, prompt })
    })
    .await
    .map_err(|error| format!("Model task failed: {error}"))?
}

#[tauri::command]
pub async fn provider_plan(
    profile_id: String,
    input: String,
    mode: String,
) -> Result<ProviderPlan, String> {
    let profile = model_store::find_profile(&profile_id)?;
    let context =
        serde_json::to_value(inspect_current_project()).unwrap_or_else(|_| serde_json::json!({}));
    let provider_mode = parse_provider_mode(&mode).unwrap_or(ProviderInputMode::AiRun);

    tauri::async_runtime::spawn_blocking(move || {
        plan_provider_input(ProviderPlanRequest {
            mode: provider_mode,
            surface: "desktop".to_string(),
            input,
            context_json: context,
            profile: Some(profile),
        })
    })
    .await
    .map_err(|error| format!("Provider task failed: {error}"))
}
