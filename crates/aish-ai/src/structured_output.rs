use std::process::Command;

pub(crate) const COMMAND_CARD_JSON_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "action_type": { "type": "string", "enum": ["shell_command", "fallback_message"] },
    "command": { "type": "string" },
    "fallback_message": { "type": "string" }
  },
  "required": ["action_type", "command", "fallback_message"],
  "additionalProperties": false
}"#;

pub(crate) const COMMAND_CARD_GBNF: &str = r#"
root ::= "{" ws action-kv "," ws command-kv "," ws fallback-kv "}" ws
action-kv ::= "\"action_type\"" ws ":" ws ("\"shell_command\"" | "\"fallback_message\"")
command-kv ::= "\"command\"" ws ":" ws string
fallback-kv ::= "\"fallback_message\"" ws ":" ws string
string ::= "\"" char* "\""
char ::= [^"\\\x7F\x00-\x1F] | "\\" (["\\/bfnrt] | "u" hex hex hex hex)
hex ::= [0-9a-fA-F]
ws ::= [ \t\n\r]*
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructuredOutputMode {
    JsonSchema,
    Grammar,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LlamaCliCapabilities {
    pub mode: StructuredOutputMode,
    pub system_prompt: bool,
    pub conversation: bool,
    pub single_turn: bool,
    pub no_display_prompt: bool,
    pub color: bool,
    pub simple_io: bool,
    pub no_show_timings: bool,
    pub log_disable: bool,
    pub no_warmup: bool,
}

pub(crate) fn inspect_llama_cli_capabilities(
    llama_cli_path: &str,
) -> Result<LlamaCliCapabilities, String> {
    let output = Command::new(llama_cli_path)
        .arg("--help")
        .output()
        .map_err(|error| format!("Failed to inspect llama-cli capabilities: {error}"))?;
    let help = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    capabilities_from_help(&help).ok_or_else(|| {
        "The installed llama-cli does not support JSON Schema or grammar-constrained output. Update llama.cpp and run AiSH setup again.".to_string()
    })
}

fn capabilities_from_help(help: &str) -> Option<LlamaCliCapabilities> {
    let mode = if help.contains("--json-schema") {
        StructuredOutputMode::JsonSchema
    } else if help.contains("--grammar") {
        StructuredOutputMode::Grammar
    } else {
        return None;
    };

    Some(LlamaCliCapabilities {
        mode,
        system_prompt: help.contains("--system-prompt"),
        conversation: help.contains("--conversation"),
        single_turn: help.contains("--single-turn"),
        no_display_prompt: help.contains("--no-display-prompt"),
        color: help.contains("--color"),
        simple_io: help.contains("--simple-io"),
        no_show_timings: help.contains("--no-show-timings"),
        log_disable: help.contains("--log-disable"),
        no_warmup: help.contains("--no-warmup"),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        capabilities_from_help, StructuredOutputMode, COMMAND_CARD_GBNF, COMMAND_CARD_JSON_SCHEMA,
    };

    #[test]
    fn command_card_schema_is_valid_json() {
        let schema: serde_json::Value =
            serde_json::from_str(COMMAND_CARD_JSON_SCHEMA).expect("valid JSON schema");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["required"].as_array().map(Vec::len), Some(3));
    }

    #[test]
    fn grammar_matches_schema_fields() {
        for field in ["action_type", "command", "fallback_message"] {
            assert!(COMMAND_CARD_GBNF.contains(field));
        }
    }

    #[test]
    fn inspects_structured_and_quiet_output_flags() {
        let capabilities = capabilities_from_help(
            "--grammar --json-schema --system-prompt --conversation --single-turn \
             --no-display-prompt --color --simple-io --no-show-timings --log-disable --no-warmup",
        )
        .expect("capabilities");
        assert_eq!(capabilities.mode, StructuredOutputMode::JsonSchema);
        assert!(capabilities.system_prompt);
        assert!(capabilities.simple_io);
        assert!(capabilities.log_disable);

        let grammar = capabilities_from_help("--grammar").expect("grammar fallback");
        assert_eq!(grammar.mode, StructuredOutputMode::Grammar);
        assert!(capabilities_from_help("--temp N").is_none());
    }
}
