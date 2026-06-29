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
        "Return exactly one JSON object. No markdown. No explanation. No thinking text.\n\nSchema A: {{\"action_type\":\"command\",\"command\":\"...\",\"risk\":\"low|medium|high\",\"reason\":\"...\"}}\nSchema B: {{\"action_type\":\"fallback_message\",\"fallback_message\":\"...\",\"reason\":\"...\"}}\n\nRules: Prefer PowerShell commands. Low risk means read-only. Medium or high means changing files, installing, deleting, deploying, admin, registry, reset, clean, publish, or cloud actions.\n\nUser request: {}\n\nContext JSON: {}\n",
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
        .arg("--no-display-prompt")
        .arg("--single-turn")
        .arg("--reasoning")
        .arg("off");

    let command_line = format!(
        "{} -m {} -p <prompt> -n {} --temp {} -c {} --no-display-prompt --single-turn --reasoning off",
        request.profile.llama_cli_path,
        request.profile.model_path,
        request.profile.max_tokens,
        request.profile.temperature,
        request.profile.context_tokens
    );

    let output = command
        .output()
        .map_err(|error| format!("Failed to start local model runtime: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(ModelRunResult {
        ok: output.status.success(),
        command_line,
        output: clean_model_output(&stdout),
        error: if output.status.success() { String::new() } else { clean_runtime_text(&stderr) },
    })
}

fn clean_model_output(raw: &str) -> String {
    if let Some(json) = extract_json_object(raw) {
        return json;
    }

    let text = clean_runtime_text(raw);
    if text.trim().is_empty() {
        "No model response returned.".to_string()
    } else {
        text
    }
}

fn clean_runtime_text(raw: &str) -> String {
    let mut kept = Vec::new();
    let mut skipping_prompt = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.contains("llama.cpp") || trimmed.starts_with("build") || trimmed.starts_with("model") || trimmed.starts_with("modalities") {
            continue;
        }
        if trimmed == "available commands:" || trimmed.starts_with("/exit") || trimmed.starts_with("/regen") || trimmed.starts_with("/clear") || trimmed.starts_with("/read") || trimmed.starts_with("/glob") {
            continue;
        }
        if trimmed.starts_with('>') {
            skipping_prompt = true;
            continue;
        }
        if skipping_prompt && (trimmed.starts_with("Schema ") || trimmed.starts_with("Rules:") || trimmed.starts_with("User request:") || trimmed.starts_with("Context JSON:")) {
            continue;
        }
        if trimmed.starts_with("[") && trimmed.contains("thinking") {
            continue;
        }
        kept.push(line);
    }

    kept.join("\n").trim().to_string()
}

fn extract_json_object(text: &str) -> Option<String> {
    let needle = "\"action_type\"";
    let anchor = text.rfind(needle)?;
    let start = text[..anchor].rfind('{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in text[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                let end = start + offset + ch.len_utf8();
                return Some(text[start..end].trim().trim_matches('`').trim().to_string());
            }
        }
    }

    None
}

pub fn default_timeout() -> Duration {
    Duration::from_secs(60)
}
