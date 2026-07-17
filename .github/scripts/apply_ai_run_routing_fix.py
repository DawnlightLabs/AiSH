from pathlib import Path


def replace_between(text: str, start_marker: str, end_marker: str, replacement: str) -> str:
    start = text.index(start_marker)
    end = text.index(end_marker, start)
    return text[:start] + replacement + text[end:]


# Remove an accidental empty placeholder from main while this branch is merged.
Path("docs/placeholder-temp").unlink(missing_ok=True)

main_path = Path("apps/provider-shell/src/main_setup.rs")
main = main_path.read_text()
main = main.replace(
    '        "Mode: {}. Type /mode normal, /mode ai, commands, natural language, or /help.",',
    '        "Mode: {}. Natural language is the default; use //command to force a literal shell command. Type /help for controls.",',
)
main = main.replace(
    '    println!("  //text                 send a literal slash-prefixed line");',
    '    println!("  //command              force a literal shell command while AI mode is active");',
)

routing = r'''fn looks_like_command_attempt(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.ends_with('?') {
        return false;
    }

    if has_explicit_shell_syntax(trimmed) {
        return true;
    }

    let mut words = trimmed.split_whitespace();
    let first = words.next().unwrap_or_default().to_ascii_lowercase();
    let second = words.next().map(|value| value.to_ascii_lowercase());

    // `go` is both a natural-language verb and the Go toolchain executable.
    // Only treat it as a literal command when the next token is a real Go CLI subcommand.
    if first == "go" {
        return is_go_cli_invocation(second.as_deref());
    }

    if is_natural_language_lead(&first) {
        return false;
    }

    is_high_confidence_command(&first)
}

fn has_explicit_shell_syntax(input: &str) -> bool {
    if input.contains("&&")
        || input.contains("||")
        || input.contains(" | ")
        || input.contains("; ")
        || input.starts_with("./")
        || input.starts_with(".\\")
        || input.starts_with("~/")
        || input.starts_with("~\\")
    {
        return true;
    }

    let first = input.split_whitespace().next().unwrap_or_default();
    let lower = first.to_ascii_lowercase();
    first.contains('\\')
        || (first.contains('/') && !lower.starts_with("http://") && !lower.starts_with("https://"))
        || [".exe", ".cmd", ".bat", ".ps1", ".sh"].iter().any(|suffix| lower.ends_with(suffix))
}

fn is_natural_language_lead(first: &str) -> bool {
    [
        "show", "find", "create", "make", "run", "install", "open", "explain", "what",
        "why", "how", "can", "could", "would", "please", "list", "tell", "check", "change",
        "switch", "move", "copy", "delete", "remove", "rename", "take", "give", "print", "display",
        "navigate", "enter", "leave", "search", "look", "help", "set", "use",
    ]
    .contains(&first)
}

fn is_go_cli_invocation(second: Option<&str>) -> bool {
    matches!(
        second,
        Some(
            "bug"
                | "build"
                | "clean"
                | "doc"
                | "env"
                | "fix"
                | "fmt"
                | "generate"
                | "get"
                | "install"
                | "list"
                | "mod"
                | "run"
                | "test"
                | "tool"
                | "version"
                | "vet"
                | "work"
        )
    )
}

fn is_high_confidence_command(first: &str) -> bool {
    [
        "cd",
        "set-location",
        "sl",
        "dir",
        "ls",
        "pwd",
        "get-location",
        "cat",
        "get-content",
        "echo",
        "write-output",
        "clear",
        "cls",
        "git",
        "gh",
        "npm",
        "pnpm",
        "yarn",
        "bun",
        "node",
        "python",
        "python3",
        "py",
        "pip",
        "pip3",
        "cargo",
        "rustc",
        "docker",
        "docker-compose",
        "kubectl",
        "helm",
        "terraform",
        "winget",
        "choco",
        "scoop",
        "code",
        "cursor",
        "mkdir",
        "new-item",
        "touch",
        "rm",
        "del",
        "remove-item",
        "cp",
        "copy-item",
        "mv",
        "move-item",
        "get-childitem",
        "select-string",
        "findstr",
        "test-path",
        "invoke-webrequest",
        "curl",
        "wget",
        "rg",
        "fd",
        "jq",
        "sed",
        "awk",
    ]
    .contains(&first)
}

'''
main = replace_between(main, "fn looks_like_command_attempt", "fn run_shell_command", routing)

