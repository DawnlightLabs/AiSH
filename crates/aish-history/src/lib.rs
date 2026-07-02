use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub shell: String,
    pub os: String,
    pub cwd_hash: String,
    pub project_type: Option<String>,
    pub typed_prefix: Option<String>,
    pub command: String,
    pub source: String,
    pub accepted: bool,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
}

impl CommandEvent {
    pub fn new(
        shell: impl Into<String>,
        os: impl Into<String>,
        command: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            shell: shell.into(),
            os: os.into(),
            cwd_hash: String::new(),
            project_type: None,
            typed_prefix: None,
            command: command.into(),
            source: "manual".to_string(),
            accepted: true,
            exit_code: None,
            duration_ms: None,
        }
    }
}
