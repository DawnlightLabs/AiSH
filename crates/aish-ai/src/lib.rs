use aish_core::Card;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

mod semantic_plan;
mod structured_output;
pub use semantic_plan::{
    parse_semantic_plan, PlanParseFailure, PlanParseSuccess, SemanticPlan, SemanticPlanKind,
};
use structured_output::{
    inspect_llama_cli_capabilities, StructuredOutputMode, SEMANTIC_PLAN_GBNF,
    SEMANTIC_PLAN_JSON_SCHEMA,
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
    #[serde(default = "default_structured_strategy")]
    pub structured_output_strategy: String,
    #[serde(default)]
    pub chat_template: Option<String>,
    #[serde(default)]
    pub use_system_prompt: bool,
    #[serde(default = "default_retry_count")]
    pub retry_count: usize,
    #[serde(default)]
    pub stop_sequences: Vec<String>,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

fn default_structured_strategy() -> String {
    "auto".to_string()
}

fn default_retry_count() -> usize {
    1
}

fn default_timeout_seconds() -> u64 {
    60
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
            structured_output_strategy: default_structured_strategy(),
            chat_template: None,
            use_system_prompt: false,
            retry_count: default_retry_count(),
            stop_sequences: Vec::new(),
            timeout_seconds: default_timeout_seconds(),
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
    pub raw_stdout: String,
    pub raw_stderr: String,
    pub exit_code: Option<i32>,
    pub structured_output: String,
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

pub fn validate_shell_command_dialect(command: &str) -> Result<(), String> {
    let os = target_os();
    let shell = target_shell();
    validate_shell_command_dialect_for(command, shell_family(&os, &shell))
}

fn validate_shell_command_dialect_for(command: &str, family: &str) -> Result<(), String> {
    if family == "powershell"
        && !is_managed_cmd_command(command)
        && command
            .split_whitespace()
            .any(|token| token.len() > 1 && token.starts_with('/') && !token.contains(['\\', ':']))
    {
        return Err(
            "PowerShell plans must use PowerShell parameters, not CMD slash switches.".to_string(),
        );
    }
    if family == "powershell" && contains_powershell_wildcard_variable(command) {
        return Err(
            "PowerShell plans must not reference wildcard-named variables such as $*.".to_string(),
        );
    }
    if family == "powershell" && is_posix_file_test(command) {
        return Err(
            "PowerShell plans must use Test-Path rather than the POSIX test command.".to_string(),
        );
    }
    if family == "powershell" && contains_posix_flags_on_powershell_alias(command) {
        return Err(
            "PowerShell plans must use native Verb-Noun cmdlets and parameters rather than Unix-style flags on PowerShell aliases."
                .to_string(),
        );
    }
    if family == "powershell" && contains_cmd_only_file_creation(command) {
        return Err(
            "PowerShell plans must use New-Item or Set-Content rather than CMD-only NUL or echo-dot file creation."
                .to_string(),
        );
    }
    Ok(())
}

fn is_managed_cmd_command(command: &str) -> bool {
    let command = command.trim_start().to_ascii_lowercase();
    command.starts_with("cmd.exe /d /s /c '") || command.starts_with("cmd /d /s /c '")
}

fn is_posix_file_test(command: &str) -> bool {
    let mut tokens = command.split_whitespace();
    tokens.next().is_some_and(|token| token == "test")
        && tokens
            .next()
            .is_some_and(|flag| matches!(flag, "-e" | "-f" | "-d" | "-r" | "-w" | "-x"))
}

fn contains_posix_flags_on_powershell_alias(command: &str) -> bool {
    const POWERSHELL_ALIASES: &[&str] = &[
        "ls", "dir", "cat", "type", "sort", "where", "select", "pwd", "echo", "ps",
    ];
    command.split(['|', ';', '&']).any(|segment| {
        let mut tokens = segment.split_whitespace();
        let program = tokens
            .next()
            .unwrap_or_default()
            .trim_matches(['(', ')', '{', '}', '\'', '"'])
            .to_ascii_lowercase();
        POWERSHELL_ALIASES.contains(&program.as_str())
            && tokens.any(|token| {
                let option = token.trim_matches([',', ')', '}', '\'', '"']);
                option.starts_with('-')
                    && option.len() > 1
                    && option[1..].chars().all(|ch| ch.is_ascii_alphabetic())
                    && option.len() <= 4
            })
    })
}

fn contains_cmd_only_file_creation(command: &str) -> bool {
    command.split(['|', ';', '&']).any(|segment| {
        let normalized = segment.trim().to_ascii_lowercase();
        let tokens = normalized.split_whitespace().collect::<Vec<_>>();
        matches!(
            tokens.as_slice(),
            [program, device, rest @ ..]
                if (*program == "type" && *device == "nul"
                    && rest.iter().any(|token| token.contains('>')))
                    || (*program == "copy" && *device == "nul")
        ) || normalized.starts_with("echo.>")
            || normalized.starts_with("echo. >")
    })
}

fn contains_powershell_wildcard_variable(command: &str) -> bool {
    let mut chars = command.chars().peekable();
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '`' && !single_quoted {
            escaped = true;
            continue;
        }
        match ch {
            '\'' if !double_quoted => single_quoted = !single_quoted,
            '"' if !single_quoted => double_quoted = !double_quoted,
            '$' if !single_quoted && chars.peek() == Some(&'*') => return true,
            _ => {}
        }
    }
    false
}

pub fn build_semantic_plan_system_prompt() -> String {
    "You are AiSH's intent planner. Return exactly one compact JSON semantic plan. Allowed kinds: change_directory uses target and optional scope; shell_command uses payload; answer uses message; clarification uses message only when required information is missing. A shell_command payload may contain two to four ordered commands separated by semicolons only when the objective genuinely requires multiple steps; the host executes them separately and stops on failure. Use recent session turns to resolve concise follow-ups when the referenced target or choice is unambiguous. For navigation, return the user's target rather than cd or Set-Location. Explanatory questions asking why, how, what, or for an explanation must return an answer and must not run a command merely to explain a general cause or concept. Only explicit requests to display, list, find, check, or inspect current machine, filesystem, process, environment, repository, or project state should return a shell_command that observes that state; never invent observed state as an answer. Do not add risk, shell, status, reasoning, Markdown, placeholders, or extra text. Never invent paths, files, tools, or facts. The host resolves paths, validates commands, classifies risk, and handles approval.".to_string()
}

pub fn build_semantic_plan_prompt(intent: &str, context_json: &serde_json::Value) -> String {
    let os = target_os();
    let shell = target_shell();
    build_semantic_plan_prompt_for(intent, context_json, &os, &shell)
}

fn build_semantic_plan_prompt_for(
    intent: &str,
    context_json: &serde_json::Value,
    os: &str,
    shell: &str,
) -> String {
    let family = shell_family(&os, &shell);
    let context = serde_json::to_string_pretty(context_json).unwrap_or_else(|_| "{}".to_string());
    let dialect = match family {
        "powershell" => {
            "Use native Windows PowerShell Verb-Noun cmdlets and parameters. Do not use Unix-style flags on PowerShell aliases. Do not use CMD built-ins or slash-style CMD switches."
        }
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
{intent}

Return one minimal semantic-plan JSON object."
    )
}

pub fn build_repair_prompt(
    intent: &str,
    context_json: &serde_json::Value,
    malformed_output: &str,
) -> String {
    let bounded = malformed_output
        .chars()
        .rev()
        .take(1200)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!(
        "{}\n\nThe previous response was invalid or incomplete:\n{}\n\nRepair the reported error instead of repeating the rejected response. Obey the specified shell family and scope. Return only one complete semantic-plan JSON object.",
        build_semantic_plan_prompt(intent, context_json),
        bounded
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
    let use_system_prompt = capabilities.system_prompt && request.profile.use_system_prompt;
    let prompt = if use_system_prompt {
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

    if use_system_prompt {
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

    if capabilities.gpu_layers {
        command.args(["--gpu-layers", "auto"]);
    }
    if capabilities.fit {
        command.args(["--fit", "on"]);
    }

    // Structured planning is a one-shot completion. Conversation/single-turn mode
    // appends chat-template special tokens after the finite JSON grammar is complete
    // on some llama.cpp builds, producing an empty-grammar-stack sampler failure.
    if capabilities.no_display_prompt {
        command.arg("--no-display-prompt");
    }
    if capabilities.color {
        command.args(["--color", "off"]);
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
    if capabilities.no_conversation {
        command.arg("--no-conversation");
    }

    if let Some(template) = request
        .profile
        .chat_template
        .as_deref()
        .filter(|_| capabilities.chat_template)
    {
        command.arg("--chat-template").arg(template);
    }

    for stop in &request.profile.stop_sequences {
        command.arg("--reverse-prompt").arg(stop);
    }

    let mode = select_structured_mode(
        &request.profile.structured_output_strategy,
        capabilities.mode,
        capabilities.json_schema,
        capabilities.grammar,
    )?;
    match mode {
        StructuredOutputMode::JsonSchema => {
            command.arg("--json-schema").arg(SEMANTIC_PLAN_JSON_SCHEMA);
        }
        StructuredOutputMode::Grammar => {
            command.arg("--grammar").arg(SEMANTIC_PLAN_GBNF);
        }
    }

    let constraint = match mode {
        StructuredOutputMode::JsonSchema => "json_schema",
        StructuredOutputMode::Grammar => "grammar",
    };
    let acceleration = if capabilities.gpu_layers {
        " --gpu-layers auto"
    } else {
        ""
    };
    let fitting = if capabilities.fit { " --fit on" } else { "" };
    let command_line = format!(
        "llama-cli -m <active-model> -p <redacted-prompt> -n {} --temp 0 --top-k 1 --seed 0 -c {}{}{} --no-conversation --structured-output {}",
        request.profile.max_tokens,
        request.profile.context_tokens,
        acceleration,
        fitting,
        constraint
    );

    let output = capture_with_timeout(command, request.profile.timeout_seconds)?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let fatal_text = runtime_fatal_text(&stdout).or_else(|| runtime_fatal_text(&stderr));
    let controlled_stop = is_controlled_stop(
        output.status.code(),
        &stdout,
        &request.profile.stop_sequences,
        fatal_text.as_deref(),
    );
    let runtime_ok =
        (output.status.success() || controlled_stop) && fatal_text.is_none() && !output.timed_out;
    Ok(ModelRunResult {
        ok: runtime_ok,
        command_line,
        output: select_model_text(&stdout, &stderr, &prompt),
        error: if output.timed_out {
            format!(
                "Local model runtime exceeded the {} second planning limit.",
                request.profile.timeout_seconds
            )
        } else if runtime_ok {
            String::new()
        } else {
            format!(
                "Local model runtime exited with {}: {}",
                output.status.code().unwrap_or(-1),
                fatal_text.unwrap_or_else(|| clean_runtime_text(&stderr))
            )
        },
        raw_stdout: sanitize_diagnostic_text(&stdout),
        raw_stderr: sanitize_diagnostic_text(&stderr),
        exit_code: output.status.code(),
        structured_output: constraint.to_string(),
    })
}

fn is_controlled_stop(
    exit_code: Option<i32>,
    stdout: &str,
    stop_sequences: &[String],
    fatal_text: Option<&str>,
) -> bool {
    exit_code == Some(130)
        && fatal_text.is_none()
        && !stop_sequences.is_empty()
        && stdout.contains('{')
        && stdout.contains('}')
}

struct CapturedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    timed_out: bool,
}

fn capture_with_timeout(
    mut command: Command,
    timeout_seconds: u64,
) -> Result<CapturedOutput, String> {
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Failed to start local model runtime: {error}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Local model runtime stdout was not captured.".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Local model runtime stderr was not captured.".to_string())?;
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });
    let started = Instant::now();
    let timeout = Duration::from_secs(timeout_seconds.max(1));
    let (status, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (status, false),
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(25)),
            Ok(None) => {
                let _ = child.kill();
                let status = child
                    .wait()
                    .map_err(|error| format!("Failed to stop timed-out model runtime: {error}"))?;
                break (status, true);
            }
            Err(error) => {
                let _ = child.kill();
                return Err(format!(
                    "Failed while waiting for local model runtime: {error}"
                ));
            }
        }
    };
    let stdout = stdout_reader
        .join()
        .map_err(|_| "Local model stdout reader failed.".to_string())?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "Local model stderr reader failed.".to_string())?;
    Ok(CapturedOutput {
        status,
        stdout,
        stderr,
        timed_out,
    })
}