navigation = r'''fn handle_cd(command: &str) -> Option<bool> {
    let trimmed = command.trim();
    let target = if trimmed.eq_ignore_ascii_case("cd")
        || trimmed.eq_ignore_ascii_case("set-location")
        || trimmed.eq_ignore_ascii_case("sl")
    {
        home_dir()
    } else {
        let remainder = command_remainder(trimmed, "cd ")
            .or_else(|| command_remainder(trimmed, "set-location "))
            .or_else(|| command_remainder(trimmed, "sl "))?;
        let remainder = strip_location_parameter(remainder);
        expand_shell_path(&unquote(remainder))
    };

    if let Err(error) = env::set_current_dir(target) {
        eprintln!("cd failed: {error}");
        Some(false)
    } else {
        Some(true)
    }
}

fn command_remainder<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
        .map(|_| &value[prefix.len()..])
}

fn strip_location_parameter(value: &str) -> &str {
    let trimmed = value.trim();
    command_remainder(trimmed, "-literalpath ")
        .or_else(|| command_remainder(trimmed, "-path "))
        .unwrap_or(trimmed)
}

fn expand_shell_path(value: &str) -> PathBuf {
    let trimmed = value.trim();
    for prefix in [
        "~/",
        "~\\",
        "$HOME/",
        "$HOME\\",
        "$env:USERPROFILE/",
        "$env:USERPROFILE\\",
        "%USERPROFILE%/",
        "%USERPROFILE%\\",
    ] {
        if let Some(rest) = strip_prefix_ascii_case(trimmed, prefix) {
            return home_dir().join(rest);
        }
    }

    if ["~", "$HOME", "$env:USERPROFILE", "%USERPROFILE%"]
        .iter()
        .any(|candidate| trimmed.eq_ignore_ascii_case(candidate))
    {
        return home_dir();
    }

    PathBuf::from(trimmed)
}

fn strip_prefix_ascii_case<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .filter(|candidate| candidate.eq_ignore_ascii_case(prefix))
        .map(|_| &value[prefix.len()..])
}

'''
main = replace_between(main, "fn handle_cd", "fn default_profile", navigation)

if "ai_mode_routes_navigation_language_to_the_planner" not in main:
    main += r'''

#[cfg(test)]
mod tests {
    use super::{expand_shell_path, looks_like_command_attempt};

    #[test]
    fn ai_mode_routes_navigation_language_to_the_planner() {
        assert!(!looks_like_command_attempt("go to downloads"));
        assert!(!looks_like_command_attempt("navigate to the Downloads folder"));
        assert!(!looks_like_command_attempt("list the top-level folders"));
    }

    #[test]
    fn ai_mode_keeps_high_confidence_commands_literal() {
        assert!(looks_like_command_attempt("git status"));
        assert!(looks_like_command_attempt("Get-ChildItem -Directory"));
        assert!(looks_like_command_attempt("go version"));
        assert!(looks_like_command_attempt(".\\tools\\build.ps1"));
    }

    #[test]
    fn navigation_expands_common_home_forms() {
        let home = super::home_dir();
        assert_eq!(expand_shell_path("$HOME\\Downloads"), home.join("Downloads"));
        assert_eq!(
            expand_shell_path("$env:USERPROFILE\\Downloads"),
            home.join("Downloads")
        );
    }
}
'''
main_path.write_text(main)


