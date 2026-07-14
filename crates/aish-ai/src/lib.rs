use aish_core::Card;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};

mod structured_output;
use structured_output::{
    detect_structured_output_mode, StructuredOutputMode, COMMAND_CARD_GBNF,
    COMMAND_CARD_JSON_SCHEMA,
};

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
    pub system_prompt: String,
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
            validation_error: Some(format!(
                "No local AI runtime configured for: {}",
                request.intent
            )),
        }
    }
}

fn target_os() -> String {
    std::env::var("AISH_TARGET_OS").unwrap_or_else(|_| std::env::consts::OS.to_string())
}

fn target_shell() -> String {
    std::env::var("AISH_TARGET_SHELL")
        .or_else(|_| std::env::var("SHELL"))
        .or_else(|_| std::env::var("COMSPEC"))
        .unwrap_or_else(|_| {
            if std::env::consts::OS == "windows" {
                "powershell".to_string()
            } else {
                "sh".to_string()
            }
        })
}

fn shell_family(os: &str, shell: &str) -> &'static str {
    let value = shell.to_lowercase();
    if os == "windows" || value.contains("powershell") || value.contains("pwsh") {
        "powershell"
    } else if value.contains("fish") {
        "fish"
    } else {
        "posix"
    }
}

pub fn build_command_card_system_prompt() -> String {
    "You are AiSH's local shell planner. Produce one command card for the user's current shell. Use only the supplied operating system, shell, current context, and user intent. The command must be directly runnable in that shell and must not invent paths, filenames, usernames, installed tools, or facts. Use fallback_message only when no useful command can be produced. Read-only work is low risk; state-changing work is medium or high risk. The host independently validates risk and approval. Keep the reason brief.".to_string()
}

pub fn build_command_card_prompt(intent: &str, context_json: &serde_json::Value) -> String {
    let os = target_os();
    let shell = target_shell();
    let family = shell_family(&os, &shell);
    let context = serde_json::to_string_pretty(context_json).unwrap_or_else(|_| "{}".to_string());
    let dialect = match family {
        "powershell" => "Use Windows PowerShell syntax.",
        "fish" => "Use fish shell syntax.",
        _ => "Use POSIX syntax suitable for bash or zsh.",
    };

    format!(
        "Operating system: {os}
Shell: {shell}
Shell family: {family}
Shell constraint: {dialect}
Context JSON:
{context}

User intent:
{intent}"
    )
}

pub fn run_gguf_model(request: ModelRunRequest) -> Result<ModelRunResult, String> {
    if request.profile.model_path.trim().is_empty() {
        return Err("Model path is empty.".to_string());
    }
    if request.profile.llama_cli_path.trim().is_empty() {
        return Err("llama-cli path is empty.".to_string());
    }
    if !Path::new(&request.profile.model_path).is_file() {
        return Err(format!(
            "Model file is missing: {}. Run `docker compose run --rm model`.",
            request.profile.model_path
        ));
    }
    if !Path::new(&request.profile.llama_cli_path).is_file() {
        return Err(format!(
            "llama-cli is missing: {}",
            request.profile.llama_cli_path
        ));
    }

    let structured_output = detect_structured_output_mode(&request.profile.llama_cli_path)?;
    let mut command = Command::new(&request.profile.llama_cli_path);
    command
        .stdin(Stdio::null())
        .arg("-m")
        .arg(&request.profile.model_path)
        .arg("--system-prompt")
        .arg(&request.system_prompt)
        .arg("-p")
        .arg(&request.prompt)
        .arg("-n")
        .arg(request.profile.max_tokens.to_string())
        .arg("--temp")
        .arg("0")
        .arg("--top-k")
        .arg("1")
        .arg("--seed")
        .arg("0")
        .arg("-c")
        .arg(request.profile.context_tokens.to_string())
        .arg("--conversation")
        .arg("--single-turn")
        .arg("--no-display-prompt")
        .arg("--reasoning")
        .arg("off");

    match structured_output {
        StructuredOutputMode::JsonSchema => {
            command.arg("--json-schema").arg(COMMAND_CARD_JSON_SCHEMA);
        }
        StructuredOutputMode::Grammar => {
            command.arg("--grammar").arg(COMMAND_CARD_GBNF);
        }
    }

    let constraint = match structured_output {
        StructuredOutputMode::JsonSchema => "--json-schema <command-card-schema>",
        StructuredOutputMode::Grammar => "--grammar <command-card-grammar>",
    };
    let command_line = format!(
        "{} -m {} --system-prompt <system> -p <prompt> -n {} --temp 0 --top-k 1 --seed 0 -c {} --conversation --single-turn --no-display-prompt --reasoning off {}",
        request.profile.llama_cli_path,
        request.profile.model_path,
        request.profile.max_tokens,
        request.profile.context_tokens,
        constraint
    );

    let output = command
        .output()
        .map_err(|error| format!("Failed to start local model runtime: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!(
            "Local model runtime failed: {}",
            clean_runtime_text(&stderr)
        ));
    }

    Ok(ModelRunResult {
        ok: true,
        command_line,
        output: clean_model_output(&stdout),
        error: String::new(),
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
        if trimmed.contains("llama_")
            || trimmed.contains("ggml_")
            || trimmed.contains("print_info:")
        {
            continue;
        }
        if trimmed.starts_with("You are Ken") {
            skipping_prompt = true;
            continue;
        }
        if skipping_prompt {
            if trimmed.starts_with('{') {
                skipping_prompt = false;
            } else {
                continue;
            }
        }
        kept.push(trimmed.to_string());
    }

    kept.join("\n")
}

fn extract_json_object(raw: &str) -> Option<String> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(raw[start..=end].trim().to_string())
}
