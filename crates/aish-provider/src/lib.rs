use aish_ai::{
    build_repair_prompt, build_semantic_plan_prompt, build_semantic_plan_system_prompt,
    parse_semantic_plan, run_gguf_model, validate_shell_command_dialect, ModelProfile,
    ModelRunRequest, ModelRunResult, SemanticPlan, SemanticPlanKind,
};
use aish_core::{AiSubmode, AppMode, CachePolicy, ContextLevel, RiskLevel};
use aish_safety::classify_risk;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

mod navigation;
pub use navigation::{
    infer_existing_target_from_request, resolve_navigation_target, NavigationResolution,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRequestType {
    Complete,
    AiSuggest,
    AiRun,
    RecordEvent,
    GetMode,
    SetMode,
    SetContextPolicy,
    ClearCache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub request_type: ProviderRequestType,
    pub surface: String,
    pub os: String,
    pub shell: String,
    pub mode: AppMode,
    pub ai_submode: Option<AiSubmode>,
    pub cwd: String,
    pub prefix: String,
    pub context_level: ContextLevel,
    pub cache_policy: CachePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub kind: String,
    pub command: String,
    pub display: String,
    pub description: String,
    pub source: String,
    pub score: f32,
    pub risk: RiskLevel,
    pub needs_confirmation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub items: Vec<CompletionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEvent {
    pub shell: String,
    pub os: String,
    pub cwd_hash: String,
    pub typed_prefix: Option<String>,
    pub command: String,
    pub source: String,
    pub accepted: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderInputMode {
    Normal,
    AiRun,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderContextMode {
    Off,
    Auto,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderPlanAction {
    ShellCommand,
    ChangeDirectory,
    ApprovalRequired,
    Fallback,
    Error,
    Noop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPlanRequest {
    pub mode: ProviderInputMode,
    pub surface: String,
    pub input: String,
    pub context_json: serde_json::Value,
    pub profile: Option<ModelProfile>,
    #[serde(default)]
    pub diagnostics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPlan {
    pub mode: ProviderInputMode,
    pub surface: String,
    pub action: ProviderPlanAction,
    pub intent: String,
    pub command: Option<String>,
    pub target: Option<String>,
    pub risk: RiskLevel,
    pub needs_approval: bool,
    pub reason: String,
    pub fallback_message: Option<String>,
    pub model_output: Option<String>,
    pub runtime: Option<String>,
    pub error: Option<String>,
    pub diagnostics: Option<PlannerDiagnostics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerDiagnostics {
    pub id: String,
    pub parser_strategy: String,
    pub runtime_arguments: String,
    pub exit_status: Option<i32>,
    pub parse_errors: Vec<String>,
    pub raw_stdout: String,
    pub raw_stderr: String,
    pub retry_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTraceEvent {
    pub level: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSessionCommand {
    pub intent: Option<String>,
    pub command: String,
    pub status: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSession {
    pub mode: ProviderInputMode,
    pub context_mode: ProviderContextMode,
    pub show_trace: bool,
    pub command_memory: Vec<ProviderSessionCommand>,
}

impl Default for ProviderSession {
    fn default() -> Self {
        Self {
            mode: ProviderInputMode::AiRun,
            context_mode: ProviderContextMode::Auto,
            show_trace: false,
            command_memory: Vec::new(),
        }
    }
}

impl ProviderSession {
    pub fn record_command(
        &mut self,
        intent: Option<&str>,
        command: &str,
        status: &str,
        reason: Option<&str>,
    ) {
        self.command_memory.push(ProviderSessionCommand {
            intent: intent.map(str::to_string),
            command: command.to_string(),
            status: status.to_string(),
            reason: reason.map(str::to_string),
        });
        if self.command_memory.len() > 24 {
            let overflow = self.command_memory.len() - 24;
            self.command_memory.drain(0..overflow);
        }
    }

    pub fn clear_context(&mut self) {
        self.command_memory.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub mode: ProviderInputMode,
    pub context_mode: ProviderContextMode,
    pub show_trace: bool,
    pub session_commands: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderControlAction {
    SetMode(ProviderInputMode),
    SetContextMode(ProviderContextMode),
    SetTrace(bool),
    ClearContext,
}

pub fn provider_status(session: &ProviderSession) -> ProviderStatus {
    ProviderStatus {
        mode: session.mode.clone(),
        context_mode: session.context_mode.clone(),
        show_trace: session.show_trace,
        session_commands: session.command_memory.len(),
    }
}

pub fn apply_provider_control(
    session: &mut ProviderSession,
    action: ProviderControlAction,
) -> ProviderStatus {
    match action {
        ProviderControlAction::SetMode(mode) => session.mode = mode,
        ProviderControlAction::SetContextMode(mode) => session.context_mode = mode,
        ProviderControlAction::SetTrace(value) => session.show_trace = value,
        ProviderControlAction::ClearContext => session.clear_context(),
    }

    provider_status(session)
}

pub fn plan_provider_input(request: ProviderPlanRequest) -> ProviderPlan {
    let input = request.input.trim().to_string();
    if input.is_empty() {
        return ProviderPlan {
            mode: request.mode,
            surface: request.surface,
            action: ProviderPlanAction::Noop,
            intent: String::new(),
            command: None,
            target: None,
            risk: RiskLevel::Low,
            needs_approval: false,
            reason: "No input supplied.".to_string(),
            fallback_message: None,
            model_output: None,
            runtime: None,
            error: None,
            diagnostics: None,
        };
    }

    match request.mode.clone() {
        ProviderInputMode::Normal => {
            plan_literal_command(&input, request.surface, ProviderInputMode::Normal)
        }
        ProviderInputMode::AiRun => plan_ai_run(&input, request),
    }
}

pub fn plan_literal_command(
    command: &str,
    surface: String,
    mode: ProviderInputMode,
) -> ProviderPlan {
    let local = classify_risk(command);
    ProviderPlan {
        mode,
        surface,
        action: ProviderPlanAction::ShellCommand,
        intent: command.to_string(),
        command: Some(command.to_string()),
        target: None,
        risk: local.risk,
        needs_approval: false,
        reason: "Normal mode command.".to_string(),
        fallback_message: None,
        model_output: None,
        runtime: None,
        error: None,
        diagnostics: None,
    }
}

pub fn plan_failed_command_recovery(
    command: &str,
    exit_code: Option<i32>,
    stderr: &str,
    surface: String,
    context_json: serde_json::Value,
    profile: Option<ModelProfile>,
    diagnostics: bool,
) -> ProviderPlan {
    let intent = "Diagnose the failed literal command. Return a shell_command only for a clear typo correction; otherwise return an answer explaining the failure.".to_string();
    let context_json = failed_command_context(context_json, command, exit_code, stderr);
    plan_provider_input(ProviderPlanRequest {
        mode: ProviderInputMode::AiRun,
        surface,
        input: intent,
        context_json,
        profile,
        diagnostics,
    })
}

fn failed_command_context(
    mut context: serde_json::Value,
    command: &str,
    exit_code: Option<i32>,
    stderr: &str,
) -> serde_json::Value {
    if !context.is_object() {
        context = serde_json::json!({ "base": context });
    }
    let bounded_stderr = stderr.chars().take(2000).collect::<String>();
    if let Some(object) = context.as_object_mut() {
        object.insert(
            "failed_command".to_string(),
            serde_json::json!({
                "command": command,
                "exit_code": exit_code,
                "stderr": bounded_stderr,
                "recovery_attempt": 1,
                "maximum_recovery_attempts": 1
            }),
        );
    }
    context
}

fn plan_ai_run(input: &str, request: ProviderPlanRequest) -> ProviderPlan {
    let Some(profile) = request.profile.clone() else {
        return ProviderPlan {
            mode: ProviderInputMode::AiRun,
            surface: request.surface,
            action: ProviderPlanAction::Error,
            intent: input.to_string(),
            command: None,
            target: None,
            risk: RiskLevel::Low,
            needs_approval: false,
            reason: "No model profile is available.".to_string(),
            fallback_message: None,
            model_output: None,
            runtime: None,
            error: Some("No model profile is available.".to_string()),
            diagnostics: None,
        };
    };
    plan_ai_run_with(input, request, profile, run_gguf_model)
}

fn plan_ai_run_with(
    input: &str,
    request: ProviderPlanRequest,
    profile: ModelProfile,
    runner: impl Fn(ModelRunRequest) -> Result<ModelRunResult, String>,
) -> ProviderPlan {
    let system_prompt = build_semantic_plan_system_prompt();
    let mut prompt = constrain_plan_prompt(
        build_semantic_plan_prompt(input, &request.context_json),
        input,
        &request.context_json,
    );
    let maximum_retries = profile.retry_count.min(1);
    let mut parse_errors = Vec::new();
    let mut last_result = None;
    let mut last_validation_error = None;

    for attempt in 0..=maximum_retries {
        let result = match runner(ModelRunRequest {
            profile: profile.clone(),
            system_prompt: system_prompt.clone(),
            prompt: prompt.clone(),
        }) {
            Ok(result) => result,
            Err(error) => return planner_runtime_error(input, request.surface, error),
        };
        if !result.ok {
            if attempt < maximum_retries
                && result.exit_code == Some(130)
                && result.raw_stderr.trim().is_empty()
            {
                parse_errors.push(
                    "runtime stopped with exit 130 before producing output; retrying once"
                        .to_string(),
                );
                prompt = constrain_plan_prompt(
                    build_repair_prompt(input, &request.context_json, &result.output),
                    input,
                    &request.context_json,
                );
                last_result = Some(result);
                continue;
            }
            let error = if result.error.trim().is_empty() {
                "The local model runtime failed. Run `/model status` for compatibility details."
                    .to_string()
            } else {
                result.error.clone()
            };
            let diagnostics = request.diagnostics.then(|| PlannerDiagnostics {
                id: diagnostic_id(input, &result.output),
                parser_strategy: "runtime_failure".to_string(),
                runtime_arguments: result.command_line.clone(),
                exit_status: result.exit_code,
                parse_errors: parse_errors.clone(),
                raw_stdout: result.raw_stdout.clone(),
                raw_stderr: result.raw_stderr.clone(),
                retry_count: attempt,
            });
            let runtime = request.diagnostics.then_some(result.command_line);
            if result.exit_code == Some(130) && result.raw_stderr.trim().is_empty() {
                let message = failed_command_evidence_message(&request.context_json)
                    .unwrap_or_else(|| planner_stop_fallback_message(input).to_string());
                return safe_fallback_plan(input, request.surface, message, runtime, diagnostics);
            }
            let mut plan = planner_runtime_error(input, request.surface, error);
            plan.runtime = runtime;
            plan.diagnostics = diagnostics;
            return plan;
        }
        match parse_semantic_plan(&result.output) {
            Ok(parsed) => {
                let mut plan = parsed.plan;
                ground_navigation_plan(&mut plan, input, &request.context_json);
                normalize_shell_plan_for_host(&mut plan, input);
                if let Err(error) = validate_model_plan(&plan, input, &request.context_json) {
                    parse_errors.push(format!("plan validation: {error}"));
                    let rejected_plan = serde_json::to_string(&plan)
                        .unwrap_or_else(|_| "<valid semantic plan>".to_string());
                    let rejected = format!("{rejected_plan}\nHost validation error: {error}");
                    prompt = constrain_plan_prompt(
                        build_repair_prompt(input, &request.context_json, &rejected),
                        input,
                        &request.context_json,
                    );
                    last_validation_error = Some(error);
                    last_result = Some(result);
                    continue;
                }
                let diagnostics = request.diagnostics.then(|| PlannerDiagnostics {
                    id: diagnostic_id(input, &result.output),
                    parser_strategy: parsed.strategy.to_string(),
                    runtime_arguments: result.command_line.clone(),
                    exit_status: result.exit_code,
                    parse_errors: parse_errors.clone(),
                    raw_stdout: result.raw_stdout.clone(),
                    raw_stderr: result.raw_stderr.clone(),
                    retry_count: attempt,
                });
                let model_output = None;
                let runtime = request.diagnostics.then_some(result.command_line);
                return semantic_to_provider_plan(
                    input,
                    plan,
                    request.surface,
                    &request.context_json,
                    model_output,
                    runtime,
                    diagnostics,
                );
            }
            Err(failure) => {
                parse_errors.extend(failure.errors);
                prompt = constrain_plan_prompt(
                    build_repair_prompt(input, &request.context_json, &result.output),
                    input,
                    &request.context_json,
                );
                last_result = Some(result);
            }
        }
    }

    let result = last_result.unwrap();
    let diagnostics = request.diagnostics.then(|| PlannerDiagnostics {
        id: diagnostic_id(input, &result.output),
        parser_strategy: "failed_after_repair".to_string(),
        runtime_arguments: result.command_line.clone(),
        exit_status: result.exit_code,
        parse_errors,
        raw_stdout: result.raw_stdout,
        raw_stderr: result.raw_stderr,
        retry_count: maximum_retries,
    });
    let fallback = validation_fallback(last_validation_error.as_deref());
    ProviderPlan {
        mode: ProviderInputMode::AiRun,
        surface: request.surface,
        action: ProviderPlanAction::Fallback,
        intent: input.to_string(),
        command: None,
        target: None,
        risk: RiskLevel::Low,
        needs_approval: false,
        reason: fallback.to_string(),
        fallback_message: Some(fallback.to_string()),
        model_output: None,
        runtime: request.diagnostics.then_some(result.command_line),
        error: None,
        diagnostics,
    }
}

fn constrain_plan_prompt(mut prompt: String, input: &str, context: &serde_json::Value) -> String {
    if context.get("failed_command").is_some() {
        prompt.push_str(
            "\n\nHost request class: failed-command recovery. Use the supplied failed command, exit code, and stderr as the evidence. Return shell_command only for a clear typo correction; otherwise return kind answer with a concise diagnosis of at most two sentences. Do not repeat the failed command, invent a cause, or include commands or step-by-step instructions in the answer.",
        );
    } else if is_explanation_request(input) && !has_unresolved_reference(input, context) {
        prompt.push_str(
            "\n\nHost request class: explanation. Return kind answer with a concise explanatory message. Do not return shell_command and do not inspect local state.",
        );
    }
    if mentions_file_object(input) {
        if request_has_recursive_scope(input) {
            prompt.push_str(
                "\n\nHost filesystem scope: recursive traversal is explicitly requested.",
            );
        } else {
            prompt.push_str(
                "\n\nHost filesystem scope: current directory only. Do not recurse or inspect subdirectories.",
            );
        }
    }
    prompt
}

fn normalize_shell_plan_for_host(plan: &mut SemanticPlan, input: &str) {
    if !cfg!(windows) || plan.kind != SemanticPlanKind::ShellCommand {
        return;
    }
    let Some(command) = plan.payload.take() else {
        return;
    };
    let command = normalize_powershell_environment_references(&command, input);
    if is_managed_cmd_command(&command) || !contains_cmd_slash_switch(&command) {
        plan.payload = Some(command);
        return;
    }
    let escaped = command.replace('\'', "''");
    plan.payload = Some(format!("cmd.exe /d /s /c '{escaped}'"));
}

fn normalize_powershell_environment_references(command: &str, input: &str) -> String {
    let mut result = String::with_capacity(command.len());
    let mut characters = command.chars().peekable();
    let mut single_quoted = false;
    while let Some(character) = characters.next() {
        if character == '\'' {
            single_quoted = !single_quoted;
            result.push(character);
            continue;
        }
        if character != '$' || single_quoted {
            result.push(character);
            continue;
        }
        let mut name = String::new();
        while characters
            .peek()
            .is_some_and(|next| next.is_ascii_alphanumeric() || *next == '_')
        {
            name.push(characters.next().unwrap());
        }
        let is_scoped = characters.peek() == Some(&':');
        let requested = input
            .split(|value: char| !value.is_ascii_alphanumeric() && value != '_')
            .any(|word| word.eq_ignore_ascii_case(&name));
        if !name.is_empty() && !is_scoped && requested && std::env::var_os(&name).is_some() {
            result.push_str("$env:");
        } else {
            result.push('$');
        }
        result.push_str(&name);
    }
    result
}

fn is_managed_cmd_command(command: &str) -> bool {
    let command = command.trim_start().to_ascii_lowercase();
    command.starts_with("cmd.exe /d /s /c '") || command.starts_with("cmd /d /s /c '")
}

fn contains_cmd_slash_switch(command: &str) -> bool {
    command
        .split_whitespace()
        .any(|token| token.len() > 1 && token.starts_with('/') && !token.contains(['\\', ':']))
}

fn ground_navigation_plan(plan: &mut SemanticPlan, input: &str, context: &serde_json::Value) {
    let cwd = context
        .get("cwd")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let Some(inferred) = infer_existing_target_from_request(input, &cwd) else {
        return;
    };
    if plan.kind == SemanticPlanKind::ShellCommand
        && plan
            .payload
            .as_deref()
            .is_some_and(is_navigation_shell_command)
    {
        plan.kind = SemanticPlanKind::ChangeDirectory;
        plan.payload = None;
        plan.target = Some(inferred);
        plan.scope = Some("current".to_string());
        return;
    }
    if plan.kind != SemanticPlanKind::ChangeDirectory {
        return;
    }
    if plan.target.as_deref().is_some_and(|target| {
        let target = target.trim().trim_matches(['\'', '"']);
        if matches!(target, "." | ".." | "~" | "%USERPROFILE%" | "$HOME") {
            return true;
        }
        let path = PathBuf::from(target);
        let rooted = if path.is_absolute() {
            path
        } else {
            cwd.join(path)
        };
        rooted.is_dir()
            && Path::new(target)
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| names_equal_on_platform(name, &inferred))
    }) {
        return;
    }
    plan.target = Some(inferred);
    plan.scope = Some("current".to_string());
}

fn is_navigation_shell_command(command: &str) -> bool {
    command.split_whitespace().next().is_some_and(|program| {
        matches!(
            program
                .trim_matches(['\'', '"'])
                .to_ascii_lowercase()
                .as_str(),
            "cd" | "chdir" | "set-location"
        )
    })
}

fn names_equal_on_platform(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

fn validate_model_plan(
    plan: &SemanticPlan,
    input: &str,
    context: &serde_json::Value,
) -> Result<(), String> {
    if is_explanation_request(input)
        && !has_unresolved_reference(input, context)
        && plan.kind != SemanticPlanKind::Answer
    {
        return Err(
            "A complete explanatory request must return an answer and must not run a command."
                .to_string(),
        );
    }
    match plan.kind {
        SemanticPlanKind::ShellCommand => {
            let command = plan.payload.as_deref().unwrap_or_default();
            validate_shell_command_dialect(command)?;
            if has_unresolved_reference(input, context) {
                return if is_navigation_shell_command(command) {
                    Err(
                        "A navigation request contains an unresolved reference; ask a clarification instead."
                            .to_string(),
                    )
                } else {
                    Err(
                        "The request contains an unresolved reference; ask a clarification instead."
                            .to_string(),
                    )
                };
            }
            if classify_risk(command).risk == RiskLevel::Low
                && command_references_nonexistent_absolute_path(command)
            {
                return Err(
                    "A read-only generated command must not reference an absolute path that does not exist."
                        .to_string(),
                );
            }
            if repeats_failed_command(command, context) {
                return Err(
                    "A recovery plan must not repeat the command that already failed; return an explanation or a corrected command."
                        .to_string(),
                );
            }
            if !is_clear_recovery_correction(command, context) {
                return Err(
                    "A recovery shell command must be a clear typo correction of the failed command; otherwise return an answer grounded in the reported error."
                        .to_string(),
                );
            }
            if command_has_recursive_flag(command) && !request_has_recursive_scope(input) {
                return Err(
                    "A command must not add recursive scope unless the user requested it."
                        .to_string(),
                );
            }
        }
        SemanticPlanKind::ChangeDirectory => {
            let target = plan.target.as_deref().unwrap_or_default();
            if mentions_file_object(input) && !mentions_directory_object(input) {
                return Err(
                    "A file-oriented request cannot be a change_directory action.".to_string(),
                );
            }
            if !navigation_target_is_grounded(target, input) {
                return Err(
                    "The directory target was not grounded in the user's request; preserve the requested target exactly."
                        .to_string(),
                );
            }
        }
        SemanticPlanKind::Answer => {
            if plan
                .message
                .as_deref()
                .is_some_and(|message| message.trim_end().ends_with('?'))
            {
                return Err(
                    "An answer must answer the request rather than restating it as a question."
                        .to_string(),
                );
            }
        }
        SemanticPlanKind::Clarification => {
            if is_explanation_request(input) && !has_unresolved_reference(input, context) {
                return Err(
                    "This is a complete explanatory question; return an answer instead of a clarification."
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

fn command_references_nonexistent_absolute_path(command: &str) -> bool {
    shell_like_tokens(command).iter().any(|token| {
        let token = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '\'' | '"' | '`' | ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}'
            )
        });
        if token.is_empty() || token.contains(['*', '?']) {
            return false;
        }
        let path = std::path::Path::new(token);
        path.is_absolute() && !path.exists()
    })
}

fn shell_like_tokens(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    for ch in command.chars() {
        if escaped {
            current.push(ch);
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
            ch if ch.is_whitespace() && !single_quoted && !double_quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            '|' | ';' | '&' if !single_quoted && !double_quoted => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn validation_fallback(error: Option<&str>) -> &'static str {
    match error {
        Some(error) if error.contains("navigation request") => {
            "Which existing directory should I open? Please provide its name or path."
        }
        Some(error) if error.contains("unresolved reference") => {
            "I need the exact target and requested new value before I can safely plan that change."
        }
        Some(error) if error.contains("explanatory request") => {
            "I could not generate a reliable explanation. Please rephrase the question with the command or error you want explained."
        }
        Some(error) if error.contains("directory target") => {
            "I could not identify a trustworthy action or existing directory target. Please provide the exact target or desired outcome."
        }
        Some(error) if error.contains("file-oriented request") => {
            "I could not generate a trustworthy file operation. Please rephrase the exact file and desired outcome."
        }
        Some(error) if error.contains("recursive scope") => {
            "I could not generate a trustworthy command with the requested directory scope. Please clarify whether subdirectories should be included."
        }
        Some(error) if error.contains("command that already failed") => {
            "The failed command was not changed, so I did not run it again. Review the reported error or provide a corrected command."
        }
        Some(error) if error.contains("clear typo correction") => {
            "The suggested command was not a clear typo correction, so I did not run it. Review the reported error before trying another command."
        }
        Some(error) if error.contains("absolute path") => {
            "The suggested command referenced a directory or file that does not exist, so I did not run it. Please provide the intended path if one is required."
        }
        _ => "I could not produce a safe executable plan from that request. Please rephrase it with the target or desired outcome.",
    }
}

fn failed_command_evidence_message(context: &serde_json::Value) -> Option<String> {
    let failed = context.get("failed_command")?;
    let exit_code = failed.get("exit_code").and_then(serde_json::Value::as_i64);
    let stderr = failed
        .get("stderr")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim();
    let status = exit_code
        .map(|code| format!(" with exit code {code}"))
        .unwrap_or_default();
    if stderr.is_empty() {
        Some(format!(
            "The command failed{status}, and no error details were reported. I could not produce a safe correction."
        ))
    } else {
        Some(format!("The command failed{status}: {stderr}"))
    }
}

fn safe_fallback_plan(
    input: &str,
    surface: String,
    message: String,
    runtime: Option<String>,
    diagnostics: Option<PlannerDiagnostics>,
) -> ProviderPlan {
    ProviderPlan {
        mode: ProviderInputMode::AiRun,
        surface,
        action: ProviderPlanAction::Fallback,
        intent: input.to_string(),
        command: None,
        target: None,
        risk: RiskLevel::Low,
        needs_approval: false,
        reason: message.clone(),
        fallback_message: Some(message),
        model_output: None,
        runtime,
        error: None,
        diagnostics,
    }
}

fn planner_stop_fallback_message(input: &str) -> &'static str {
    if is_explanation_request(input) {
        "I could not generate a concise reliable explanation. Please rephrase the question with the specific concept or error you want explained."
    } else {
        "I could not generate a safe executable plan. Please rephrase the request with the exact target and desired outcome."
    }
}

fn repeats_failed_command(command: &str, context: &serde_json::Value) -> bool {
    let Some(failed) = context
        .get("failed_command")
        .and_then(|value| value.get("command"))
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    normalized_command(command) == normalized_command(failed)
}

fn is_clear_recovery_correction(command: &str, context: &serde_json::Value) -> bool {
    let Some(failed) = context
        .get("failed_command")
        .and_then(|value| value.get("command"))
        .and_then(serde_json::Value::as_str)
    else {
        return true;
    };
    let command = normalized_command(command).to_ascii_lowercase();
    let failed = normalized_command(failed).to_ascii_lowercase();
    command != failed && edit_distance_with_limit(&command, &failed, 3).is_some()
}

fn edit_distance_with_limit(left: &str, right: &str, limit: usize) -> Option<usize> {
    let left = left.chars().collect::<Vec<_>>();
    let right = right.chars().collect::<Vec<_>>();
    if left.len().abs_diff(right.len()) > limit {
        return None;
    }
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (left_index, left_char) in left.iter().enumerate() {
        current[0] = left_index + 1;
        let mut row_minimum = current[0];
        for (right_index, right_char) in right.iter().enumerate() {
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + usize::from(left_char != right_char));
            row_minimum = row_minimum.min(current[right_index + 1]);
        }
        if row_minimum > limit {
            return None;
        }
        std::mem::swap(&mut previous, &mut current);
    }
    (previous[right.len()] <= limit).then_some(previous[right.len()])
}

fn normalized_command(command: &str) -> String {
    let normalized = command.split_whitespace().collect::<Vec<_>>().join(" ");
    if cfg!(windows) {
        normalized.to_ascii_lowercase()
    } else {
        normalized
    }
}

fn navigation_target_is_grounded(target: &str, input: &str) -> bool {
    let target = target.trim().trim_matches(['\'', '"']);
    if matches!(target, "." | ".." | "~") {
        return true;
    }
    let input = input.to_lowercase();
    let target_lower = target.to_lowercase();
    if input.contains(&target_lower) {
        return true;
    }
    if target_lower == "%userprofile%" || target_lower.starts_with("$home") {
        return input.contains("home");
    }
    if target_lower.len() >= 2 && target_lower.as_bytes()[1] == b':' {
        let drive = target_lower.chars().next().unwrap_or_default();
        if input.contains(&format!("{drive}:")) || input.contains(&format!("{drive} drive")) {
            return true;
        }
    }
    target_lower
        .rsplit(['/', '\\'])
        .find(|part| !part.is_empty())
        .is_some_and(|name| input.contains(name))
}

fn has_unresolved_reference(input: &str, context: &serde_json::Value) -> bool {
    let raw_words = input
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let words = raw_words
        .iter()
        .copied()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let cwd_available = context
        .get("cwd")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|cwd| !cwd.trim().is_empty())
        || std::env::current_dir().is_ok();

    for (index, word) in words.iter().enumerate() {
        if matches!(
            word.as_str(),
            "that" | "these" | "those" | "one" | "other" | "another"
        ) {
            return true;
        }
        if word == "it" && !has_explicit_antecedent(&raw_words, &words, index) {
            return true;
        }
        if word == "this" {
            let resolved_by_cwd = cwd_available
                && words.get(index + 1).is_some_and(|object| {
                    matches!(object.as_str(), "directory" | "folder" | "project")
                });
            if !resolved_by_cwd {
                return true;
            }
        }
        if word == "the"
            && words.get(index + 1).is_some_and(|object| {
                matches!(
                    object.as_str(),
                    "script"
                        | "config"
                        | "file"
                        | "folder"
                        | "directory"
                        | "command"
                        | "process"
                        | "permission"
                        | "permissions"
                )
            })
        {
            return true;
        }
    }
    false
}

fn has_explicit_antecedent(raw_words: &[&str], words: &[String], pronoun_index: usize) -> bool {
    raw_words.iter().take(pronoun_index).any(|word| {
        word.chars().any(|character| {
            character.is_ascii_digit() || matches!(character, '.' | '/' | '\\' | ':')
        }) || (word.len() > 1
            && word
                .chars()
                .all(|character| !character.is_alphabetic() || character.is_uppercase()))
    }) || words
        .iter()
        .take(pronoun_index)
        .any(|word| matches!(word.as_str(), "named" | "called"))
}

fn is_explanation_request(input: &str) -> bool {
    let words = input
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    words
        .first()
        .is_some_and(|word| matches!(word.as_str(), "why" | "what" | "how"))
        || words
            .iter()
            .any(|word| matches!(word.as_str(), "explain" | "describe"))
}

fn mentions_file_object(input: &str) -> bool {
    input
        .split(|ch: char| !ch.is_alphanumeric())
        .any(|word| matches!(word.to_ascii_lowercase().as_str(), "file" | "files"))
        || input.split_whitespace().any(token_looks_like_file_name)
}

fn token_looks_like_file_name(token: &str) -> bool {
    let token = token.trim_matches(|ch: char| {
        matches!(
            ch,
            '\'' | '"' | '`' | ',' | ';' | ':' | '(' | ')' | '[' | ']' | '{' | '}'
        )
    });
    let Some(extension) = std::path::Path::new(token)
        .extension()
        .and_then(|value| value.to_str())
    else {
        return false;
    };
    !extension.is_empty() && extension.len() <= 16 && extension.chars().any(|ch| ch.is_alphabetic())
}

fn mentions_directory_object(input: &str) -> bool {
    input.split(|ch: char| !ch.is_alphanumeric()).any(|word| {
        matches!(
            word.to_ascii_lowercase().as_str(),
            "folder" | "folders" | "directory" | "directories" | "path"
        )
    })
}

fn command_has_recursive_flag(command: &str) -> bool {
    let trimmed = command.trim();
    let command = trimmed
        .strip_prefix("cmd.exe /d /s /c '")
        .or_else(|| trimmed.strip_prefix("cmd /d /s /c '"))
        .and_then(|inner| inner.strip_suffix('\''))
        .unwrap_or(trimmed);
    command.split_whitespace().any(|token| {
        matches!(
            token
                .trim_matches(|ch: char| matches!(ch, '\'' | '"' | ',' | ';'))
                .to_ascii_lowercase()
                .as_str(),
            "-recurse" | "--recursive" | "/s"
        )
    })
}

fn request_has_recursive_scope(input: &str) -> bool {
    input.split(|ch: char| !ch.is_alphanumeric()).any(|word| {
        matches!(
            word.to_ascii_lowercase().as_str(),
            "recursive"
                | "recursively"
                | "subdirectory"
                | "subdirectories"
                | "tree"
                | "project"
                | "below"
                | "under"
        )
    })
}

fn semantic_to_provider_plan(
    input: &str,
    plan: SemanticPlan,
    surface: String,
    context: &serde_json::Value,
    model_output: Option<String>,
    runtime: Option<String>,
    diagnostics: Option<PlannerDiagnostics>,
) -> ProviderPlan {
    match plan.kind {
        SemanticPlanKind::ShellCommand => {
            let command = plan.payload.unwrap_or_default();
            let mut result = evaluate_generated_command(
                input,
                &command,
                None,
                Some("Generated command validated by the host."),
                surface,
                model_output,
                runtime,
            );
            result.diagnostics = diagnostics;
            result
        }
        SemanticPlanKind::ChangeDirectory => {
            let target = plan.target.unwrap_or_default();
            let cwd = context
                .get("cwd")
                .and_then(serde_json::Value::as_str)
                .map(PathBuf::from)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));
            let home = home_dir();
            match resolve_navigation_target(&target, plan.scope.as_deref(), input, &cwd, &home) {
                NavigationResolution::Resolved(path) => ProviderPlan {
                    mode: ProviderInputMode::AiRun,
                    surface,
                    action: ProviderPlanAction::ChangeDirectory,
                    intent: input.to_string(),
                    command: None,
                    target: Some(path.display().to_string()),
                    risk: RiskLevel::Low,
                    needs_approval: false,
                    reason: "Resolved an existing directory without executing shell text."
                        .to_string(),
                    fallback_message: None,
                    model_output,
                    runtime,
                    error: None,
                    diagnostics,
                },
                NavigationResolution::Ambiguous(paths) => {
                    let choices = paths
                        .iter()
                        .take(5)
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    response_plan(input, surface, format!("I found multiple matching directories: {choices}. Which one should I open?"), model_output, runtime, diagnostics)
                }
                NavigationResolution::Missing(message) => {
                    response_plan(input, surface, message, model_output, runtime, diagnostics)
                }
            }
        }
        SemanticPlanKind::Answer | SemanticPlanKind::Clarification => {
            let message = plan
                .message
                .unwrap_or_else(|| "Please clarify the request.".to_string());
            response_plan(
                input,
                surface,
                grounded_recovery_answer(&message, context),
                model_output,
                runtime,
                diagnostics,
            )
        }
    }
}

fn grounded_recovery_answer(message: &str, context: &serde_json::Value) -> String {
    if context.get("failed_command").is_none() {
        return message.to_string();
    }
    let lower = message.to_ascii_lowercase();
    let speculative_at = [
        " likely ",
        " probably ",
        " perhaps ",
        " may be ",
        " might be ",
        " could be ",
    ]
    .iter()
    .filter_map(|marker| lower.find(marker))
    .min();
    let Some(speculative_at) = speculative_at else {
        return message.to_string();
    };
    let supported = message[..speculative_at]
        .rfind(['.', '!', '?'])
        .map(|index| message[..=index].trim());
    supported
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| failed_command_evidence_message(context))
        .unwrap_or_else(|| message.to_string())
}

fn response_plan(
    input: &str,
    surface: String,
    message: String,
    model_output: Option<String>,
    runtime: Option<String>,
    diagnostics: Option<PlannerDiagnostics>,
) -> ProviderPlan {
    ProviderPlan {
        mode: ProviderInputMode::AiRun,
        surface,
        action: ProviderPlanAction::Fallback,
        intent: input.to_string(),
        command: None,
        target: None,
        risk: RiskLevel::Low,
        needs_approval: false,
        reason: message.clone(),
        fallback_message: Some(message),
        model_output,
        runtime,
        error: None,
        diagnostics,
    }
}

fn planner_runtime_error(input: &str, surface: String, error: String) -> ProviderPlan {
    ProviderPlan {
        mode: ProviderInputMode::AiRun,
        surface,
        action: ProviderPlanAction::Error,
        intent: input.to_string(),
        command: None,
        target: None,
        risk: RiskLevel::Low,
        needs_approval: false,
        reason: error.clone(),
        fallback_message: None,
        model_output: None,
        runtime: None,
        error: Some(error),
        diagnostics: None,
    }
}

fn diagnostic_id(input: &str, output: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    output.hash(&mut hasher);
    format!("planner-{:08x}", hasher.finish() as u32)
}

pub fn evaluate_generated_command(
    intent: &str,
    command: &str,
    model_risk: Option<&str>,
    model_reason: Option<&str>,
    surface: String,
    model_output: Option<String>,
    runtime: Option<String>,
) -> ProviderPlan {
    let local = classify_risk(command);
    let model = parse_model_risk(model_risk);
    let risk = combine_risk(&local.risk, &model);
    let model_high = matches!(model, RiskLevel::High);
    let needs_approval =
        local.needs_confirmation || matches!(local.risk, RiskLevel::High) || model_high;
    let reason = if local.needs_confirmation || matches!(local.risk, RiskLevel::High) {
        local.reason.clone()
    } else {
        model_reason
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&local.reason)
            .to_string()
    };

    ProviderPlan {
        mode: ProviderInputMode::AiRun,
        surface,
        action: if needs_approval {
            ProviderPlanAction::ApprovalRequired
        } else {
            ProviderPlanAction::ShellCommand
        },
        intent: intent.to_string(),
        command: Some(command.to_string()),
        target: None,
        risk,
        needs_approval,
        reason,
        fallback_message: None,
        model_output,
        runtime,
        error: None,
        diagnostics: None,
    }
}

pub fn trace_provider_plan(plan: &ProviderPlan) -> Vec<ProviderTraceEvent> {
    let mut events = vec![
        trace_event("info", "mode", describe_provider_mode(&plan.mode)),
        trace_event("info", "action", &format!("{:?}", plan.action)),
        trace_event("info", "request", &plan.intent),
        trace_event("info", "risk", risk_label(&plan.risk)),
        trace_event("info", "reason", &plan.reason),
    ];

    if let Some(command) = &plan.command {
        events.push(trace_event("info", "shell", command));
    }
    if let Some(target) = &plan.target {
        events.push(trace_event("info", "directory", target));
    }
    if let Some(runtime) = &plan.runtime {
        events.push(trace_event("debug", "runtime", runtime));
    }
    if let Some(model_output) = &plan.model_output {
        events.push(trace_event("debug", "model_card", model_output));
    }
    if let Some(error) = &plan.error {
        events.push(trace_event("error", "error", error));
    }
    if let Some(diagnostics) = &plan.diagnostics {
        events.push(trace_event("debug", "diagnostic_id", &diagnostics.id));
        events.push(trace_event("debug", "parser", &diagnostics.parser_strategy));
        events.push(trace_event(
            "debug",
            "runtime_args",
            &diagnostics.runtime_arguments,
        ));
        events.push(trace_event(
            "debug",
            "exit_status",
            &format!("{:?}", diagnostics.exit_status),
        ));
        events.push(trace_event(
            "debug",
            "parse_errors",
            &diagnostics.parse_errors.join(" | "),
        ));
        events.push(trace_event("debug", "raw_stdout", &diagnostics.raw_stdout));
        events.push(trace_event("debug", "raw_stderr", &diagnostics.raw_stderr));
    }

    events
}

fn trace_event(level: &str, key: &str, value: &str) -> ProviderTraceEvent {
    ProviderTraceEvent {
        level: level.to_string(),
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn risk_label(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "low",
        RiskLevel::Medium => "medium",
        RiskLevel::High => "high",
    }
}

pub fn build_provider_context(
    mut base: serde_json::Value,
    session: &ProviderSession,
) -> serde_json::Value {
    if session.context_mode == ProviderContextMode::Off {
        return serde_json::json!({ "context_mode": describe_context_mode(&session.context_mode) });
    }
    if !base.is_object() {
        base = serde_json::json!({ "base": base });
    }
    if let Some(object) = base.as_object_mut() {
        object.insert(
            "context_mode".to_string(),
            serde_json::json!(describe_context_mode(&session.context_mode)),
        );
        object.insert(
            "session_commands".to_string(),
            serde_json::to_value(&session.command_memory).unwrap_or_else(|_| serde_json::json!([])),
        );
        object.insert(
            "agent_context_allowed".to_string(),
            serde_json::json!(session.context_mode == ProviderContextMode::Agent),
        );
    }
    base
}

pub fn parse_provider_mode(value: &str) -> Option<ProviderInputMode> {
    match value.to_lowercase().as_str() {
        "normal" | "shell" | "off" => Some(ProviderInputMode::Normal),
        "ai" | "ai_run" | "run" | "ken" => Some(ProviderInputMode::AiRun),
        _ => None,
    }
}

pub fn describe_provider_mode(mode: &ProviderInputMode) -> &'static str {
    match mode {
        ProviderInputMode::Normal => "normal",
        ProviderInputMode::AiRun => "ai_run",
    }
}

pub fn parse_context_mode(value: &str) -> Option<ProviderContextMode> {
    match value.to_lowercase().as_str() {
        "off" | "none" | "manual" => Some(ProviderContextMode::Off),
        "on" | "auto" => Some(ProviderContextMode::Auto),
        "agent" | "agent_mode" => Some(ProviderContextMode::Agent),
        _ => None,
    }
}

pub fn describe_context_mode(mode: &ProviderContextMode) -> &'static str {
    match mode {
        ProviderContextMode::Off => "off",
        ProviderContextMode::Auto => "auto",
        ProviderContextMode::Agent => "agent",
    }
}

pub fn default_model_profile() -> ModelProfile {
    let model_path = std::env::var("AISH_MODEL_PATH").unwrap_or_default();
    let llama_cli_path =
        std::env::var("AISH_LLAMA_CLI").unwrap_or_else(|_| "llama-cli".to_string());
    ModelProfile {
        id: "configured-local-model".to_string(),
        label: "Configured local GGUF model".to_string(),
        family: "generic-gguf".to_string(),
        model_path,
        llama_cli_path,
        context_tokens: 4096,
        max_tokens: 192,
        temperature: 0.0,
        structured_output_strategy: "auto".to_string(),
        chat_template: None,
        use_system_prompt: false,
        retry_count: 1,
        stop_sequences: Vec::new(),
        timeout_seconds: 60,
    }
}

fn parse_model_risk(value: Option<&str>) -> RiskLevel {
    match value.unwrap_or("low").to_lowercase().as_str() {
        "high" => RiskLevel::High,
        "medium" => RiskLevel::Medium,
        _ => RiskLevel::Low,
    }
}

fn combine_risk(local: &RiskLevel, model: &RiskLevel) -> RiskLevel {
    if matches!(local, RiskLevel::High) || matches!(model, RiskLevel::High) {
        RiskLevel::High
    } else if matches!(local, RiskLevel::Medium) || matches!(model, RiskLevel::Medium) {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

fn home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod planner_tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn profile() -> ModelProfile {
        ModelProfile {
            model_path: "fixture.gguf".to_string(),
            llama_cli_path: "fixture-llama-cli".to_string(),
            retry_count: 1,
            ..ModelProfile::default()
        }
    }

    fn request(context_json: serde_json::Value) -> ProviderPlanRequest {
        ProviderPlanRequest {
            mode: ProviderInputMode::AiRun,
            surface: "test".to_string(),
            input: "fixture intent".to_string(),
            context_json,
            profile: Some(profile()),
            diagnostics: true,
        }
    }

    fn result(output: &str) -> ModelRunResult {
        ModelRunResult {
            ok: true,
            command_line: "llama-cli <sanitized>".to_string(),
            output: output.to_string(),
            error: String::new(),
            raw_stdout: output.to_string(),
            raw_stderr: String::new(),
            exit_code: Some(0),
            structured_output: "json_schema".to_string(),
        }
    }

    fn plan_from(outputs: &[&str], context: serde_json::Value) -> ProviderPlan {
        plan_from_input("fixture intent", outputs, context)
    }

    fn plan_from_input(input: &str, outputs: &[&str], context: serde_json::Value) -> ProviderPlan {
        let outputs = RefCell::new(
            outputs
                .iter()
                .map(|output| result(output))
                .collect::<VecDeque<_>>(),
        );
        let request = request(context);
        plan_ai_run_with(input, request, profile(), |_| {
            outputs
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| "fixture output exhausted".to_string())
        })
    }

    #[test]
    fn converts_clean_commands_and_keeps_risk_host_controlled() {
        let low = plan_from(
            &[r#"{"kind":"shell_command","payload":"git status"}"#],
            serde_json::json!({}),
        );
        assert_eq!(low.action, ProviderPlanAction::ShellCommand);
        assert_eq!(low.command.as_deref(), Some("git status"));
        assert!(!low.needs_approval);

        let risky = plan_from(
            &[r#"{"kind":"shell_command","payload":"Remove-Item result.txt"}"#],
            serde_json::json!({}),
        );
        assert_eq!(risky.action, ProviderPlanAction::ApprovalRequired);
        assert!(risky.needs_approval);
        assert_eq!(risky.risk, RiskLevel::High);
    }

    #[test]
    fn ordinary_plans_do_not_retain_raw_model_or_runtime_output() {
        let mut request = request(serde_json::json!({}));
        request.diagnostics = false;
        let plan = plan_ai_run_with("fixture intent", request, profile(), |_| {
            Ok(result(r#"{"kind":"shell_command","payload":"git status"}"#))
        });
        assert!(plan.model_output.is_none());
        assert!(plan.runtime.is_none());
        assert!(plan.diagnostics.is_none());
    }

    #[test]
    fn accepts_answers_clarifications_and_wrapped_json_without_execution() {
        for output in [
            "```json\n{\"kind\":\"answer\",\"message\":\"The prior command used an unknown flag.\"}\n```",
            r#"{"kind":"clarification","message":"Which file should I rename?"}"#,
        ] {
            let plan = plan_from(&[output], serde_json::json!({}));
            assert_eq!(plan.action, ProviderPlanAction::Fallback);
            assert!(plan.command.is_none());
            assert!(plan.fallback_message.is_some());
        }
    }

    #[test]
    fn retries_once_after_truncation_and_records_parser_diagnostics() {
        let plan = plan_from(
            &[
                r#"{"kind":"shell_command","payload":"Get-ChildItem"#,
                r#"{"kind":"shell_command","payload":"Get-ChildItem"}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::ShellCommand);
        let diagnostics = plan.diagnostics.expect("diagnostics");
        assert_eq!(diagnostics.retry_count, 1);
        assert!(!diagnostics.parse_errors.is_empty());
    }

    #[test]
    fn retries_once_after_an_empty_controlled_runtime_stop() {
        let outputs = RefCell::new(VecDeque::from([
            ModelRunResult {
                ok: false,
                command_line: "llama-cli <sanitized>".to_string(),
                output: "runtime banner without a semantic plan".to_string(),
                error: "Local model runtime exited with 130: ".to_string(),
                raw_stdout: "runtime banner without a semantic plan".to_string(),
                raw_stderr: String::new(),
                exit_code: Some(130),
                structured_output: "grammar".to_string(),
            },
            result(r#"{"kind":"shell_command","payload":"Get-ChildItem Env:PATH"}"#),
        ]));
        let prompts = RefCell::new(Vec::new());
        let plan = plan_ai_run_with(
            "display the PATH without changing it",
            request(serde_json::json!({})),
            profile(),
            |request| {
                prompts.borrow_mut().push(request.prompt);
                outputs
                    .borrow_mut()
                    .pop_front()
                    .ok_or_else(|| "fixture output exhausted".to_string())
            },
        );
        assert_eq!(plan.action, ProviderPlanAction::ShellCommand);
        assert_eq!(plan.diagnostics.as_ref().unwrap().retry_count, 1);
        assert!(plan
            .diagnostics
            .as_ref()
            .unwrap()
            .parse_errors
            .iter()
            .any(|error| error.contains("exit 130")));
        let prompts = prompts.into_inner();
        assert_eq!(prompts.len(), 2);
        assert!(prompts[1].contains("previous response was invalid or incomplete"));
    }

    #[test]
    fn grounds_semantically_invalid_navigation_without_exposing_an_invented_path() {
        let root = std::env::temp_dir().join(format!(
            "aish-grounded-plan-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let target = root.join("orbit-zone");
        fs::create_dir_all(&target).unwrap();
        let plan = plan_from_input(
            "go to orbit-zone",
            &[
                r#"{"kind":"change_directory","target":"C:\\FixtureRoot\\NewFolder"}"#,
                r#"{"kind":"change_directory","target":"orbit-zone"}"#,
            ],
            serde_json::json!({ "cwd": root }),
        );
        assert_eq!(plan.action, ProviderPlanAction::ChangeDirectory);
        assert_eq!(
            plan.target.as_deref(),
            Some(
                fs::canonicalize(&target)
                    .unwrap()
                    .to_string_lossy()
                    .as_ref()
            )
        );
        assert_eq!(plan.diagnostics.as_ref().unwrap().retry_count, 0);
        assert!(plan.diagnostics.as_ref().unwrap().parse_errors.is_empty());
        assert!(!plan
            .target
            .as_deref()
            .unwrap_or_default()
            .contains("FixtureRoot"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unresolved_state_change_is_repaired_to_a_clarification() {
        let plan = plan_from_input(
            "rename this file",
            &[
                r#"{"kind":"shell_command","payload":"Rename-Item package.json renamed.json"}"#,
                r#"{"kind":"clarification","message":"Which file should I rename, and what should its new name be?"}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert_eq!(plan.diagnostics.as_ref().unwrap().retry_count, 1);
    }

    #[test]
    fn question_shaped_answer_is_repaired_to_an_explanation() {
        let plan = plan_from_input(
            "explain why access can be denied",
            &[
                r#"{"kind":"answer","message":"Why can access be denied?"}"#,
                r#"{"kind":"answer","message":"Access can be denied when permissions do not allow the requested operation."}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan
            .fallback_message
            .as_deref()
            .is_some_and(|message| message.starts_with("Access can")));
        assert_eq!(plan.diagnostics.as_ref().unwrap().retry_count, 1);
    }

    #[test]
    fn explanatory_request_cannot_become_an_unrelated_command() {
        let plan = plan_from_input(
            "explain why access can be denied",
            &[
                r#"{"kind":"shell_command","payload":"Get-ChildItem -Force"}"#,
                r#"{"kind":"answer","message":"Access can be denied when permissions do not allow the operation."}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert!(plan
            .fallback_message
            .as_deref()
            .is_some_and(|message| message.starts_with("Access can")));
    }

    #[test]
    fn explanatory_constraint_is_present_on_initial_and_repair_prompts() {
        let outputs = RefCell::new(VecDeque::from([
            result(r#"{"kind":"shell_command","payload":"Get-ExecutionPolicy"}"#),
            result(
                r#"{"kind":"answer","message":"Access can be denied because permissions, ownership, policy, or elevation do not allow the operation."}"#,
            ),
        ]));
        let prompts = RefCell::new(Vec::new());
        let plan = plan_ai_run_with(
            "why would a command return access denied?",
            request(serde_json::json!({})),
            profile(),
            |model_request| {
                prompts.borrow_mut().push(model_request.prompt);
                outputs
                    .borrow_mut()
                    .pop_front()
                    .ok_or_else(|| "fixture output exhausted".to_string())
            },
        );

        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        let prompts = prompts.into_inner();
        assert_eq!(prompts.len(), 2);
        assert!(prompts.iter().all(|prompt| prompt.contains(
            "Host request class: explanation. Return kind answer with a concise explanatory message."
        )));
    }

    #[test]
    fn file_inspection_cannot_become_directory_navigation() {
        let plan = plan_from_input(
            "show hidden files here",
            &[
                r#"{"kind":"change_directory","target":"hidden"}"#,
                r#"{"kind":"shell_command","payload":"Get-ChildItem -Force"}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::ShellCommand);
        assert_eq!(plan.command.as_deref(), Some("Get-ChildItem -Force"));
    }

    #[test]
    fn file_name_move_cannot_become_directory_navigation() {
        let plan = plan_from_input(
            "move note-482.txt into Orbit-482",
            &[
                r#"{"kind":"change_directory","target":"Orbit-482"}"#,
                r#"{"kind":"change_directory","target":"Orbit-482"}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert!(plan
            .fallback_message
            .as_deref()
            .is_some_and(|message| message.contains("file operation")));
    }

    #[test]
    fn nonexistent_absolute_command_path_is_repaired() {
        let missing = std::env::temp_dir()
            .join(format!(
                "aish-missing-command-path-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("directory with spaces");
        let invalid = serde_json::json!({
            "kind": "shell_command",
            "payload": format!("Get-ChildItem -Path '{}'", missing.display())
        })
        .to_string();
        let plan = plan_from_input(
            "show the installed compiler version",
            &[
                invalid.as_str(),
                r#"{"kind":"shell_command","payload":"rustc --version"}"#,
            ],
            serde_json::json!({}),
        );

        assert_eq!(plan.action, ProviderPlanAction::ShellCommand);
        assert_eq!(plan.command.as_deref(), Some("rustc --version"));
        assert_eq!(plan.diagnostics.as_ref().unwrap().retry_count, 1);
    }

    #[test]
    fn nonexistent_mutation_target_remains_approval_gated() {
        let missing = std::env::temp_dir()
            .join(format!(
                "aish-missing-mutation-target-{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("new file.txt");
        let generated = serde_json::json!({
            "kind": "shell_command",
            "payload": format!("New-Item -Path '{}' -ItemType File", missing.display())
        })
        .to_string();
        let plan = plan_from_input(
            "create an empty file named new file.txt",
            &[generated.as_str()],
            serde_json::json!({}),
        );

        assert_eq!(plan.action, ProviderPlanAction::ApprovalRequired);
        assert!(plan.needs_approval);
        assert_eq!(plan.risk, RiskLevel::Medium);
        assert!(plan.command.is_some());
    }

    #[test]
    fn current_directory_request_cannot_gain_recursive_scope() {
        let plan = plan_from_input(
            "show hidden files here",
            &[
                r#"{"kind":"shell_command","payload":"Get-ChildItem -Recurse -Force"}"#,
                r#"{"kind":"shell_command","payload":"Get-ChildItem -Force"}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::ShellCommand);
        assert_eq!(plan.command.as_deref(), Some("Get-ChildItem -Force"));
        assert_eq!(plan.diagnostics.as_ref().unwrap().retry_count, 1);
    }

    #[test]
    fn cmd_recursion_is_repaired_unless_the_request_explicitly_includes_it() {
        let repaired = plan_from_input(
            "list files sorted by size",
            &[
                r#"{"kind":"shell_command","payload":"dir /b /s /o-s"}"#,
                r#"{"kind":"shell_command","payload":"Get-ChildItem -File | Sort-Object Length -Descending"}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(repaired.action, ProviderPlanAction::ShellCommand);
        assert_eq!(
            repaired.command.as_deref(),
            Some("Get-ChildItem -File | Sort-Object Length -Descending")
        );
        assert_eq!(repaired.diagnostics.as_ref().unwrap().retry_count, 1);

        let recursive = plan_from_input(
            "find every package.json below here",
            &[
                r#"{"kind":"shell_command","payload":"Get-ChildItem -Recurse -Filter package.json"}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(recursive.action, ProviderPlanAction::ShellCommand);
        assert_eq!(recursive.diagnostics.as_ref().unwrap().retry_count, 0);
    }

    #[test]
    fn file_request_prompt_declares_whether_recursion_is_allowed() {
        let local = constrain_plan_prompt(
            "fixture prompt".to_string(),
            "list files sorted by size",
            &serde_json::json!({}),
        );
        assert!(local.contains("current directory only"));
        assert!(local.contains("Do not recurse"));

        let recursive = constrain_plan_prompt(
            "fixture prompt".to_string(),
            "find files recursively below here",
            &serde_json::json!({}),
        );
        assert!(recursive.contains("recursive traversal is explicitly requested"));
        assert!(!recursive.contains("Do not recurse"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn malformed_powershell_variable_is_repaired_before_exposure() {
        let plan = plan_from_input(
            "find large files in this project",
            &[
                r#"{"kind":"shell_command","payload":"Get-ChildItem -Recurse -File | Where-Object { $*.Length -gt 10MB }"}"#,
                r#"{"kind":"shell_command","payload":"Get-ChildItem -Recurse -File | Where-Object { $_.Length -gt 10MB }"}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::ShellCommand);
        assert!(plan
            .command
            .as_deref()
            .is_some_and(|command| command.contains("$_.Length")));
        assert_eq!(plan.diagnostics.as_ref().unwrap().retry_count, 1);
        assert!(plan
            .diagnostics
            .as_ref()
            .unwrap()
            .parse_errors
            .iter()
            .any(|error| error.contains("wildcard-named")));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn cmd_style_switches_use_a_host_managed_shell_adapter() {
        let low = plan_from_input(
            "show hidden files here",
            &[r#"{"kind":"shell_command","payload":"dir /a"}"#],
            serde_json::json!({}),
        );
        assert_eq!(low.action, ProviderPlanAction::ShellCommand);
        assert_eq!(low.command.as_deref(), Some("cmd.exe /d /s /c 'dir /a'"));
        assert!(!low.needs_approval);

        let risky = plan_from_input(
            "delete the named file",
            &[r#"{"kind":"shell_command","payload":"del /q important.txt"}"#],
            serde_json::json!({}),
        );
        assert_eq!(risky.action, ProviderPlanAction::ApprovalRequired);
        assert_eq!(risky.risk, RiskLevel::High);
        assert!(risky.needs_approval);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn requested_environment_variables_use_powershell_environment_scope() {
        let plan = plan_from_input(
            "display the PATH without changing it",
            &[r#"{"kind":"shell_command","payload":"echo $PATH"}"#],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::ShellCommand);
        assert_eq!(plan.command.as_deref(), Some("echo $env:PATH"));
        assert!(!plan.needs_approval);
        assert_eq!(
            normalize_powershell_environment_references(
                "Where-Object { $_.Name -eq $PSVersionTable }",
                "show the PowerShell version"
            ),
            "Where-Object { $_.Name -eq $PSVersionTable }"
        );
    }

    #[test]
    fn repeated_unresolved_mutation_returns_a_targeted_clarification() {
        let plan = plan_from_input(
            "rename this file",
            &[
                r#"{"kind":"shell_command","payload":"Rename-Item first.txt second.txt"}"#,
                r#"{"kind":"shell_command","payload":"Rename-Item source.txt target.txt"}"#,
            ],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert!(plan.reason.contains("exact target"));
        assert!(plan.model_output.is_none());
    }

    #[test]
    fn returns_safe_user_language_when_both_attempts_are_prose() {
        let plan = plan_from(
            &["I would run a command.", "Still not JSON."],
            serde_json::json!({}),
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert!(!plan.reason.contains("command card"));
        assert!(!plan.reason.contains("schema"));
    }

    #[test]
    fn resolves_an_arbitrary_existing_navigation_target() {
        let root = std::env::temp_dir().join(format!(
            "aish-provider-plan-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let target = root.join("Nebula Work Area");
        fs::create_dir_all(&target).unwrap();
        let plan = plan_from_input(
            "go to Nebula Work Area",
            &[r#"{"kind":"change_directory","target":"Nebula Work Area"}"#],
            serde_json::json!({ "cwd": root }),
        );
        assert_eq!(plan.action, ProviderPlanAction::ChangeDirectory);
        assert_eq!(
            plan.target.as_deref(),
            Some(
                fs::canonicalize(&target)
                    .unwrap()
                    .to_string_lossy()
                    .as_ref()
            )
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn repairs_an_ungrounded_model_target_from_existing_request_names() {
        let root = std::env::temp_dir().join(format!(
            "aish-provider-grounding-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let target = root.join("Orbit-8f31");
        fs::create_dir_all(&target).unwrap();
        let wrong_plan = serde_json::json!({
            "kind": "change_directory",
            "target": root.clone()
        })
        .to_string();
        let plan = plan_from_input(
            "go to Orbit-8f31",
            &[wrong_plan.as_str()],
            serde_json::json!({ "cwd": root.clone() }),
        );
        assert_eq!(plan.action, ProviderPlanAction::ChangeDirectory);
        assert_eq!(
            plan.target.as_deref(),
            Some(
                fs::canonicalize(&target)
                    .unwrap()
                    .to_string_lossy()
                    .as_ref()
            )
        );
        assert_eq!(plan.diagnostics.as_ref().unwrap().retry_count, 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replaces_a_partial_missing_target_with_the_full_existing_name() {
        let root = std::env::temp_dir().join(format!(
            "aish-provider-spaced-grounding-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let target = root.join("Work Area 8f31");
        fs::create_dir_all(&target).unwrap();
        let wrong_plan = serde_json::json!({
            "kind": "change_directory",
            "target": root.join("Work")
        })
        .to_string();
        let plan = plan_from_input(
            "navigate to \"Work Area 8f31\"",
            &[wrong_plan.as_str()],
            serde_json::json!({ "cwd": root.clone() }),
        );
        assert_eq!(plan.action, ProviderPlanAction::ChangeDirectory);
        assert_eq!(
            plan.target.as_deref(),
            Some(
                fs::canonicalize(&target)
                    .unwrap()
                    .to_string_lossy()
                    .as_ref()
            )
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn converts_model_cd_output_to_a_direct_grounded_directory_change() {
        let root = std::env::temp_dir().join(format!(
            "aish-provider-cd-grounding-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let target = root.join("Work Area 8f31");
        fs::create_dir_all(&target).unwrap();
        let wrong_plan = serde_json::json!({
            "kind": "shell_command",
            "payload": format!("cd {}", root.display())
        })
        .to_string();
        let plan = plan_from_input(
            "open Work Area 8f31 from here",
            &[wrong_plan.as_str()],
            serde_json::json!({ "cwd": root.clone() }),
        );
        assert_eq!(plan.action, ProviderPlanAction::ChangeDirectory);
        assert!(plan.command.is_none());
        assert_eq!(
            plan.target.as_deref(),
            Some(
                fs::canonicalize(&target)
                    .unwrap()
                    .to_string_lossy()
                    .as_ref()
            )
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn converts_model_cd_output_without_hiding_directory_ambiguity() {
        let root = std::env::temp_dir().join(format!(
            "aish-provider-cd-ambiguity-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(root.join("left").join("Echo-8f31")).unwrap();
        fs::create_dir_all(root.join("right").join("Echo-8f31")).unwrap();
        let wrong_plan = r#"{"kind":"shell_command","payload":"cd C:\\invented\\Echo-8f31"}"#;
        let plan = plan_from_input(
            "enter Echo-8f31 from this directory",
            &[wrong_plan],
            serde_json::json!({ "cwd": root.clone() }),
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert!(plan.reason.contains("multiple matching directories"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn surfaces_runtime_failure_without_parsing_or_retrying() {
        let request = request(serde_json::json!({}));
        let plan = plan_ai_run_with("fixture intent", request, profile(), |_| {
            Ok(ModelRunResult {
                ok: false,
                command_line: "llama-cli <sanitized>".to_string(),
                output: String::new(),
                error: "Local model runtime exited with 2".to_string(),
                raw_stdout: String::new(),
                raw_stderr: "failure".to_string(),
                exit_code: Some(2),
                structured_output: "json_schema".to_string(),
            })
        });
        assert_eq!(plan.action, ProviderPlanAction::Error);
        assert!(plan
            .error
            .as_deref()
            .is_some_and(|error| error.contains('2')));
        let diagnostics = plan.diagnostics.expect("runtime failure diagnostics");
        assert_eq!(diagnostics.parser_strategy, "runtime_failure");
        assert_eq!(diagnostics.exit_status, Some(2));
        assert_eq!(diagnostics.raw_stderr, "failure");
    }

    #[test]
    fn failed_command_context_is_bounded_and_single_attempt() {
        let context = failed_command_context(
            serde_json::json!({ "cwd": "fixture" }),
            "tool --mistyped",
            Some(127),
            &"failure".repeat(1000),
        );
        let failed = &context["failed_command"];
        assert_eq!(failed["command"], "tool --mistyped");
        assert_eq!(failed["exit_code"], 127);
        assert_eq!(failed["recovery_attempt"], 1);
        assert_eq!(failed["maximum_recovery_attempts"], 1);
        assert!(failed["stderr"].as_str().unwrap().chars().count() <= 2000);
    }

    #[test]
    fn recovery_answers_drop_unsupported_speculation_after_grounded_evidence() {
        let context = failed_command_context(
            serde_json::json!({}),
            "npm run build",
            Some(1),
            "Error [ERR_MODULE_NOT_FOUND]: Cannot find package 'vite'",
        );
        let answer = grounded_recovery_answer(
            "The build failed because the package 'vite' was not found. This is likely due to a typo in vite.config.mjs.",
            &context,
        );
        assert_eq!(
            answer,
            "The build failed because the package 'vite' was not found."
        );
        assert_eq!(
            grounded_recovery_answer(
                "The command is misspelled. The corrected command is git pull.",
                &context,
            ),
            "The command is misspelled. The corrected command is git pull."
        );
    }

    #[test]
    fn recovery_never_repeats_the_failed_command() {
        let context = failed_command_context(
            serde_json::json!({}),
            "gti status",
            Some(127),
            "gti is not recognized",
        );
        let plan = plan_from_input(
            "Diagnose the failed literal command.",
            &[
                r#"{"kind":"shell_command","payload":"gti status"}"#,
                r#"{"kind":"shell_command","payload":"gti status"}"#,
            ],
            context,
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert!(plan.reason.contains("did not run it again"));
    }

    #[test]
    fn recovery_accepts_a_distinct_low_risk_typo_correction() {
        let context = failed_command_context(
            serde_json::json!({}),
            "gti status",
            Some(127),
            "gti is not recognized",
        );
        let plan = plan_from_input(
            "Diagnose the failed literal command.",
            &[r#"{"kind":"shell_command","payload":"git status"}"#],
            context,
        );
        assert_eq!(plan.action, ProviderPlanAction::ShellCommand);
        assert_eq!(plan.command.as_deref(), Some("git status"));
        assert!(!plan.needs_approval);
    }

    #[test]
    fn recovery_rejects_an_unrelated_command_as_a_typo_correction() {
        let context = failed_command_context(
            serde_json::json!({}),
            "npm test",
            Some(1),
            "Missing script: test",
        );
        let plan = plan_from_input(
            "Diagnose the failed literal command.",
            &[
                r#"{"kind":"shell_command","payload":"npm init -y"}"#,
                r#"{"kind":"answer","message":"The package does not define a test script."}"#,
            ],
            context,
        );

        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert_eq!(
            plan.fallback_message.as_deref(),
            Some("The package does not define a test script.")
        );
        assert_eq!(plan.diagnostics.as_ref().unwrap().retry_count, 1);
    }

    #[test]
    fn truncated_recovery_returns_supplied_failure_evidence() {
        let context = failed_command_context(
            serde_json::json!({}),
            "git push",
            Some(128),
            "No configured push destination",
        );
        let mut stopped = result(r#"{"kind":"answer","message":"The command failed because"#);
        stopped.ok = false;
        stopped.exit_code = Some(130);
        stopped.error = "Local model runtime exited with 130: ".to_string();
        let outputs = RefCell::new(VecDeque::from([stopped.clone(), stopped]));
        let plan = plan_ai_run_with(
            "Diagnose the failed literal command.",
            request(context),
            profile(),
            |_| {
                outputs
                    .borrow_mut()
                    .pop_front()
                    .ok_or_else(|| "fixture output exhausted".to_string())
            },
        );

        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert_eq!(
            plan.fallback_message.as_deref(),
            Some("The command failed with exit code 128: No configured push destination")
        );
        assert!(plan.error.is_none());
        assert_eq!(
            plan.diagnostics.as_ref().unwrap().parser_strategy,
            "runtime_failure"
        );
    }

    #[test]
    fn repeated_clean_runtime_stop_returns_user_facing_fallback() {
        let mut stopped = result(r#"{"kind":"answer","message":"A file is"#);
        stopped.ok = false;
        stopped.exit_code = Some(130);
        stopped.error = "Local model runtime exited with 130: ".to_string();
        let outputs = RefCell::new(VecDeque::from([stopped.clone(), stopped]));
        let plan = plan_ai_run_with(
            "what is the difference between a file and a directory?",
            request(serde_json::json!({})),
            profile(),
            |_| {
                outputs
                    .borrow_mut()
                    .pop_front()
                    .ok_or_else(|| "fixture output exhausted".to_string())
            },
        );

        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert!(plan.error.is_none());
        assert!(plan
            .fallback_message
            .as_deref()
            .is_some_and(|message| message.contains("reliable explanation")));
    }

    #[test]
    fn recovery_prompt_is_grounded_in_supplied_failure_evidence() {
        let context = failed_command_context(
            serde_json::json!({}),
            "tool --mistyped",
            Some(127),
            "the option is not recognized",
        );
        let prompt = constrain_plan_prompt(
            "fixture prompt".to_string(),
            "Diagnose the failed literal command.",
            &context,
        );

        assert!(prompt.contains("Use the supplied failed command, exit code, and stderr"));
        assert!(prompt.contains("Do not repeat the failed command"));
        assert!(prompt.contains("invent a cause"));
        assert!(prompt.contains("at most two sentences"));
    }

    #[test]
    fn underspecified_definite_object_requires_clarification() {
        let plan = plan_from_input(
            "run the script",
            &[
                r#"{"kind":"shell_command","payload":"powershell -File .\\script.ps1"}"#,
                r#"{"kind":"shell_command","payload":"powershell -File .\\script.ps1"}"#,
            ],
            serde_json::json!({ "cwd": "fixture" }),
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert!(plan.reason.contains("exact target"));
    }

    #[test]
    fn unresolved_navigation_shell_output_requires_a_directory_clarification() {
        for input in ["open that folder", "use the other project"] {
            let plan = plan_from_input(
                input,
                &[
                    r#"{"kind":"shell_command","payload":"cd C:\\invented\\OtherProject"}"#,
                    r#"{"kind":"shell_command","payload":"cd C:\\invented\\OtherProject"}"#,
                ],
                serde_json::json!({ "cwd": "C:\\workspace" }),
            );
            assert_eq!(plan.action, ProviderPlanAction::Fallback);
            assert!(plan.command.is_none());
            assert!(plan.reason.starts_with("Which existing directory"));
        }
        let plan = plan_from_input(
            "open that folder",
            &[
                r#"{"kind":"shell_command","payload":"Get-ChildItem -Path ."}"#,
                r#"{"kind":"shell_command","payload":"Get-ChildItem -Path ."}"#,
            ],
            serde_json::json!({ "cwd": "C:\\workspace" }),
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
    }

    #[test]
    fn unresolved_permission_changes_never_reach_approval_or_execution() {
        let plan = plan_from_input(
            "fix the permissions",
            &[
                r#"{"kind":"shell_command","payload":"icacls . /grant Everyone:F"}"#,
                r#"{"kind":"shell_command","payload":"icacls . /grant Everyone:F"}"#,
            ],
            serde_json::json!({ "cwd": "C:\\workspace" }),
        );
        assert_eq!(plan.action, ProviderPlanAction::Fallback);
        assert!(plan.command.is_none());
        assert!(plan.reason.contains("exact target"));
    }

    #[test]
    fn current_directory_reference_is_grounded_by_host_context() {
        let plan = plan_from_input(
            "add this directory to PATH",
            &[r#"{"kind":"shell_command","payload":"setx PATH \"%PATH%;%CD%\""}"#],
            serde_json::json!({ "cwd": "C:\\workspace\\arbitrary" }),
        );
        assert_eq!(plan.action, ProviderPlanAction::ApprovalRequired);
        assert!(plan.needs_approval);
    }
}