structured_path = Path("crates/aish-ai/src/structured_output.rs")
structured_path.write_text(r'''use std::process::Command;

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
        capabilities_from_help, StructuredOutputMode, COMMAND_CARD_GBNF,
        COMMAND_CARD_JSON_SCHEMA,
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
''')


ai_path = Path("crates/aish-ai/src/lib.rs")
ai = ai_path.read_text()
ai = ai.replace(
    "    detect_structured_output_mode, StructuredOutputMode, COMMAND_CARD_GBNF,\n    COMMAND_CARD_JSON_SCHEMA,",
    "    inspect_llama_cli_capabilities, StructuredOutputMode, COMMAND_CARD_GBNF,\n    COMMAND_CARD_JSON_SCHEMA,",
)
ai = ai.replace(
    '    "You are AiSH\'s local shell planner. Produce one command card for the user\'s current shell. Use only the supplied operating system, shell, current context, and user intent. The command must be directly runnable in that shell and must not invent paths, filenames, usernames, installed tools, or facts. Use fallback_message only when no useful command can be produced. Read-only work is low risk; state-changing work is medium or high risk. The host independently validates risk and approval. Keep the reason brief.".to_string()',
    '    "You are AiSH\'s local shell planner. Produce one command card for the user\'s current shell. Return exactly one JSON object with action_type, command, risk, reason, and fallback_message. action_type is shell_command or fallback_message; keep the inactive command or fallback field empty. Use only the supplied operating system, shell, current context, and user intent. The command must be directly runnable in that shell and must not invent paths, filenames, usernames, installed tools, or facts. Read-only work is low risk; state-changing work is medium or high risk. The host independently validates risk and approval. Keep the reason brief.".to_string()',
)

run_model = r'''pub fn run_gguf_model(request: ModelRunRequest) -> Result<ModelRunResult, String> {
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
    let prompt = if capabilities.system_prompt {
        request.prompt.clone()
    } else {
        format!("System instructions:\n{}\n\n{}", request.system_prompt, request.prompt)
    };

    let mut command = Command::new(&request.profile.llama_cli_path);
    command
        .stdin(Stdio::null())
        .arg("-m")
        .arg(&request.profile.model_path);

    if capabilities.system_prompt {
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

    if capabilities.conversation {
        command.arg("--conversation");
    }
    if capabilities.single_turn {
        command.arg("--single-turn");
    }
    if capabilities.no_display_prompt {
        command.arg("--no-display-prompt");
    }
    if capabilities.color {
        command.args(["--color", "off"]);
    }
    if capabilities.simple_io {
        command.arg("--simple-io");
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

    match capabilities.mode {
        StructuredOutputMode::JsonSchema => {
            command.arg("--json-schema").arg(COMMAND_CARD_JSON_SCHEMA);
        }
        StructuredOutputMode::Grammar => {
            command.arg("--grammar").arg(COMMAND_CARD_GBNF);
        }
    }

    let constraint = match capabilities.mode {
        StructuredOutputMode::JsonSchema => "--json-schema <command-card-schema>",
        StructuredOutputMode::Grammar => "--grammar <command-card-grammar>",
    };
    let command_line = format!(
        "{} -m {} -p <prompt> -n {} --temp 0 --top-k 1 --seed 0 -c {} {}",
        request.profile.llama_cli_path,
        request.profile.model_path,
        request.profile.max_tokens,
        request.profile.context_tokens,
        constraint
    );

    let output = command
        .output()
        .map_err(|error| format!("Failed to start local model runtime: {error}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!(
            "Local model runtime failed: {}",
            clean_runtime_text(&stderr)
        ));
    }

    Ok(ModelRunResult {
        ok: true,
        command_line,
        output: clean_model_output(&stdout),
        error: String::new(),
    })
}

'''
ai = replace_between(ai, "pub fn run_gguf_model", "fn clean_model_output", run_model)

