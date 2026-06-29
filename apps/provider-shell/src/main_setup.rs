mod setup;

use aish_ai::{build_command_card_prompt, run_gguf_model, ModelProfile, ModelRunRequest};
use serde::Deserialize;
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
struct CommandCard {
    action_type: String,
    command: Option<String>,
    risk: Option<String>,
    reason: Option<String>,
    fallback_message: Option<String>,
}

#[derive(Debug, Clone)]
struct PendingCommand {
    command: String,
    risk: String,
    reason: String,
}

#[derive(Debug, Clone)]
struct ProviderState {
    profile: ModelProfile,
    pending: Option<PendingCommand>,
    show_trace: bool,
}

fn main() {
    setup::handle_setup_args();
    install_prompt_env();

    let mut state = ProviderState {
        profile: default_profile(),
        pending: None,
        show_trace: false,
    };
    setup::ensure_model(&state.profile);

    println!("AiSH provider shell");
    println!("AI Run mode. Type natural language, direct commands, or /help.");

    loop {
        print!("{}> ", prompt_cwd());
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() { break; }
        let input = input.trim();
        if input.is_empty() { continue; }

        if input.starts_with('/') && !input.starts_with("//") {
            if handle_slash(input, &mut state) { break; }
            continue;
        }

        let input = input.strip_prefix("//").map(str::trim).unwrap_or(input);
        if looks_like_direct_command(input) {
            run_shell_command(input);
        } else {
            run_ai_request(input, &mut state);
        }
    }
}

fn install_prompt_env() {
    env::set_var("AISH_TARGET_OS", env::consts::OS);
    env::set_var("AISH_TARGET_SHELL", shell_name());
}

fn handle_slash(input: &str, state: &mut ProviderState) -> bool {
    let mut parts = input.split_whitespace();
    match parts.next().unwrap_or_default() {
        "/exit" | "/quit" => return true,
        "/help" => print_help(),
        "/setup" => setup::handle_setup_args(),
        "/status" => {
            println!("os: {}", env::consts::OS);
            println!("shell: {}", shell_name());
            println!("model: {}", state.profile.label);
            println!("model_path: {}", state.profile.model_path);
            println!("llama_cli: {}", state.profile.llama_cli_path);
        }
        "/model" => match (parts.next(), parts.next()) {
            (None, _) => println!("model: {}", state.profile.label),
            (Some("list"), _) => println!("{}", state.profile.label),
            (Some("use"), Some(_)) => println!("Only Qwen2.5 Coder 1.5B is enabled in this build."),
            _ => println!("usage: /model | /model list | /model use qwen2.5-coder"),
        },
        "/reasoning" | "/working" => match parts.next() {
            Some("on") => { state.show_trace = true; println!("full working trace: on"); }
            Some("off") => { state.show_trace = false; println!("full working trace: off"); }
            _ => println!("full working trace: {}", if state.show_trace { "on" } else { "off" }),
        },
        "/approve" => {
            if let Some(pending) = state.pending.take() {
                println!("approved: {}", pending.command);
                run_shell_command(&pending.command);
            } else {
                println!("no pending command");
            }
        }
        "/cancel" => {
            if state.pending.take().is_some() { println!("pending command cancelled"); } else { println!("no pending command"); }
        }
        _ => println!("unknown slash command. Try /help."),
    }
    false
}

fn print_help() {
    println!("AiSH slash commands:");
    println!("  /model                 show current model");
    println!("  /model list            list enabled models");
    println!("  /status                show provider status");
    println!("  /setup                 run setup wizard");
    println!("  /reasoning on|off      toggle full working trace");
    println!("  /working on|off        alias for reasoning trace");
    println!("  /approve               approve pending risky command");
    println!("  /cancel                cancel pending risky command");
    println!("  /exit                  exit provider shell");
    println!("  //text                 send a literal slash-prefixed line");
}

fn run_ai_request(intent: &str, state: &mut ProviderState) {
    let prompt = build_command_card_prompt(intent, &serde_json::json!({}));
    let result = run_gguf_model(ModelRunRequest { profile: state.profile.clone(), prompt });
    let Ok(result) = result else {
        println!("AiSH model error: {}", result.err().unwrap_or_else(|| "unknown error".to_string()));
        return;
    };

    let body = result.output.trim();
    let Ok(card) = serde_json::from_str::<CommandCard>(body) else {
        println!("AiSH could not parse a command card.");
        if state.show_trace { println!("raw: {body}"); }
        return;
    };

    if card.action_type == "fallback_message" {
        println!("{}", card.fallback_message.unwrap_or_else(|| card.reason.unwrap_or_else(|| "No command available.".to_string())));
        return;
    }

    let Some(command) = card.command.as_deref().map(str::trim).filter(|value| !value.is_empty()) else {
        println!("AiSH returned no command.");
        return;
    };

    let risk = classify_risk(card.risk.as_deref().unwrap_or("medium"), command);
    let reason = card.reason.unwrap_or_else(|| "No reason supplied.".to_string());

    if state.show_trace {
        println!("working: request: {intent}");
        println!("working: shell: {command}");
        println!("working: risk: {risk}");
        println!("working: reason: {reason}");
    }

    if risk == "low" {
        run_shell_command(command);
    } else {
        state.pending = Some(PendingCommand { command: command.to_string(), risk: risk.to_string(), reason: reason.clone() });
        println!("AiSH needs approval: {risk}");
        println!("reason: {reason}");
        println!("command: {command}");
        println!("type /approve or /cancel");
    }
}

