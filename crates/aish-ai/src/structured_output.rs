use std::process::Command;

pub(crate) const COMMAND_CARD_JSON_SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "action_type": { "type": "string", "enum": ["shell_command", "fallback_message"] },
    "command": { "type": "string" },
    "risk": { "type": "string", "enum": ["low", "medium", "high"] },
    "reason": { "type": "string" },
    "fallback_message": { "type": "string" }
  },
  "required": ["action_type", "command", "risk", "reason", "fallback_message"],
  "additionalProperties": false
}"#;

pub(crate) const COMMAND_CARD_GBNF: &str = r#"
root ::= "{" ws action-kv "," ws command-kv "," ws risk-kv "," ws reason-kv "," ws fallback-kv "}" ws
action-kv ::= "\"action_type\"" ws ":" ws ("\"shell_command\"" | "\"fallback_message\"")
command-kv ::= "\"command\"" ws ":" ws string
risk-kv ::= "\"risk\"" ws ":" ws ("\"low\"" | "\"medium\"" | "\"high\"")
reason-kv ::= "\"reason\"" ws ":" ws string
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

pub(crate) fn detect_structured_output_mode(
    llama_cli_path: &str,
) -> Result<StructuredOutputMode, String> {
    let output = Command::new(llama_cli_path)
        .arg("--help")
        .output()
        .map_err(|error| format!("Failed to inspect llama-cli capabilities: {error}"))?;
    let help = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    mode_from_help(&help).ok_or_else(|| {
        "The installed llama-cli does not support JSON Schema or grammar-constrained output. Update llama.cpp and run AiSH setup again.".to_string()
    })
}

fn mode_from_help(help: &str) -> Option<StructuredOutputMode> {
    if help.contains("--json-schema") {
        Some(StructuredOutputMode::JsonSchema)
    } else if help.contains("--grammar") {
        Some(StructuredOutputMode::Grammar)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        mode_from_help, StructuredOutputMode, COMMAND_CARD_GBNF, COMMAND_CARD_JSON_SCHEMA,
    };

    #[test]
    fn command_card_schema_is_valid_json() {
        let schema: serde_json::Value =
            serde_json::from_str(COMMAND_CARD_JSON_SCHEMA).expect("valid JSON schema");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["required"].as_array().map(Vec::len), Some(5));
    }

    #[test]
    fn grammar_matches_schema_fields() {
        for field in [
            "action_type",
            "command",
            "risk",
            "reason",
            "fallback_message",
        ] {
            assert!(COMMAND_CARD_GBNF.contains(field));
        }
    }

    #[test]
    fn prefers_json_schema_and_falls_back_to_grammar() {
        assert_eq!(
            mode_from_help("--grammar GBNF --json-schema SCHEMA"),
            Some(StructuredOutputMode::JsonSchema)
        );
        assert_eq!(
            mode_from_help("--grammar GBNF"),
            Some(StructuredOutputMode::Grammar)
        );
        assert_eq!(mode_from_help("--temp N"), None);
    }
}
