mod logging;
mod setup;

use aish_ai::{build_command_card_prompt, run_gguf_model, ModelProfile, ModelRunRequest};
use aish_core::RiskLevel;
use aish_safety::classify_risk;
use serde::Deserialize;
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const CREATOR: &str = "Dawnlight Labs";
const COPYRIGHT: &str = "Copyright (c) 2026 Dawnlight Labs. All rights reserved.";

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
    intent: Option<String>,
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
    println!("{COPYRIGHT}");
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
            approve_or_run(input, None, &mut state, None);
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
        "/setup" => setup::run_setup_wizard(false),
        "/status" => {
            let settings = logging::read_settings();
            println!("creator: {CREATOR}");
            println!("copyright: {COPYRIGHT}");
            println!("os: {}", env::consts::OS);
            println!("shell: {}", shell_name());
            println!("model: {}", state.profile.label);
            println!("model_path: {}", state.profile.model_path);
            println!("llama_cli: {}", state.profile.llama_cli_path);
            println!("command_log_policy: {}", logging::describe_policy(&settings.command_log_policy));
            println!("command_log_path: {}", logging::command_log_path().display());
            println!("crash_log_sharing_opt_in: {}", settings.crash_log_sharing_opt_in);
        }
        "/logs" => match parts.next() {
            None => {
                let settings = logging::read_settings();
                println!("command log policy: {}", logging::describe_policy(&settings.command_log_policy));
                println!("command log path: {}", logging::command_log_path().display());
                println!("usage: /logs off | /logs failed | /logs all");
            }
            Some(value) => match logging::parse_policy(value) {
                Some(policy) => match logging::set_policy(policy) {
                    Ok(settings) => println!("command log policy: {}", logging::describe_policy(&settings.command_log_policy)),
                    Err(error) => eprintln!("failed to save log settings: {error}"),
                },
                None => println!("usage: /logs off | /logs failed | /logs all"),
            },
        },
        "/crash-reports" | "/crash" => match parts.next() {
            None => {
                let settings = logging::read_settings();
                println!("crash-log sharing opt-in: {}", settings.crash_log_sharing_opt_in);
                println!("AiSH stores logs locally in this build and does not upload them.");
                println!("usage: /crash-reports on | /crash-reports off");
            }
            Some("on") | Some("yes") => match logging::set_crash_log_sharing(true) {
                Ok(_) => println!("crash-log sharing preference: on"),
                Err(error) => eprintln!("failed to save crash-log preference: {error}"),
            },
            Some("off") | Some("no") => match logging::set_crash_log_sharing(false) {
                Ok(_) => println!("crash-log sharing preference: off"),
                Err(error) => eprintln!("failed to save crash-log preference: {error}"),
            },
            _ => println!("usage: /crash-reports on | /crash-reports off"),
        },
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
                println!("approved: {} ({})", pending.command, pending.risk);
                println!("reason: {}", pending.reason);
                let ok = run_shell_command(&pending.command);
                logging::record_command(
                    pending.intent.as_deref(),
                    Some(&pending.command),
                    if ok { "success" } else { "failed" },
                    Some(&pending.risk),
                    Some(&pending.reason),
                    if ok { None } else { Some("command exited unsuccessfully") },
                );
            } else {
                println!("no pending command");
            }
        }
        "/cancel" => {
            if let Some(pending) = state.pending.take() {
                logging::record_command(
                    pending.intent.as_deref(),
                    Some(&pending.command),
                    "cancelled",
                    Some(&pending.risk),
                    Some(&pending.reason),
                    None,
                );
                println!("pending command cancelled");
            } else {
                println!("no pending command");
            }
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
    println!("  /logs                  show local command log settings");
    println!("  /logs off|failed|all   set local command log policy");
    println!("  /crash-reports on|off  set saved crash-log sharing preference");
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
        let error = result.err().unwrap_or_else(|| "unknown error".to_string());
        println!("AiSH model error: {error}");
        logging::record_command(Some(intent), None, "error", None, None, Some(&error));
        return;
    };

    let body = result.output.trim();
    let Ok(card) = serde_json::from_str::<CommandCard>(body) else {
        println!("AiSH could not parse a command card.");
        if state.show_trace { println!("raw: {body}"); }
        logging::record_command(Some(intent), None, "error", None, None, Some("could not parse command card"));
        return;
    };

    if card.action_type == "fallback_message" {
        let message = card.fallback_message.unwrap_or_else(|| card.reason.unwrap_or_else(|| "No command available.".to_string()));
        println!("{message}");
        logging::record_command(Some(intent), None, "fallback", None, Some(&message), None);
        return;
    }

    let Some(command) = card.command.as_deref().map(str::trim).filter(|value| !value.is_empty()) else {
        println!("AiSH returned no command.");
        logging::record_command(Some(intent), None, "error", card.risk.as_deref(), card.reason.as_deref(), Some("empty command"));
        return;
    };

    let reason = card.reason.unwrap_or_else(|| "No reason supplied.".to_string());

    if state.show_trace {
        println!("working: request: {intent}");
        println!("working: shell: {command}");
        println!("working: reason: {reason}");
    }

    approve_or_run(command, Some((card.risk.as_deref().unwrap_or("medium"), &reason)), state, Some(intent));
}

