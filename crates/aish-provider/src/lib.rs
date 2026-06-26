use aish_core::{AiSubmode, AppMode, CachePolicy, ContextLevel, RiskLevel};
use serde::{Deserialize, Serialize};

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