fn runtime_fatal_text(raw: &str) -> Option<String> {
    raw.lines()
        .map(str::trim)
        .find(|line| {
            line.starts_with("Error:")
                || line.contains("Failed to initialize samplers")
                || line.contains("failed to load model")
        })
        .map(str::to_string)
}

fn select_structured_mode(
    requested: &str,
    automatic: StructuredOutputMode,
    json_schema: bool,
    grammar: bool,
) -> Result<StructuredOutputMode, String> {
    match requested.trim().to_ascii_lowercase().as_str() {
        "auto" | "" => Ok(automatic),
        "json_schema" | "json-schema" if json_schema => Ok(StructuredOutputMode::JsonSchema),
        "grammar" if grammar => Ok(StructuredOutputMode::Grammar),
        unsupported => Err(format!(
            "The active model profile requests structured-output strategy '{unsupported}', but the installed llama-cli does not support it. Repair the runtime or select another model profile."
        )),
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

fn select_model_text(stdout: &str, stderr: &str, prompt: &str) -> String {
    let stdout = strip_prompt_echo(&strip_ansi(stdout), prompt)
        .trim()
        .to_string();
    if !stdout.is_empty() {
        return stdout;
    }
    clean_runtime_text(&strip_ansi(stderr))
}

fn strip_prompt_echo(stdout: &str, prompt: &str) -> String {
    let stdout = stdout.replace("\r\n", "\n");
    let prompt = prompt.replace("\r\n", "\n");
    if let Some(index) = stdout.rfind(&prompt) {
        return stdout[index + prompt.len()..].to_string();
    }

    let anchor = prompt
        .chars()
        .rev()
        .take(240)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    if !anchor.is_empty() {
        if let Some(index) = stdout.rfind(&anchor) {
            return stdout[index + anchor.len()..].to_string();
        }
    }
    if let Some(last_line) = prompt.lines().rev().find(|line| !line.trim().is_empty()) {
        if let Some(index) = stdout.rfind(last_line.trim()) {
            return stdout[index + last_line.trim().len()..].to_string();
        }
    }
    stdout
}

fn sanitize_diagnostic_text(raw: &str) -> String {
    let mut text = strip_ansi(raw);
    if let Ok(cwd) = std::env::current_dir() {
        text = redact_path(text, &cwd.display().to_string(), "<WORKING_DIRECTORY>");
    }
    for (name, value) in std::env::vars() {
        if value.len() >= 5
            && (name.contains("HOME")
                || name.contains("USER")
                || name.contains("PATH")
                || name == "AISH_LLAMA_CLI")
        {
            text = redact_path(text, &value, &format!("<{name}>"));
        }
    }
    text.chars().take(4096).collect()
}

fn redact_path(mut text: String, value: &str, replacement: &str) -> String {
    if value.is_empty() {
        return text;
    }
    text = text.replace(value, replacement);
    let slash = value.replace('\\', "/");
    let backslash = value.replace('/', "\\");
    text = text.replace(&slash, replacement);
    text.replace(&backslash, replacement)
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
    use super::{
        build_repair_prompt, build_semantic_plan_prompt_for, build_semantic_plan_system_prompt,
        is_controlled_stop, sanitize_diagnostic_text, select_model_text, select_structured_mode,
        validate_shell_command_dialect, validate_shell_command_dialect_for,
    };
    use crate::structured_output::StructuredOutputMode;

    #[test]
    fn uses_stderr_only_when_stdout_is_empty() {
        assert_eq!(select_model_text(" plan ", "noise", "prompt"), "plan");
        assert_eq!(select_model_text("", " response ", "prompt"), "response");
    }

    #[test]
    fn isolates_generated_text_from_an_echoed_prompt() {
        let prompt = r#"System instructions:
Use {"kind":"clarification","message":"one concise question"}.
Return JSON."#;
        let stdout = format!(
            "runtime banner\r\n> {}\r\n{{\"kind\":\"shell_command\",\"payload\":\"Get-ChildItem -Force\"}}\r\nExiting...",
            prompt.replace('\n', "\r\n")
        );
        let selected = select_model_text(&stdout, "", prompt);
        assert!(!selected.contains("one concise question"));
        assert!(selected.contains("Get-ChildItem -Force"));
    }

    #[test]
    fn isolates_generation_when_terminal_rendering_changes_the_prompt_body() {
        let prompt = "System instructions:\nlong original body\nReturn JSON only.";
        let stdout = "banner\n> System instructions:\nrendered differently\nReturn JSON only.\n{\"kind\":\"answer\",\"message\":\"done\"}";
        let selected = select_model_text(stdout, "", prompt);
        assert_eq!(selected, r#"{"kind":"answer","message":"done"}"#);
    }

    #[test]
    fn validates_profile_strategy_against_runtime() {
        assert_eq!(
            select_structured_mode("grammar", StructuredOutputMode::JsonSchema, true, true)
                .unwrap(),
            StructuredOutputMode::Grammar
        );
        assert!(
            select_structured_mode("grammar", StructuredOutputMode::JsonSchema, true, false)
                .is_err()
        );
    }

    #[test]
    fn diagnostics_redact_local_paths_and_bound_output() {
        let cwd = std::env::current_dir().unwrap().display().to_string();
        let raw = format!("model: {cwd}\\models\\fixture.gguf\n{}", "x".repeat(5000));
        let sanitized = sanitize_diagnostic_text(&raw);
        assert!(!sanitized.contains(&cwd));
        assert!(sanitized.contains("<WORKING_DIRECTORY>"));
        assert!(sanitized.chars().count() <= 4096);
    }

    #[test]
    fn only_accepts_code_130_for_a_complete_configured_stop() {
        let stops = vec!["<|im_start|>".to_string()];
        assert!(is_controlled_stop(
            Some(130),
            r#"{"kind":"answer","message":"done"}"#,
            &stops,
            None
        ));
        assert!(!is_controlled_stop(Some(130), "", &stops, None));
        assert!(!is_controlled_stop(
            Some(130),
            r#"{"kind":"answer"}"#,
            &[],
            None
        ));
        assert!(!is_controlled_stop(
            Some(130),
            r#"{"kind":"answer"}"#,
            &stops,
            Some("fatal")
        ));
    }

    #[test]
    fn rejects_cmd_switches_for_powershell_without_rejecting_posix_commands() {
        assert!(validate_shell_command_dialect_for("dir /a", "powershell").is_err());
        assert!(
            validate_shell_command_dialect_for("cmd.exe /d /s /c 'dir /a /s'", "powershell")
                .is_ok()
        );
        assert!(validate_shell_command_dialect_for(
            "Get-ChildItem | Where-Object { $*.Length -gt 10MB }",
            "powershell"
        )
        .is_err());
        assert!(validate_shell_command_dialect_for("ls /tmp", "posix").is_ok());
        assert!(validate_shell_command_dialect_for("test -f package.json", "posix").is_ok());
        assert!(validate_shell_command_dialect_for("test -f package.json", "powershell").is_err());
        assert!(validate_shell_command_dialect_for("Get-ChildItem -Force", "powershell").is_ok());
        assert!(validate_shell_command_dialect_for("type nul > sample.txt", "powershell").is_err());
        assert!(validate_shell_command_dialect_for("copy nul sample.txt", "powershell").is_err());
        assert!(validate_shell_command_dialect_for("echo. > sample.txt", "powershell").is_err());
        assert!(validate_shell_command_dialect_for(
            "New-Item -ItemType File -Path sample.txt",
            "powershell"
        )
        .is_ok());
        assert!(validate_shell_command_dialect_for("ls -l | sort -h", "powershell").is_err());
        assert!(validate_shell_command_dialect_for("git -C . status", "powershell").is_ok());
        assert!(validate_shell_command_dialect_for(
            "Get-ChildItem | Where-Object { $_.Length -gt 10MB }",
            "powershell"
        )
        .is_ok());
        assert!(validate_shell_command_dialect_for(
            "Write-Output '$* is literal text'",
            "powershell"
        )
        .is_ok());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn public_shell_validator_uses_powershell_on_windows() {
        assert!(validate_shell_command_dialect(
            "Get-ChildItem | Where-Object { $*.Length -gt 10MB }"
        )
        .is_err());
    }

    #[test]
    fn system_prompt_supports_bounded_workflows_and_session_follow_ups() {
        let prompt = build_semantic_plan_system_prompt();
        assert!(prompt.contains("two to four ordered commands"));
        assert!(prompt.contains("recent session turns"));
        assert!(prompt.contains("host executes them separately and stops on failure"));
    }

    #[test]
    fn repair_prompt_keeps_the_relevant_tail_of_noisy_runtime_output() {
        let noisy = format!(
            "{}{}",
            "runtime banner ".repeat(200),
            r#"{"kind":"shell_command","payload":"Get-ChildItem"}"#
        );
        let prompt = build_repair_prompt("show files", &serde_json::json!({}), &noisy);
        assert!(prompt.contains(r#"{"kind":"shell_command","payload":"Get-ChildItem"}"#));
        assert!(!prompt.contains(&"runtime banner ".repeat(100)));
    }

    #[test]
    fn prompt_declares_the_actual_cross_platform_shell_family() {
        let powershell = build_semantic_plan_prompt_for(
            "show files",
            &serde_json::json!({}),
            "windows",
            "pwsh.exe",
        );
        assert!(powershell.contains("Shell family: powershell"));
        assert!(powershell.contains("Do not use CMD built-ins"));

        let zsh = build_semantic_plan_prompt_for(
            "show files",
            &serde_json::json!({}),
            "macos",
            "/bin/zsh",
        );
        assert!(zsh.contains("Shell family: posix"));
        assert!(zsh.contains("POSIX syntax suitable for bash or zsh"));
    }
}
