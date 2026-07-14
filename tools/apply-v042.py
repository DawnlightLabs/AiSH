from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def write(path: str, content: str) -> None:
    target = ROOT / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"Could not find {label}")
    return text.replace(old, new, 1)


structured_output = r'''use std::process::Command;

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
'''
write("crates/aish-ai/src/structured_output.rs", structured_output)

ai_path = "crates/aish-ai/src/lib.rs"
ai = read(ai_path)
ai = replace_once(
    ai,
    "use std::process::{Command, Stdio};\n",
    "use std::process::{Command, Stdio};\n\nmod structured_output;\nuse structured_output::{\n    detect_structured_output_mode, StructuredOutputMode, COMMAND_CARD_GBNF,\n    COMMAND_CARD_JSON_SCHEMA,\n};\n",
    "aish-ai structured output imports",
)
ai = replace_once(
    ai,
    "pub struct ModelRunRequest {\n    pub profile: ModelProfile,\n    pub prompt: String,\n}",
    "pub struct ModelRunRequest {\n    pub profile: ModelProfile,\n    pub system_prompt: String,\n    pub prompt: String,\n}",
    "ModelRunRequest",
)
start = ai.index("pub fn build_command_card_prompt(")
end = ai.index("\npub fn run_gguf_model", start)
new_prompts = '''pub fn build_command_card_system_prompt() -> String {
    "You are AiSH's local shell planner. Produce one command card for the user's current shell. Use only the supplied operating system, shell, current context, and user intent. The command must be directly runnable in that shell and must not invent paths, filenames, usernames, installed tools, or facts. Use fallback_message only when no useful command can be produced. Read-only work is low risk; state-changing work is medium or high risk. The host independently validates risk and approval. Keep the reason brief.".to_string()
}

pub fn build_command_card_prompt(intent: &str, context_json: &serde_json::Value) -> String {
    let os = target_os();
    let shell = target_shell();
    let family = shell_family(&os, &shell);
    let context = serde_json::to_string_pretty(context_json).unwrap_or_else(|_| "{}".to_string());
    let dialect = match family {
        "powershell" => "Use Windows PowerShell syntax.",
        "fish" => "Use fish shell syntax.",
        _ => "Use POSIX syntax suitable for bash or zsh.",
    };

    format!(
        "Operating system: {os}\nShell: {shell}\nShell family: {family}\nShell constraint: {dialect}\nContext JSON:\n{context}\n\nUser intent:\n{intent}"
    )
}
'''
ai = ai[:start] + new_prompts + ai[end:]
old_command = '''    let mut command = Command::new(&request.profile.llama_cli_path);
    command
        .stdin(Stdio::null())
        .arg("-m")
        .arg(&request.profile.model_path)
        .arg("-p")
        .arg(&request.prompt)
        .arg("-n")
        .arg(request.profile.max_tokens.to_string())
        .arg("--temp")
        .arg(request.profile.temperature.to_string())
        .arg("-c")
        .arg(request.profile.context_tokens.to_string())
        .arg("--no-display-prompt")
        .arg("--single-turn")
        .arg("--reasoning")
        .arg("off");

    let command_line = format!(
        "{} -m {} -p <prompt> -n {} --temp {} -c {} --no-display-prompt --single-turn --reasoning off",
        request.profile.llama_cli_path,
        request.profile.model_path,
        request.profile.max_tokens,
        request.profile.temperature,
        request.profile.context_tokens
    );
'''
new_command = '''    let structured_output = detect_structured_output_mode(&request.profile.llama_cli_path)?;
    let mut command = Command::new(&request.profile.llama_cli_path);
    command
        .stdin(Stdio::null())
        .arg("-m")
        .arg(&request.profile.model_path)
        .arg("--system-prompt")
        .arg(&request.system_prompt)
        .arg("-p")
        .arg(&request.prompt)
        .arg("-n")
        .arg(request.profile.max_tokens.to_string())
        .arg("--temp")
        .arg("0")
        .arg("--top-k")
        .arg("1")
        .arg("--seed")
        .arg("0")
        .arg("-c")
        .arg(request.profile.context_tokens.to_string())
        .arg("--conversation")
        .arg("--single-turn")
        .arg("--no-display-prompt")
        .arg("--reasoning")
        .arg("off");

    match structured_output {
        StructuredOutputMode::JsonSchema => {
            command.arg("--json-schema").arg(COMMAND_CARD_JSON_SCHEMA);
        }
        StructuredOutputMode::Grammar => {
            command.arg("--grammar").arg(COMMAND_CARD_GBNF);
        }
    }

    let constraint = match structured_output {
        StructuredOutputMode::JsonSchema => "--json-schema <command-card-schema>",
        StructuredOutputMode::Grammar => "--grammar <command-card-grammar>",
    };
    let command_line = format!(
        "{} -m {} --system-prompt <system> -p <prompt> -n {} --temp 0 --top-k 1 --seed 0 -c {} --conversation --single-turn --no-display-prompt --reasoning off {}",
        request.profile.llama_cli_path,
        request.profile.model_path,
        request.profile.max_tokens,
        request.profile.context_tokens,
        constraint
    );
'''
ai = replace_once(ai, old_command, new_command, "llama-cli command construction")
write(ai_path, ai)

