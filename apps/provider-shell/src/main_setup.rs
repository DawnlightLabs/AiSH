mod logging;
mod setup;

use aish_ai::ModelProfile;
use aish_completion::demo_suggestions;
use aish_context::inspect_current_project;
use aish_core::RiskLevel;
use aish_provider::{
    build_provider_context, default_model_profile, describe_context_mode, describe_provider_mode,
    parse_context_mode, parse_provider_mode, plan_failed_command_recovery, plan_provider_input,
    trace_provider_plan, ProviderInputMode, ProviderPlan, ProviderPlanAction, ProviderPlanRequest,
    ProviderSession,
};
use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const CREATOR: &str = "Dawnlight Labs";
const COPYRIGHT: &str = "Copyright (c) 2026 Dawnlight Labs. All rights reserved.";

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
    session: ProviderSession,
}

fn main() {
    setup::handle_setup_args();
    install_prompt_env();

    let mut state = ProviderState {
        profile: default_profile(),
        pending: None,
        session: ProviderSession::default(),
    };
    setup::ensure_model(&state.profile);

    println!("AiSH provider shell");
    println!("{COPYRIGHT}");
    println!(
        "Mode: {}. Type /mode normal, /mode ai, commands, natural language, or /help.",
        describe_provider_mode(&state.session.mode)
    );

    loop {
        print!("{}> ", prompt_cwd(&state.session.mode));
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input.starts_with('/') && !input.starts_with("//") {
            if handle_slash(input, &mut state) {
                break;
            }
            continue;
        }

        if let Some(command) = input.strip_prefix("//").map(str::trim) {
            run_user_command_or_recover(command, &mut state);
            continue;
        }

        if state.session.mode == ProviderInputMode::Normal || looks_like_command_attempt(input) {
            run_user_command_or_recover(input, &mut state);
            continue;
        }

        let plan = plan_provider_input(ProviderPlanRequest {
            mode: ProviderInputMode::AiRun,
            surface: "provider_shell".to_string(),
            input: input.to_string(),
            context_json: provider_context(&state),
            profile: Some(state.profile.clone()),
        });

        handle_plan(plan, &mut state);
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
        "/ai" => set_mode(state, ProviderInputMode::AiRun),
        "/normal" => set_mode(state, ProviderInputMode::Normal),
        "/mode" => match parts.next() {
            None => println!("mode: {}", describe_provider_mode(&state.session.mode)),
            Some(value) => match parse_provider_mode(value) {
                Some(mode) => set_mode(state, mode),
                None => println!("usage: /mode normal | /mode ai"),
            },
        },
        "/context" => match parts.next() {
            None => {
                println!(
                    "context: {}",
                    describe_context_mode(&state.session.context_mode)
                );
                println!("session commands: {}", state.session.command_memory.len());
                println!("usage: /context off | /context auto | /context agent | /context clear");
            }
            Some("clear") => {
                state.session.clear_context();
                println!("context memory cleared");
            }
            Some(value) => match parse_context_mode(value) {
                Some(mode) => {
                    state.session.context_mode = mode;
                    println!(
                        "context: {}",
                        describe_context_mode(&state.session.context_mode)
                    );
                }
                None => println!(
                    "usage: /context off | /context auto | /context agent | /context clear"
                ),
            },
        },
        "/status" => {
            let settings = logging::read_settings();
            println!("creator: {CREATOR}");
            println!("copyright: {COPYRIGHT}");
            println!("mode: {}", describe_provider_mode(&state.session.mode));
            println!(
                "context: {}",
                describe_context_mode(&state.session.context_mode)
            );
            println!("pending_approval: {}", state.pending.is_some());
            println!("session_commands: {}", state.session.command_memory.len());
            println!("os: {}", env::consts::OS);
            println!("shell: {}", shell_name());
            println!("model: {}", state.profile.label);
            println!("model_path: {}", state.profile.model_path);
            println!("llama_cli: {}", state.profile.llama_cli_path);
            println!(
                "command_log_policy: {}",
                logging::describe_policy(&settings.command_log_policy)
            );
            println!(
                "command_log_path: {}",
                logging::command_log_path().display()
            );
            println!(
                "crash_log_sharing_opt_in: {}",
                settings.crash_log_sharing_opt_in
            );
        }
        "/logs" => match parts.next() {
            None => {
                let settings = logging::read_settings();
                println!(
                    "command log policy: {}",
                    logging::describe_policy(&settings.command_log_policy)
                );
                println!(
                    "command log path: {}",
                    logging::command_log_path().display()
                );
                println!("usage: /logs off | /logs failed | /logs all");
            }
            Some(value) => match logging::parse_policy(value) {
                Some(policy) => match logging::set_policy(policy) {
                    Ok(settings) => println!(
                        "command log policy: {}",
                        logging::describe_policy(&settings.command_log_policy)
                    ),
                    Err(error) => eprintln!("failed to save log settings: {error}"),
                },
                None => println!("usage: /logs off | /logs failed | /logs all"),
            },
        },
        "/crash-reports" | "/crash" => match parts.next() {
            None => {
                let settings = logging::read_settings();
                println!(
                    "crash-log sharing opt-in: {}",
                    settings.crash_log_sharing_opt_in
                );
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
        "/complete" => {
            let prefix = parts.collect::<Vec<_>>().join(" ");
            let suggestions = demo_suggestions(&prefix);
            if suggestions.is_empty() {
                println!("no completions");
            } else {
                for item in suggestions {
                    println!("{}    {} [{}]", item.command, item.description, item.source);
                }
            }
        }
        "/model" => match (parts.next(), parts.next()) {
            (None, _) => println!("model: {}", state.profile.label),
            (Some("list"), _) => println!("{}", state.profile.label),
            (Some("use"), Some(_)) => println!("Only Qwen2.5 Coder 1.5B is enabled in this build."),
            _ => println!("usage: /model | /model list | /model use qwen2.5-coder"),
        },
        "/reasoning" | "/working" => match parts.next() {
            Some("on") => {
                state.session.show_trace = true;
                println!("full working trace: on");
            }
            Some("off") => {
                state.session.show_trace = false;
                println!("full working trace: off");
            }
            _ => println!(
                "full working trace: {}",
                if state.session.show_trace {
                    "on"
                } else {
                    "off"
                }
            ),
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
                    if ok {
                        None
                    } else {
                        Some("command exited unsuccessfully")
                    },
                );
                state.session.record_command(
                    pending.intent.as_deref(),
                    &pending.command,
                    if ok { "success" } else { "failed" },
                    Some(&pending.reason),
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
    println!("  /mode                  show current mode");
    println!("  /mode normal           pass input through as shell commands");
    println!("  /mode ai               treat non-command input as AI Run requests");
    println!("  /ai                    shortcut for /mode ai");
    println!("  /normal                shortcut for /mode normal");
    println!("  /complete [prefix]     show shared command completions");
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

fn set_mode(state: &mut ProviderState, mode: ProviderInputMode) {
    state.session.mode = mode;
    state.pending = None;
    println!("mode: {}", describe_provider_mode(&state.session.mode));
}

fn handle_plan(plan: ProviderPlan, state: &mut ProviderState) {
    if state.session.show_trace {
        print_plan_trace(&plan);
    }

    match &plan.action {
        ProviderPlanAction::Noop => {}
        ProviderPlanAction::Error => {
            let error = plan.error.as_deref().unwrap_or(&plan.reason);
            println!("AiSH error: {error}");
            logging::record_command(
                Some(&plan.intent),
                plan.command.as_deref(),
                "error",
                Some(risk_label(&plan.risk)),
                Some(&plan.reason),
                Some(error),
            );
        }
        ProviderPlanAction::Fallback => {
            let message = plan.fallback_message.as_deref().unwrap_or(&plan.reason);
            println!("{message}");
            logging::record_command(
                Some(&plan.intent),
                None,
                "fallback",
                Some(risk_label(&plan.risk)),
                Some(message),
                None,
            );
        }
        ProviderPlanAction::ApprovalRequired => {
            let Some(command) = plan.command.as_deref() else {
                println!("AiSH needs approval but returned no command.");
                logging::record_command(
                    Some(&plan.intent),
                    None,
                    "error",
                    Some(risk_label(&plan.risk)),
                    Some(&plan.reason),
                    Some("approval missing command"),
                );
                return;
            };

            state.pending = Some(PendingCommand {
                intent: Some(plan.intent.clone()),
                command: command.to_string(),
                risk: risk_label(&plan.risk).to_string(),
                reason: plan.reason.clone(),
            });

            println!("AiSH needs approval: {}", risk_label(&plan.risk));
            println!("reason: {}", plan.reason);
            println!("command: {command}");
            println!("type /approve or /cancel");

            logging::record_command(
                Some(&plan.intent),
                Some(command),
                "approval_required",
                Some(risk_label(&plan.risk)),
                Some(&plan.reason),
                None,
            );
        }
        ProviderPlanAction::ShellCommand => {
            let Some(command) = plan.command.as_deref() else {
                println!("AiSH returned no command.");
                logging::record_command(
                    Some(&plan.intent),
                    None,
                    "error",
                    Some(risk_label(&plan.risk)),
                    Some(&plan.reason),
                    Some("missing command"),
                );
                return;
            };

            let ok = run_shell_command(command);
            logging::record_command(
                Some(&plan.intent),
                Some(command),
                if ok { "success" } else { "failed" },
                Some(risk_label(&plan.risk)),
                Some(&plan.reason),
                if ok {
                    None
                } else {
                    Some("command exited unsuccessfully")
                },
            );
            state.session.record_command(
                Some(&plan.intent),
                command,
                if ok { "success" } else { "failed" },
                Some(&plan.reason),
            );
        }
    }
}

fn risk_label(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "low",
        RiskLevel::Medium => "medium",
        RiskLevel::High => "high",
    }
}

fn print_plan_trace(plan: &ProviderPlan) {
    for event in trace_provider_plan(plan) {
        println!("working: {}: {}", event.key, event.value);
    }
}

fn provider_context(state: &ProviderState) -> serde_json::Value {
    let base =
        serde_json::to_value(inspect_current_project()).unwrap_or_else(|_| serde_json::json!({}));
    build_provider_context(base, &state.session)
}

fn run_user_command_or_recover(command: &str, state: &mut ProviderState) {
    let ok = run_shell_command(command);
    logging::record_command(
        None,
        Some(command),
        if ok { "success" } else { "failed" },
        Some("user"),
        Some("User-entered command."),
        if ok {
            None
        } else {
            Some("command exited unsuccessfully")
        },
    );
    state.session.record_command(
        None,
        command,
        if ok { "success" } else { "failed" },
        Some("User-entered command."),
    );
    if ok {
        return;
    }
    println!("AiSH detected that command failed. Trying to diagnose or correct it...");
    let recovery = plan_failed_command_recovery(
        command,
        "provider_shell".to_string(),
        provider_context(state),
        Some(state.profile.clone()),
    );
    handle_plan(recovery, state);
}

fn looks_like_command_attempt(input: &str) -> bool {
    if looks_like_direct_command(input) {
        return true;
    }

    let words: Vec<&str> = input.split_whitespace().collect();
    if words.is_empty() || input.ends_with('?') {
        return false;
    }

    let first = words[0].to_lowercase();
    let nlp_verbs = [
        "show", "find", "create", "make", "run", "install", "open", "explain", "what", "why",
        "how", "can", "please", "list", "tell", "check",
    ];

    if nlp_verbs.contains(&first.as_str()) {
        return false;
    }

    input.contains("--")
        || input.contains(" -")
        || input.contains('|')
        || input.contains("&&")
        || input.contains('\\')
        || input.contains('/')
        || input.contains('.')
        || (words.len() > 1
            && first
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
}

fn run_shell_command(command: &str) -> bool {
    if let Some(ok) = handle_cd(command) {
        return ok;
    }
    let output = if env::consts::OS == "windows" {
        Command::new("powershell.exe")
            .args(["-NoLogo", "-NoProfile", "-Command", command])
            .output()
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
    let first = input
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_lowercase();
    let direct = [
        "cd",
        "dir",
        "ls",
        "pwd",
        "cat",
        "type",
        "echo",
        "clear",
        "cls",
        "git",
        "npm",
        "pnpm",
        "yarn",
        "bun",
        "node",
        "python",
        "pip",
        "cargo",
        "go",
        "docker",
        "kubectl",
        "where",
        "which",
        "grep",
        "find",
        "rm",
        "rmdir",
        "del",
        "erase",
        "unlink",
        "shred",
        "mv",
        "cp",
        "mkdir",
        "touch",
        "remove-item",
        "clear-content",
        "move-item",
        "rename-item",
        "copy-item",
        "set-content",
        "add-content",
        "out-file",
        "get-childitem",
        "get-location",
        "select-string",
    ];
    direct.contains(&first.as_str()) || input.contains('|') || input.contains("&&")
}

fn default_profile() -> ModelProfile {
    default_model_profile()
}

fn prompt_cwd(mode: &ProviderInputMode) -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    format!("aish:{} {}", describe_provider_mode(mode), cwd.display())
}

fn shell_name() -> String {
    if env::consts::OS == "windows" {
        env::var("AISH_SHELL").unwrap_or_else(|_| "powershell".to_string())
    } else {
        Path::new(&shell_path())
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("sh")
            .to_string()
    }
}

fn shell_path() -> String {
    env::var("SHELL").unwrap_or_else(|_| {
        if env::consts::OS == "macos" {
            "/bin/zsh".to_string()
        } else {
            "/bin/bash".to_string()
        }
    })
}

fn home_dir() -> PathBuf {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn expand_home(path: PathBuf) -> PathBuf {
    let text = path.display().to_string();
    if let Some(rest) = text.strip_prefix("~/") {
        home_dir().join(rest)
    } else if text == "~" {
        home_dir()
    } else {
        path
    }
}

fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}