fn approve_or_run(command: &str, model_assessment: Option<(&str, &str)>, state: &mut ProviderState, intent: Option<&str>) {
    let local = classify_risk(command);
    let local_reason = local.reason.clone();
    let model_risk = model_assessment.map(|(risk, _)| risk).unwrap_or("low");
    let risk = combined_risk(&local.risk, model_risk);
    let needs_confirmation = local.needs_confirmation || risk != "low";
    let reason = if local.needs_confirmation {
        local_reason.clone()
    } else {
        model_assessment
            .map(|(_, reason)| reason.to_string())
            .unwrap_or(local_reason.clone())
    };

    if state.show_trace {
        println!("working: risk: {risk}");
        println!("working: safety: {local_reason}");
    }

    if needs_confirmation {
        state.pending = Some(PendingCommand {
            intent: intent.map(str::to_string),
            command: command.to_string(),
            risk: risk.to_string(),
            reason: reason.clone(),
        });
        println!("AiSH needs approval: {risk}");
        println!("reason: {reason}");
        println!("command: {command}");
        println!("type /approve or /cancel");
        logging::record_command(intent, Some(command), "approval_required", Some(risk), Some(&reason), None);
    } else {
        let ok = run_shell_command(command);
        logging::record_command(
            intent,
            Some(command),
            if ok { "success" } else { "failed" },
            Some(risk),
            Some(&reason),
            if ok { None } else { Some("command exited unsuccessfully") },
        );
    }
}

fn combined_risk(local: &RiskLevel, model_risk: &str) -> &'static str {
    if matches!(local, RiskLevel::High) || model_risk.eq_ignore_ascii_case("high") {
        "high"
    } else if matches!(local, RiskLevel::Medium) || !model_risk.eq_ignore_ascii_case("low") {
        "medium"
    } else {
        "low"
    }
}

fn run_shell_command(command: &str) -> bool {
    if let Some(ok) = handle_cd(command) { return ok; }
    let output = if env::consts::OS == "windows" {
        Command::new("powershell.exe").args(["-NoLogo", "-NoProfile", "-Command", command]).output()
    } else {
        Command::new(shell_path()).args(["-lc", command]).output()
    };
    match output {
        Ok(output) => {
            print!("{}", String::from_utf8_lossy(&output.stdout));
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            output.status.success()
        }
        Err(error) => {
            eprintln!("failed to run command: {error}");
            false
        }
    }
}

fn handle_cd(command: &str) -> Option<bool> {
    let trimmed = command.trim();
    let lower = trimmed.to_lowercase();
    let target = if lower == "cd" || lower == "set-location" {
        home_dir()
    } else if lower.starts_with("cd ") {
        PathBuf::from(unquote(&trimmed[3..]))
    } else if lower.starts_with("set-location ") {
        PathBuf::from(unquote(&trimmed[13..]))
    } else {
        return None;
    };
    if let Err(error) = env::set_current_dir(expand_home(target)) {
        eprintln!("cd failed: {error}");
        Some(false)
    } else {
        Some(true)
    }
}

fn looks_like_direct_command(input: &str) -> bool {
    let first = input.split_whitespace().next().unwrap_or_default().to_lowercase();
    let direct = ["cd", "dir", "ls", "pwd", "cat", "type", "echo", "clear", "cls", "git", "npm", "pnpm", "yarn", "bun", "node", "python", "pip", "cargo", "go", "docker", "kubectl", "where", "which", "grep", "find", "rm", "rmdir", "del", "erase", "unlink", "shred", "mv", "cp", "mkdir", "touch", "remove-item", "clear-content", "move-item", "rename-item", "copy-item", "set-content", "add-content", "out-file", "get-childitem", "get-location", "select-string"];
    direct.contains(&first.as_str()) || input.contains('|') || input.contains("&&")
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