provider_path = "crates/aish-provider/src/lib.rs"
provider = read(provider_path)
provider = replace_once(
    provider,
    "use aish_ai::{build_command_card_prompt, run_gguf_model, ModelProfile, ModelRunRequest};",
    "use aish_ai::{\n    build_command_card_prompt, build_command_card_system_prompt, run_gguf_model, ModelProfile,\n    ModelRunRequest,\n};",
    "aish-provider imports",
)
provider = replace_once(
    provider,
    "    let prompt = build_command_card_prompt(input, &request.context_json);\n    let result = run_gguf_model(ModelRunRequest { profile, prompt });",
    "    let system_prompt = build_command_card_system_prompt();\n    let prompt = build_command_card_prompt(input, &request.context_json);\n    let result = run_gguf_model(ModelRunRequest {\n        profile,\n        system_prompt,\n        prompt,\n    });",
    "provider model request",
)
provider = provider.replace(
    'error: Some("could not parse command card".to_string()),',
    'error: Some("the constrained model response was not a command card".to_string()),',
    1,
)
write(provider_path, provider)

setup_path = "apps/provider-shell/src/setup.rs"
setup = read(setup_path)
setup = replace_once(
    setup,
    '        let current = env::var("PATH").unwrap_or_default();',
    '        let current = get_windows_user_env("Path")?;',
    "Windows user PATH lookup",
)
old_env = '''fn set_windows_user_env(name: &str, value: &str) -> Result<(), String> {
    let script = format!(
        "[Environment]::SetEnvironmentVariable('{}', $args[0], 'User')",
        name.replace("'", "''")
    );
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
            value,
        ])
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(format!(
            "failed to persist user environment variable {name}"
        ));
    }
    Ok(())
}
'''
new_env = '''fn get_windows_user_env(name: &str) -> Result<String, String> {
    let output = Command::new("powershell")
        .env("AISH_ENV_NAME", name)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "[Environment]::GetEnvironmentVariable($env:AISH_ENV_NAME, 'User')",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(format!("failed to read user environment variable {name}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn set_windows_user_env(name: &str, value: &str) -> Result<(), String> {
    let status = Command::new("powershell")
        .env("AISH_ENV_NAME", name)
        .env("AISH_ENV_VALUE", value)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            "[Environment]::SetEnvironmentVariable($env:AISH_ENV_NAME, $env:AISH_ENV_VALUE, 'User')",
        ])
        .status()
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(format!(
            "failed to persist user environment variable {name}"
        ));
    }
    Ok(())
}
'''
setup = replace_once(setup, old_env, new_env, "Windows environment persistence")
write(setup_path, setup)

