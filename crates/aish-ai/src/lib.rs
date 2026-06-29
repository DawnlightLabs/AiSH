use aish_core::Card;
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    pub intent: String,
    pub os: String,
    pub shell: String,
    pub context_json: serde_json::Value,
    pub submode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub raw: String,
    pub card: Option<Card>,
    pub validation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub id: String,
    pub label: String,
    pub family: String,
    pub model_path: String,
    pub llama_cli_path: String,
    pub context_tokens: usize,
    pub max_tokens: usize,
    pub temperature: f32,
}

impl Default for ModelProfile {
    fn default() -> Self {
        Self {
            id: "local-gguf".to_string(),
            label: "Local GGUF".to_string(),
            family: "generic".to_string(),
            model_path: String::new(),
            llama_cli_path: "llama-cli".to_string(),
            context_tokens: 4096,
            max_tokens: 192,
            temperature: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRunRequest {
    pub profile: ModelProfile,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRunResult {
    pub ok: bool,
    pub command_line: String,
    pub output: String,
    pub error: String,
}

pub trait AiRuntime {
    fn create_card(&self, request: AiRequest) -> AiResponse;
}

pub struct NullAiRuntime;

impl AiRuntime for NullAiRuntime {
    fn create_card(&self, request: AiRequest) -> AiResponse {
        AiResponse {
            raw: String::new(),
            card: None,
            validation_error: Some(format!("No local AI runtime configured for: {}", request.intent)),
        }
    }
}

pub fn build_command_card_prompt(intent: &str, context_json: &serde_json::Value) -> String {
    format!(
        "You are AiSH, a local Windows terminal assistant. Convert the user request into exactly one JSON object. Do not use markdown. Do not include explanations outside JSON. Do not include <think> blocks.\n\nReturn one of these shapes:\n{{\"action_type\":\"command\",\"command\":\"...\",\"risk\":\"low|medium|high\",\"reason\":\"...\"}}\n{{\"action_type\":\"plan\",\"steps\":[{{\"command\":\"...\",\"risk\":\"low|medium|high\",\"reason\":\"...\"}}],\"reason\":\"...\"}}\n{{\"action_type\":\"fallback_message\",\"fallback_message\":\"...\",\"reason\":\"...\"}}\n\nRules:\n- Prefer PowerShell-compatible commands.\n- Use inspected project context when useful.\n- Do not invent tools if context does not show them.\n- Use low risk only for read-only commands.\n- Use medium/high risk for install, delete, deploy, cloud, admin, registry, chmod, reset, clean, publish, or mutation commands.\n\nUser request:\n{}\n\nContext JSON:\n{}\n",
        intent,
        context_json
    )
}

pub fn run_gguf_model(request: ModelRunRequest) -> Result<ModelRunResult, String> {
    if request.profile.model_path.trim().is_empty() {
        return Err("Model path is empty.".to_string());
    }
    if request.profile.llama_cli_path.trim().is_empty() {
        return Err("llama-cli path is empty.".to_string());
    }

    let mut command = Command::new(&request.profile.llama_cli_path);
    command
        .stdin(Stdio::null())
        .arg("-m")
        .arg(&request.profile.model_path)
        .arg("-p")
        .arg(&request.prompt)
        .arg("-n")
        .arg(request.profile.max_tokens.to_string())
        .arg("--temp")
        .arg(request.profile.temperature.to_string())
        .arg("-c")
        .arg(request.profile.context_tokens.to_string())
        .arg("--no-display-prompt");

    let command_line = format!(
        "{} -m {} -p <prompt> -n {} --temp {} -c {} --no-display-prompt",
        request.profile.llama_cli_path,
        request.profile.model_path,
        request.profile.max_tokens,
        request.profile.temperature,
        request.profile.context_tokens
    );

    let output = command
        .output()
        .map_err(|error| format!("Failed to start local model runtime: {error}"))?;

    Ok(ModelRunResult {
        ok: output.status.success(),
        command_line,
        output: String::from_utf8_lossy(&output.stdout).to_string(),
        error: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

pub fn default_timeout() -> Duration {
    Duration::from_secs(60)
}