fn run_shell_command(command: &str) {
    if handle_cd(command) { return; }
    let output = if env::consts::OS == "windows" {
        Command::new("powershell.exe").args(["-NoLogo", "-NoProfile", "-Command", command]).output()
    } else {
        Command::new(shell_path()).args(["-lc", command]).output()
    };
    match output {
        Ok(output) => {
            print!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
        Err(error) => eprintln!("failed to run command: {error}"),
    }
}

fn handle_cd(command: &str) -> bool {
    let trimmed = command.trim();
    let lower = trimmed.to_lowercase();
    let target = if lower == "cd" || lower == "set-location" {
        home_dir()
    } else if lower.starts_with("cd ") {
        PathBuf::from(unquote(&trimmed[3..]))
    } else if lower.starts_with("set-location ") {
        PathBuf::from(unquote(&trimmed[13..]))
    } else {
        return false;
    };
    if let Err(error) = env::set_current_dir(expand_home(target)) { eprintln!("cd failed: {error}"); }
    true
}

fn looks_like_direct_command(input: &str) -> bool {
    let first = input.split_whitespace().next().unwrap_or_default().to_lowercase();
    let direct = ["cd", "dir", "ls", "pwd", "cat", "type", "echo", "clear", "cls", "git", "npm", "pnpm", "yarn", "bun", "node", "python", "pip", "cargo", "go", "docker", "kubectl", "where", "which", "grep", "find", "get-childitem", "get-location", "select-string"];
    direct.contains(&first.as_str()) || input.contains('|') || input.contains("&&")
}

fn classify_risk(model_risk: &str, command: &str) -> &'static str {
    let lower = format!(" {} ", command.to_lowercase());
    let risky = ["remove".to_string() + "-item", " rm ".to_string(), " del ".to_string(), " erase ".to_string(), " rmdir ".to_string(), "git ".to_string() + "reset", "git ".to_string() + "clean", "git ".to_string() + "push", "npm ".to_string() + "publish", "deploy".to_string(), "format ".to_string(), "reg ".to_string() + "add", "reg ".to_string() + "delete", "chmod".to_string(), "chown".to_string(), "stop".to_string() + "-service", "shutdown".to_string(), "terraform ".to_string() + "apply", "kubectl ".to_string() + "delete"];
    if risky.iter().any(|item| lower.contains(item)) { "high" } else if model_risk.eq_ignore_ascii_case("low") || is_read_only(command) { "low" } else { "medium" }
}

fn is_read_only(command: &str) -> bool {
    let lower = command.trim().to_lowercase();
    let prefixes = ["get-", "dir", "ls", "pwd", "cat", "type", "grep", "find", "where", "which", "git status", "git log", "npm list", "npm run", "node -v", "python --version"];
    prefixes.iter().any(|prefix| lower.starts_with(prefix)) || lower.contains("select-object") || lower.contains("sort-object")
}

fn default_profile() -> ModelProfile {
    let home = home_dir().display().to_string().replace('\\', "/");
    let model_path = env::var("AISH_MODEL_PATH").unwrap_or_else(|_| format!("{home}/Downloads/aish-model/models/Qwen2.5-Coder-1.5B-Instruct-Q4_K_M.gguf"));
    let llama_cli_path = env::var("AISH_LLAMA_CLI").unwrap_or_else(|_| if env::consts::OS == "windows" { format!("{home}/Downloads/llama.cpp/build/bin/Release/llama-cli.exe") } else { "llama-cli".to_string() });
    ModelProfile { id: "qwen25-coder-15b-q4-k-m".to_string(), label: "Qwen2.5 Coder 1.5B Instruct Q4_K_M".to_string(), family: "qwen2.5-coder".to_string(), model_path, llama_cli_path, context_tokens: 4096, max_tokens: 192, temperature: 0.1 }
}

fn prompt_cwd() -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    format!("aish {}", cwd.display())
}

fn shell_name() -> String {
    if env::consts::OS == "windows" { env::var("AISH_SHELL").unwrap_or_else(|_| "powershell".to_string()) } else { Path::new(&shell_path()).file_name().and_then(|value| value.to_str()).unwrap_or("sh").to_string() }
}

fn shell_path() -> String {
    env::var("SHELL").unwrap_or_else(|_| if env::consts::OS == "macos" { "/bin/zsh".to_string() } else { "/bin/bash".to_string() })
}

fn home_dir() -> PathBuf {
    env::var("USERPROFILE").or_else(|_| env::var("HOME")).map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."))
}

fn expand_home(path: PathBuf) -> PathBuf {
    let text = path.display().to_string();
    if let Some(rest) = text.strip_prefix("~/") { home_dir().join(rest) } else if text == "~" { home_dir() } else { path }
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').trim_matches('\'').to_string()
}
