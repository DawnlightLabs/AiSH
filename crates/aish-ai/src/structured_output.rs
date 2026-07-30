use std::process::Command;

use crate::SemanticPlanKind;

pub(crate) const SEMANTIC_PLAN_JSON_SCHEMA: &str = r#"{
  "oneOf": [
    {"type":"object","properties":{"kind":{"const":"shell_command"},"payload":{"type":"string","minLength":1}},"required":["kind","payload"],"additionalProperties":false},
    {"type":"object","properties":{"kind":{"const":"change_directory"},"target":{"type":"string","minLength":1},"scope":{"type":"string"}},"required":["kind","target"],"additionalProperties":false},
    {"type":"object","properties":{"kind":{"const":"filesystem_action"},"operation":{"enum":["create_file","create_directory","delete","rename","move","copy","write_file","append_file"]},"target":{"type":"string","minLength":1},"destination":{"type":"string","minLength":1},"content":{"type":"string","minLength":1},"scope":{"type":"string"}},"required":["kind","operation","target"],"additionalProperties":false},
    {"type":"object","properties":{"kind":{"const":"answer"},"message":{"type":"string","minLength":1}},"required":["kind","message"],"additionalProperties":false},
    {"type":"object","properties":{"kind":{"const":"clarification"},"message":{"type":"string","minLength":1}},"required":["kind","message"],"additionalProperties":false}
  ]
}"#;

pub(crate) const SEMANTIC_PLAN_GBNF: &str = r#"
root ::= plan ws trailer?
plan ::= shell | navigation | navigation-scoped | filesystem | filesystem-destination | filesystem-scoped | filesystem-destination-scoped | filesystem-content | filesystem-content-scoped | answer | clarification
shell ::= "{" ws "\"kind\"" ws ":" ws "\"shell_command\"" ws "," ws "\"payload\"" ws ":" ws string ws "}" ws
navigation ::= "{" ws "\"kind\"" ws ":" ws "\"change_directory\"" ws "," ws "\"target\"" ws ":" ws string ws "}" ws
navigation-scoped ::= "{" ws "\"kind\"" ws ":" ws "\"change_directory\"" ws "," ws "\"target\"" ws ":" ws string ws "," ws "\"scope\"" ws ":" ws string ws "}" ws
filesystem ::= "{" ws "\"kind\"" ws ":" ws "\"filesystem_action\"" ws "," ws "\"operation\"" ws ":" ws filesystem-operation ws "," ws "\"target\"" ws ":" ws string ws "}" ws
filesystem-destination ::= "{" ws "\"kind\"" ws ":" ws "\"filesystem_action\"" ws "," ws "\"operation\"" ws ":" ws filesystem-operation ws "," ws "\"target\"" ws ":" ws string ws "," ws "\"destination\"" ws ":" ws string ws "}" ws
filesystem-scoped ::= "{" ws "\"kind\"" ws ":" ws "\"filesystem_action\"" ws "," ws "\"operation\"" ws ":" ws filesystem-operation ws "," ws "\"target\"" ws ":" ws string ws "," ws "\"scope\"" ws ":" ws string ws "}" ws
filesystem-destination-scoped ::= "{" ws "\"kind\"" ws ":" ws "\"filesystem_action\"" ws "," ws "\"operation\"" ws ":" ws filesystem-operation ws "," ws "\"target\"" ws ":" ws string ws "," ws "\"destination\"" ws ":" ws string ws "," ws "\"scope\"" ws ":" ws string ws "}" ws
filesystem-content ::= "{" ws "\"kind\"" ws ":" ws "\"filesystem_action\"" ws "," ws "\"operation\"" ws ":" ws filesystem-write-operation ws "," ws "\"target\"" ws ":" ws string ws "," ws "\"content\"" ws ":" ws string ws "}" ws
filesystem-content-scoped ::= "{" ws "\"kind\"" ws ":" ws "\"filesystem_action\"" ws "," ws "\"operation\"" ws ":" ws filesystem-write-operation ws "," ws "\"target\"" ws ":" ws string ws "," ws "\"content\"" ws ":" ws string ws "," ws "\"scope\"" ws ":" ws string ws "}" ws
filesystem-operation ::= "\"create_file\"" | "\"create_directory\"" | "\"delete\"" | "\"rename\"" | "\"move\"" | "\"copy\"" | filesystem-write-operation
filesystem-write-operation ::= "\"write_file\"" | "\"append_file\""
answer ::= "{" ws "\"kind\"" ws ":" ws "\"answer\"" ws "," ws "\"message\"" ws ":" ws string ws "}" ws
clarification ::= "{" ws "\"kind\"" ws ":" ws "\"clarification\"" ws "," ws "\"message\"" ws ":" ws string ws "}" ws
string ::= "\"" char+ "\""
char ::= [^"\\\x7F\x00-\x1F] | "\\" (["\\/bfnrt] | "u" hex hex hex hex)
hex ::= [0-9a-fA-F]
ws ::= [ \t\n\r]*
trailer ::= "<|im_start|>" | "<|im_end|>"
"#;

