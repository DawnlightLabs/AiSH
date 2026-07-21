use aish_core::Card;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};

mod structured_output;
use structured_output::{
    inspect_llama_cli_capabilities, StructuredOutputMode, COMMAND_CARD_GBNF,
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
    "You are AiSH's local shell planner. Return exactly one compact JSON object with action_type, command, and fallback_message. For runnable work, use action_type shell_command, put one directly runnable command in command, and leave fallback_message empty. For a question or a request that cannot be converted into one safe command, use action_type fallback_message, leave command empty, and put the response in fallback_message. Use only the supplied operating system, shell, context, and user intent. Never invent paths, filenames, usernames, installed tools, or facts. Do not include markdown or extra text. The host classifies risk and handles approval.".to_string()
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

    let capabilities = inspect_llama_cli_capabilities(&request.profile.llama_cli_path)?;
    let prompt = if capabilities.system_prompt {
        request.prompt.clone()
    } else {
        format!(
            "System instructions:\n{}\n\n{}",
            request.system_prompt, request.prompt
        )
    };

    let mut command = Command::new(&request.profile.llama_cli_path);
    command
        .stdin(Stdio::null())
        .arg("-m")
        .arg(&request.profile.model_path);

    if capabilities.system_prompt {
        command.arg("--system-prompt").arg(&request.system_prompt);
    }

    command
        .arg("-p")
        .arg(&prompt)
        .arg("-n")
        .arg(request.profile.max_tokens.to_string())
        .arg("--temp")
        .arg("0")
        .arg("--top-k")
        .arg("1")
        .arg("--seed")
        .arg("0")
        .arg("-c")
        .arg(request.profile.context_tokens.to_string());

    if capabilities.conversation {
        command.arg("--conversation");
    }
    if capabilities.single_turn {
        command.arg("--single-turn");
    }
    if capabilities.no_display_prompt {
        command.arg("--no-display-prompt");
    }
    if capabilities.color {
        command.args(["--color", "off"]);
    }
    if capabilities.simple_io {
        command.arg("--simple-io");
    }
    if capabilities.no_show_timings {
        command.arg("--no-show-timings");
    }
    if capabilities.log_disable {
        command.arg("--log-disable");
    }
    if capabilities.no_warmup {
        command.arg("--no-warmup");
    }

    match capabilities.mode {
        StructuredOutputMode::JsonSchema => {
            command.arg("--json-schema").arg(COMMAND_CARD_JSON_SCHEMA);
        }
        StructuredOutputMode::Grammar => {
            command.arg("--grammar").arg(COMMAND_CARD_GBNF);
        }
    }

    let constraint = match capabilities.mode {
        StructuredOutputMode::JsonSchema => "--json-schema <command-card-schema>",
        StructuredOutputMode::Grammar => "--grammar <command-card-grammar>",
    };
    let command_line = format!(
        "{} -m {} -p <prompt> -n {} --temp 0 --top-k 1 --seed 0 -c {} {}",
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
    let normalized = strip_ansi(raw);
    if let Some(json) = extract_command_card_json(&normalized) {
        return json;
    }

    let text = clean_runtime_text(&normalized);
    if text.trim().is_empty() {
        "No model response returned.".to_string()
    } else {
        text
    }
}

fn clean_runtime_text(raw: &str) -> String {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            !line.contains("llama_")
                && !line.contains("ggml_")
                && !line.contains("print_info:")
                && !line.starts_with("main: build")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_command_card_json(raw: &str) -> Option<String> {
    let mut selected = None;

    for (start, ch) in raw.char_indices() {
        if ch != '{' {
            continue;
        }
        let Some(length) = matching_json_object_length(&raw[start..]) else {
            continue;
        };
        let candidate = &raw[start..start + length];
        let Ok(value) = serde_json::from_str::<serde_json::Value>(candidate) else {
            continue;
        };
        if is_command_card_value(&value) {
            selected = Some(candidate.trim().to_string());
        }
    }

    selected
}

fn matching_json_object_length(value: &str) -> Option<usize> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(offset + ch.len_utf8());
                }
            }
            _ => {}
        }
    }

    None
}

fn is_command_card_value(value: &serde_json::Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    matches!(
        object
            .get("action_type")
            .and_then(serde_json::Value::as_str),
        Some("shell_command" | "fallback_message")
    ) && object
        .get("command")
        .and_then(serde_json::Value::as_str)
        .is_some()
        && object
            .get("fallback_message")
            .and_then(serde_json::Value::as_str)
            .is_some()
}

fn strip_ansi(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for code in chars.by_ref() {
                if ('@'..='~').contains(&code) {
                    break;
                }
            }
            continue;
        }
        output.push(ch);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{clean_model_output, extract_command_card_json};

    const CARD: &str = r#"{"action_type":"shell_command","command":"Set-Location \"$HOME\\Downloads\"","risk":"low","reason":"Navigate to Downloads.","fallback_message":""}"#;

    #[test]
    fn selects_the_command_card_instead_of_echoed_context_json() {
        let raw = format!(
            "Context JSON:\n{{\"cwd\":\"C:\\\\Users\\\\Amaan\",\"nested\":{{\"kind\":\"repo\"}}}}\nassistant:\n{CARD}\n"
        );
        assert_eq!(clean_model_output(&raw), CARD);
    }

    #[test]
    fn accepts_fenced_and_colored_strict_json() {
        let raw = format!("\u{1b}[36m```json\n{CARD}\n```\u{1b}[0m");
        assert_eq!(clean_model_output(&raw), CARD);
    }

    #[test]
    fn accepts_compact_command_cards() {
        let compact =
            r#"{"action_type":"shell_command","command":"git status","fallback_message":""}"#;
        assert_eq!(extract_command_card_json(compact).as_deref(), Some(compact));
    }

    #[test]
    fn rejects_unrelated_json_objects() {
        assert!(extract_command_card_json(r#"{"cwd":"C:/Users/Amaan"}"#).is_none());
    }
}
