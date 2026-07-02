use aish_core::Card;
use serde::{Deserialize, Serialize};
use std::path::Path;
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

pub fn build_command_card_prompt(intent: &str, context_json: &serde_json::Value) -> String {
    let os = target_os();
    let shell = target_shell();
    let family = shell_family(&os, &shell);
    let context = serde_json::to_string_pretty(context_json).unwrap_or_else(|_| "{}".to_string());

    let command_contract = match family {
        "powershell" => "Return one Windows PowerShell-compatible command. Use PowerShell cmdlets and syntax only. Use Set-Location for changing folders. Use Get-ChildItem for listing/searching. Use Remove-Item -LiteralPath <path> -Recurse -Force for folder deletion. Do not use cmd.exe commands such as del/rmdir/copy/xcopy, and do not use cmd.exe chaining. Use semicolon-separated PowerShell statements when chaining is necessary. Never use && for Windows PowerShell compatibility.",
        "fish" => "Return one fish-compatible shell command. Use POSIX-style filesystem paths where possible, but avoid bash-only syntax when fish syntax differs. Use $HOME for user-profile folders.",
        _ => "Return one POSIX shell command suitable for bash/zsh. Use $HOME for user-profile folders. Use find/ls/grep/sed/awk/git/npm style commands where appropriate.",
    };

    let path_rules = match family {
        "powershell" => "Path rules for PowerShell: if no path is named, omit -Path and rely on the live shell current location. If the user names Downloads/Desktop/Documents, build it with Join-Path $env:USERPROFILE '<FolderName>'. If the user provides an absolute path such as D:\\, C:\\work, /tmp, or ~/work, use that path directly; never prefix it with $env:USERPROFILE or $HOME. Never produce paths like $env:USERPROFILE\\D:\\. Use -LiteralPath for exact paths. Use -Filter only for filename patterns, not for folder names. Use -File only when files specifically are requested. Use Select-Object -ExpandProperty FullName only when exact paths or locations are requested.",
        _ => "Path rules for POSIX shells: if no path is named, use . or omit the path and rely on the live shell current location. If the user names Downloads/Desktop/Documents, build it from $HOME. If the user provides an absolute path such as /tmp or /Volumes/Data, use it directly. Use find -name only when searching for an unknown item by name. Use file-only predicates only when files specifically are requested. Print full paths only when exact paths or locations are requested.",
    };

    format!(
        "You are Ken, the AiSH command planner.\nReturn exactly one JSON object and nothing else. No markdown. No prose. No thinking text.\nUse keys: action_type, command, risk, reason. For fallback use action_type, fallback_message, reason.\nThe command must be a single runnable command for this environment.\n\nEnvironment:\n- OS: {os}\n- Shell: {shell}\n- Shell family: {family}\n- Context JSON: {context}\n\nCommand contract:\n- {command_contract}\n- The command runs in the user's existing live shell session.\n- Do not invent the current directory. Use the current directory from context when available.\n- Never output placeholder usernames, tutorial paths, angle-bracket placeholders, or sample targets not present in the user request.\n- If a user-profile folder is named, build it from the shell's home environment variable.\n- If the user says go to, cd to, change to, or open a folder in the shell, return a directory-change command, not a literal phrase.\n- If the user asks to delete/remove a file or folder, return the correct destructive command and mark risk high. The app or provider shell will request approval.\n\nPath and output rules:\n- {path_rules}\n- Do not copy filenames, folder names, or examples that are not in the user request.\n\nRisk rules:\n- Read-only inspection, recursive listing, sorting, filtering, text search, status checks, version checks, and path searches are low risk even if long-running.\n- Commands that delete, overwrite, move, rename, install, uninstall, publish, deploy, push, mutate git history, change permissions, edit registry, stop services/processes, or alter cloud/system state are medium or high risk.\n- For medium/high risk, still return the best command card. The app or provider shell will request approval.\n\nQuality rules:\n- Keep the command complete and directly runnable.\n- Keep the reason short and factual.\n\nUser request:\n{intent}\n"
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