cleaning = r'''fn clean_model_output(raw: &str) -> String {
    let normalized = strip_ansi(raw);
    if let Some(json) = extract_command_card_json(&normalized) {
        return json;
    }

    let text = clean_runtime_text(&normalized);
    if text.trim().is_empty() {
        "No model response returned.".to_string()
    } else {
        text
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

fn extract_command_card_json(raw: &str) -> Option<String> {
    let mut selected = None;

    for (start, ch) in raw.char_indices() {
        if ch != '{' {
            continue;
        }
        let Some(length) = matching_json_object_length(&raw[start..]) else {
            continue;
        };
        let candidate = &raw[start..start + length];
        let Ok(value) = serde_json::from_str::<serde_json::Value>(candidate) else {
            continue;
        };
        if is_command_card_value(&value) {
            selected = Some(candidate.trim().to_string());
        }
    }

    selected
}

fn matching_json_object_length(value: &str) -> Option<usize> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(offset + ch.len_utf8());
                }
            }
            _ => {}
        }
    }

    None
}

fn is_command_card_value(value: &serde_json::Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    matches!(
        object.get("action_type").and_then(serde_json::Value::as_str),
        Some("shell_command" | "fallback_message")
    ) && object.get("command").is_some()
        && object.get("risk").is_some()
        && object.get("reason").is_some()
        && object.get("fallback_message").is_some()
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
    use super::{clean_model_output, extract_command_card_json};

    const CARD: &str = r#"{"action_type":"shell_command","command":"Set-Location \\"$HOME\\Downloads\\"","risk":"low","reason":"Navigate to Downloads.","fallback_message":""}"#;

    #[test]
    fn selects_the_command_card_instead_of_echoed_context_json() {
        let raw = format!(
            "Context JSON:\n{{\"cwd\":\"C:\\\\Users\\\\Amaan\",\"nested\":{{\"kind\":\"repo\"}}}}\nassistant:\n{CARD}\n"
        );
        assert_eq!(clean_model_output(&raw), CARD);
    }

    #[test]
    fn accepts_fenced_and_colored_strict_json() {
        let raw = format!("\\u{{1b}}[36m```json\n{CARD}\n```\\u{{1b}}[0m");
        assert_eq!(clean_model_output(&raw), CARD);
    }

    #[test]
    fn rejects_unrelated_json_objects() {
        assert!(extract_command_card_json(r#"{"cwd":"C:/Users/Amaan"}"#).is_none());
    }
}
'''
ai = replace_between(ai, "fn clean_model_output", "", cleaning) if False else ai[: ai.index("fn clean_model_output")] + cleaning
ai_path.write_text(ai)


# Ensure the normal validation workflows run the shell routing tests too.
for workflow_name in [".github/workflows/ci.yml", ".github/workflows/apply-command-card-fix.yml"]:
    workflow = Path(workflow_name)
    if workflow.exists():
        text = workflow.read_text()
        text = text.replace(
            "cargo test -p aish-ai -p aish-provider",
            "cargo test -p aish-ai -p aish-provider -p aish-provider-shell",
        )
        workflow.write_text(text)

Path("docs/releases/v0.4.4.md").write_text(
    """# AiSH v0.4.4

AiSH 0.4.4 fixes AI-mode routing and local-model output transport.

## Fixed

- Treats natural-language navigation such as `go to downloads` as an AI request instead of invoking the Go compiler command.
- Makes AI mode natural-language-first while preserving high-confidence literal commands and the `//command` override.
- Selects the final strict command-card JSON object from noisy `llama-cli` output instead of accidentally parsing echoed context JSON.
- Uses supported quiet-output flags detected from the installed `llama-cli` version.
- Expands `$HOME`, `$env:USERPROFILE`, `%USERPROFILE%`, and `~` for persistent `cd` and `Set-Location` navigation.
- Adds routing and noisy-output regression tests on Windows and Linux.
"""
)
