use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AppMode {
    Normal,
    History,
    Ai,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AiSubmode {
    Suggest,
    Run,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextLevel {
    Off,
    Minimal,
    Project,
    Terminal,
    Selected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CachePolicy {
    Off,
    ProjectOnly,
    FullLocal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub mode: AppMode,
    pub ai_submode: String,
    pub context_level: ContextLevel,
    pub cache_policy: CachePolicy,
    pub shell: String,
    pub cwd: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Card {
    Command(CommandCard),
    Plan(PlanCard),
    Script(ScriptCard),
    FallbackMessage(FallbackCard),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCard {
    pub os: String,
    pub shell: String,
    pub command: String,
    pub risk: RiskLevel,
    pub category: String,
    pub requires_admin: bool,
    pub modifies_system: bool,
    pub needs_confirmation: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanCard {
    pub os: String,
    pub shell: String,
    pub command: String,
    pub risk: RiskLevel,
    pub category: String,
    pub requires_admin: bool,
    pub modifies_system: bool,
    pub needs_confirmation: bool,
    pub reason: String,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub index: usize,
    pub command: String,
    pub risk: RiskLevel,
    pub modifies_system: bool,
    pub needs_confirmation: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptCard {
    pub os: String,
    pub shell: String,
    pub script: String,
    pub risk: RiskLevel,
    pub category: String,
    pub requires_admin: bool,
    pub modifies_system: bool,
    pub needs_confirmation: bool,
    pub reason: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackCard {
    pub os: String,
    pub shell: String,
    pub command: String,
    pub risk: RiskLevel,
    pub category: String,
    pub requires_admin: bool,
    pub modifies_system: bool,
    pub needs_confirmation: bool,
    pub reason: String,
    pub fallback_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandTrace {
    pub intent: String,
    pub card_type: String,
    pub risk: RiskLevel,
    pub context_used: Vec<String>,
    pub commands: Vec<String>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u128>,
    pub safety_decision: String,
    pub output: String,
    pub error: String,
}
