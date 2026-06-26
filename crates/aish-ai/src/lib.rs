use aish_core::Card;
use serde::{Deserialize, Serialize};

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