release_notes = '''# AiSH v0.4.2

AiSH 0.4.2 makes local command planning structurally reliable without request-specific prompt rules.

## Fixed

- Uses llama.cpp JSON Schema constrained decoding for command cards.
- Falls back to an equivalent GBNF grammar when JSON Schema is unavailable.
- Uses the GGUF model's chat template with separate system and user messages.
- Uses deterministic planner sampling while retaining independent AiSH risk classification.
- Fixes Windows user PATH and `AISH_MODEL_PATH` persistence for values containing spaces.
- Reads and updates only the user PATH rather than copying the combined process PATH into it.

## Upgrade

Run `/update` inside AiSH or `aish --update` from another terminal and accept the update.
'''
write("docs/releases/v0.4.2.md", release_notes)

ci = subprocess.check_output(
    ["git", "show", "origin/main:.github/workflows/ci.yml"],
    cwd=ROOT,
    text=True,
)
ci = ci.replace(
    "      - run: cargo check --workspace\n",
    "      - run: cargo check --workspace\n      - run: cargo test -p aish-ai -p aish-provider\n",
    1,
)
marker = "      - name: Verify embedded Windows icon resource\n"
env_smoke = '''      - name: Verify Windows user environment persistence
        shell: pwsh
        run: |
          $builtExe = Join-Path $env:GITHUB_WORKSPACE 'target\\debug\\aish.exe'
          $stagedRoot = Join-Path $env:RUNNER_TEMP 'AiSH environment smoke with spaces'
          $oldUserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
          $oldModelPath = [Environment]::GetEnvironmentVariable('AISH_MODEL_PATH', 'User')
          try {
            [Environment]::SetEnvironmentVariable('Path', 'C:\\Program Files\\Node Test', 'User')
            [Environment]::SetEnvironmentVariable('AISH_MODEL_PATH', $null, 'User')
            & $builtExe --install-headless --install-dir $stagedRoot --skip-model --add-path --set-model-path --no-windows-terminal --no-editor-profiles
            if ($LASTEXITCODE -ne 0) { throw 'Environment persistence setup failed.' }
            $savedPath = [Environment]::GetEnvironmentVariable('Path', 'User')
            $expectedBin = Join-Path $stagedRoot 'bin'
            if (($savedPath -split ';') -notcontains $expectedBin) {
              throw 'AiSH bin directory was not added to the user PATH.'
            }
            if ($savedPath -notmatch [regex]::Escape('C:\\Program Files\\Node Test')) {
              throw 'Existing user PATH entries containing spaces were not preserved.'
            }
            $savedModel = [Environment]::GetEnvironmentVariable('AISH_MODEL_PATH', 'User')
            if ([string]::IsNullOrWhiteSpace($savedModel)) {
              throw 'AISH_MODEL_PATH was not persisted.'
            }
          } finally {
            [Environment]::SetEnvironmentVariable('Path', $oldUserPath, 'User')
            [Environment]::SetEnvironmentVariable('AISH_MODEL_PATH', $oldModelPath, 'User')
          }
'''
if marker not in ci:
    raise SystemExit("Could not find Windows CI insertion point")
ci = ci.replace(marker, env_smoke + marker, 1)
write(".github/workflows/ci.yml", ci)

for temporary in [
    ".github/workflows/apply-command-card-fix.yml",
    ".github/workflows/kick-command-card-fix.yml",
]:
    path = ROOT / temporary
    if path.exists():
        path.unlink()

Path(__file__).unlink()

subprocess.run(["cargo", "fmt", "--all"], cwd=ROOT, check=True)
subprocess.run(
    ["cargo", "test", "-p", "aish-ai", "-p", "aish-provider"],
    cwd=ROOT,
    check=True,
)
