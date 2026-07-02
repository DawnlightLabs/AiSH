use aish_ai::{build_command_card_prompt, run_gguf_model, ModelProfile, ModelRunRequest};
use aish_core::{AiSubmode, AppMode, CachePolicy, ContextLevel, RiskLevel};
use aish_safety::classify_risk;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPlan {
    pub mode: ProviderInputMode,
    pub surface: String,
    pub action: ProviderPlanAction,
    pub intent: String,
    pub command: Option<String>,
    pub risk: RiskLevel,
    pub needs_approval: bool,
    pub reason: String,
    pub fallback_message: Option<String>,
    pub model_output: Option<String>,
    pub runtime: Option<String>,
    pub error: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
struct CommandCard {
    action_type: String,
    command: Option<String>,
    risk: Option<String>,
    reason: Option<String>,
    fallback_message: Option<String>,
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
            risk: RiskLevel::Low,
            needs_approval: false,
            reason: "No input supplied.".to_string(),
            fallback_message: None,
            model_output: None,
            runtime: None,
            error: None,
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
        risk: local.risk,
        needs_approval: false,
        reason: "Normal mode command.".to_string(),
        fallback_message: None,
        model_output: None,
        runtime: None,
        error: None,
    }
}

pub fn plan_failed_command_recovery(
    command: &str,
    surface: String,
    context_json: serde_json::Value,
    profile: Option<ModelProfile>,
) -> ProviderPlan {
    let intent = format!("The user typed this shell command and it failed: `{command}`. If it is a typo or wrong command, return one corrected shell command. If it cannot be fixed, explain why it failed as a fallback message.");
    plan_provider_input(ProviderPlanRequest {
        mode: ProviderInputMode::AiRun,
        surface,
        input: intent,
        context_json,
        profile,
    })
}

fn plan_ai_run(input: &str, request: ProviderPlanRequest) -> ProviderPlan {
    let Some(profile) = request.profile else {
        return ProviderPlan {
            mode: ProviderInputMode::AiRun,
            surface: request.surface,
            action: ProviderPlanAction::Error,
            intent: input.to_string(),
            command: None,
            risk: RiskLevel::Low,
            needs_approval: false,
            reason: "No model profile is available.".to_string(),
            fallback_message: None,
            model_output: None,
            runtime: None,
            error: Some("No model profile is available.".to_string()),
        };
    };

    let prompt = build_command_card_prompt(input, &request.context_json);
    let result = run_gguf_model(ModelRunRequest { profile, prompt });
    let Ok(result) = result else {
        let error = result
            .err()
            .unwrap_or_else(|| "unknown model error".to_string());
        return ProviderPlan {
            mode: ProviderInputMode::AiRun,
            surface: request.surface,
            action: ProviderPlanAction::Error,
            intent: input.to_string(),
            command: None,
            risk: RiskLevel::Low,
            needs_approval: false,
            reason: error.clone(),
            fallback_message: None,
            model_output: None,
            runtime: None,
            error: Some(error),
        };
    };

    let body = result.output.trim().to_string();
    let runtime = result.command_line;
    let Ok(card) = serde_json::from_str::<CommandCard>(&body) else {
        return ProviderPlan {
            mode: ProviderInputMode::AiRun,
            surface: request.surface,
            action: ProviderPlanAction::Error,
            intent: input.to_string(),
            command: None,
            risk: RiskLevel::Low,
            needs_approval: false,
            reason: "AiSH could not parse a command card from the model.".to_string(),
            fallback_message: None,
            model_output: Some(body),
            runtime: Some(runtime),
            error: Some("could not parse command card".to_string()),
        };
    };

    if card.action_type == "fallback_message" {
        let message = card
            .fallback_message
            .or(card.reason)
            .unwrap_or_else(|| "No command available.".to_string());
        return ProviderPlan {
            mode: ProviderInputMode::AiRun,
            surface: request.surface,
            action: ProviderPlanAction::Fallback,
            intent: input.to_string(),
            command: None,
            risk: RiskLevel::Low,
            needs_approval: false,
            reason: message.clone(),
            fallback_message: Some(message),
            model_output: Some(body),
            runtime: Some(runtime),
            error: None,
        };
    }

    let Some(command) = card
        .command
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return ProviderPlan {
            mode: ProviderInputMode::AiRun,
            surface: request.surface,
            action: ProviderPlanAction::Error,
            intent: input.to_string(),
            command: None,
            risk: parse_model_risk(card.risk.as_deref()),
            needs_approval: false,
            reason: card
                .reason
                .unwrap_or_else(|| "AiSH returned no command.".to_string()),
            fallback_message: None,
            model_output: Some(body),
            runtime: Some(runtime),
            error: Some("empty command".to_string()),
        };
    };

    evaluate_generated_command(
        input,
        command,
        card.risk.as_deref(),
        card.reason.as_deref(),
        request.surface,
        Some(body),
        Some(runtime),
    )
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
        risk,
        needs_approval,
        reason,
        fallback_message: None,
        model_output,
        runtime,
        error: None,
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
    if let Some(runtime) = &plan.runtime {
        events.push(trace_event("debug", "runtime", runtime));
    }
    if let Some(model_output) = &plan.model_output {
        events.push(trace_event("debug", "model_card", model_output));
    }
    if let Some(error) = &plan.error {
        events.push(trace_event("error", "error", error));
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
    let home = home_dir().display().to_string().replace('\\', "/");
    let model_path = std::env::var("AISH_MODEL_PATH").unwrap_or_else(|_| {
        format!("{home}/Downloads/aish-model/models/Qwen2.5-Coder-1.5B-Instruct-Q4_K_M.gguf")
    });
    let llama_cli_path = std::env::var("AISH_LLAMA_CLI").unwrap_or_else(|_| {
        if std::env::consts::OS == "windows" {
            format!("{home}/Downloads/llama.cpp/build/bin/Release/llama-cli.exe")
        } else {
            "llama-cli".to_string()
        }
    });
    ModelProfile {
        id: "qwen25-coder-15b-q4-k-m".to_string(),
        label: "Qwen2.5 Coder 1.5B Instruct Q4_K_M".to_string(),
        family: "qwen2.5-coder".to_string(),
        model_path,
        llama_cli_path,
        context_tokens: 4096,
        max_tokens: 192,
        temperature: 0.1,
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