pub(crate) fn semantic_plan_grammar_for(kind: SemanticPlanKind) -> String {
    let production = match kind {
        SemanticPlanKind::ShellCommand => "shell",
        SemanticPlanKind::ChangeDirectory => "navigation | navigation-scoped",
        SemanticPlanKind::FilesystemAction => {
            "filesystem | filesystem-destination | filesystem-scoped | filesystem-destination-scoped | filesystem-content | filesystem-content-scoped"
        }
        SemanticPlanKind::Answer => "answer",
        SemanticPlanKind::Clarification => "clarification",
    };
    grammar_with_plan_production(production)
}

pub(crate) fn failed_command_recovery_grammar() -> String {
    grammar_with_plan_production("shell | answer")
}

fn grammar_with_plan_production(production: &str) -> String {
    SEMANTIC_PLAN_GBNF.replacen(
        "plan ::= shell | navigation | navigation-scoped | filesystem | filesystem-destination | filesystem-scoped | filesystem-destination-scoped | filesystem-content | filesystem-content-scoped | answer | clarification",
        &format!("plan ::= {production}"),
        1,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructuredOutputMode {
    JsonSchema,
    Grammar,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LlamaCliCapabilities {
    pub mode: StructuredOutputMode,
    pub json_schema: bool,
    pub grammar: bool,
    pub system_prompt: bool,
    pub no_display_prompt: bool,
    pub color: bool,
    pub no_show_timings: bool,
    pub log_disable: bool,
    pub no_warmup: bool,
    pub no_conversation: bool,
    pub chat_template: bool,
    pub gpu_layers: bool,
    pub fit: bool,
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
    if !output.status.success() {
        return Err(format!(
            "llama-cli capability check exited with {}. Reinstall the managed runtime.",
            output.status.code().unwrap_or(-1)
        ));
    }
    capabilities_from_help(&help).ok_or_else(|| {
        "The installed llama-cli cannot constrain semantic-plan output. Run AiSH setup to repair the managed runtime.".to_string()
    })
}

fn capabilities_from_help(help: &str) -> Option<LlamaCliCapabilities> {
    let json_schema = help.contains("--json-schema");
    let grammar = help.contains("--grammar");
    let mode = if json_schema {
        StructuredOutputMode::JsonSchema
    } else if grammar {
        StructuredOutputMode::Grammar
    } else {
        return None;
    };
    Some(LlamaCliCapabilities {
        mode,
        json_schema,
        grammar,
        system_prompt: help.contains("--system-prompt"),
        no_display_prompt: help.contains("--no-display-prompt"),
        color: help.contains("--color"),
        no_show_timings: help.contains("--no-show-timings"),
        log_disable: help.contains("--log-disable"),
        no_warmup: help.contains("--no-warmup"),
        no_conversation: help.contains("--no-conversation"),
        chat_template: help.contains("--chat-template"),
        gpu_layers: help.contains("--gpu-layers") || help.contains("--n-gpu-layers"),
        fit: help.contains("--fit"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_valid_and_grammar_matches_contract() {
        let schema: serde_json::Value = serde_json::from_str(SEMANTIC_PLAN_JSON_SCHEMA).unwrap();
        assert_eq!(schema["oneOf"].as_array().map(Vec::len), Some(5));
        for field in [
            "kind",
            "payload",
            "target",
            "scope",
            "message",
            "operation",
            "destination",
            "content",
        ] {
            assert!(SEMANTIC_PLAN_GBNF.contains(field));
        }
        assert!(SEMANTIC_PLAN_GBNF.contains("<|im_start|>"));
        let answer_only = semantic_plan_grammar_for(SemanticPlanKind::Answer);
        assert!(answer_only.contains("plan ::= answer"));
        assert!(!answer_only.contains("plan ::= shell |"));
        let recovery = failed_command_recovery_grammar();
        assert!(recovery.contains("plan ::= shell | answer"));
    }

    #[test]
    fn detects_runtime_capabilities() {
        let capabilities = capabilities_from_help(
            "--grammar --json-schema --system-prompt --conversation --no-conversation --single-turn --no-display-prompt --color --simple-io --no-show-timings --log-disable --no-warmup --chat-template --gpu-layers --fit",
        )
        .unwrap();
        assert_eq!(capabilities.mode, StructuredOutputMode::JsonSchema);
        assert!(capabilities.grammar && capabilities.chat_template);
        assert!(capabilities.no_conversation);
        assert!(capabilities.gpu_layers && capabilities.fit);
        assert_eq!(
            capabilities_from_help("--grammar").unwrap().mode,
            StructuredOutputMode::Grammar
        );
        assert!(capabilities_from_help("--temp N").is_none());
    }
}
