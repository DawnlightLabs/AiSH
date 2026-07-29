mod logging;
mod known_folders;
mod model_registry;
mod setup;
mod theme;
mod updater;

use aish_ai::ModelProfile;
use known_folders::KnownFolders;
use model_registry::ModelRegistry;
use aish_completion::demo_suggestions;
use aish_context::inspect_current_project;
use aish_core::RiskLevel;
use aish_provider::{
    build_provider_context, default_model_profile, describe_context_mode, describe_provider_mode,
    parse_context_mode, parse_provider_mode, plan_failed_command_recovery, plan_provider_input,
    split_planned_commands, trace_provider_plan, ProviderInputMode, ProviderPlan,
    ProviderPlanAction, ProviderPlanRequest, ProviderSession,
};
use std::collections::HashMap;
use std::env;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use theme::Theme;

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
    registry: ModelRegistry,
    pending: Option<PendingCommand>,
    session: ProviderSession,
    diagnostics: bool,
    theme: Theme,
    known_folders: KnownFolders,
}

#[derive(Debug)]
struct CommandOutcome {
    success: bool,
    exit_code: Option<i32>,
    stderr: String,
    had_output: bool,
}

fn main() {
    setup::handle_setup_args();
    if updater::handle_update_args() {
        return;
    }
    install_prompt_env();

    let initial = default_profile();
    let registry = ModelRegistry::discover(&initial.llama_cli_path);
    let profile = registry.active().cloned().unwrap_or(initial);
    let mut state = ProviderState {
        profile,
        registry,
        pending: None,
        session: ProviderSession::default(),
        diagnostics: env::var("AISH_DIAGNOSTICS").ok().as_deref() == Some("1"),
        theme: Theme::load(),
        known_folders: KnownFolders::discover(),
    };
    if handle_headless_route() {
        return;
    }
    setup::ensure_model(&state.profile);
    if handle_headless_plan(&state) {
        return;
    }

    println!("{}", state.theme.accent("AiSH provider shell"));
    println!(
        "{} {}",
        state.theme.muted("version:"),
        state.theme.command(updater::current_version())
    );
    println!("{}", state.theme.muted(COPYRIGHT));
    println!(
        "Mode: {}. Natural language is the default; use //command to force a literal shell command. Type /help for controls.",
        describe_provider_mode(&state.session.mode)
    );

    loop {
        print!("{}", state.theme.prompt(format!("{}> ", prompt_cwd(&state.session.mode))));
        let _ = io::stdout().flush();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        match classify_shell_input(input, &state.session.mode) {
            ShellInputRoute::SlashCommand => {
                if handle_slash(input, &mut state) {
                    break;
                }
                continue;
            }
            ShellInputRoute::ForcedLiteral => {
                let command = input.strip_prefix("//").map(str::trim).unwrap_or_default();
                run_user_command_or_recover(command, &mut state);
                continue;
            }
            ShellInputRoute::LiteralCommand => {
                run_user_command_or_recover(input, &mut state);
                continue;
            }
            ShellInputRoute::NaturalLanguage => {}
        }

        let plan = plan_provider_input(ProviderPlanRequest {
            mode: ProviderInputMode::AiRun,
            surface: "provider_shell".to_string(),
            input: input.to_string(),
            context_json: provider_context(&state),
            profile: Some(state.profile.clone()),
            diagnostics: state.diagnostics,
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
        "/help" => print_help(&state.theme),
        "/setup" | "/install" => setup::run_interactive_install(false),
        "/update" => {
            if updater::run_update_flow() {
                return true;
            }
        }
        "/version" => updater::print_version(),
        "/theme" => match (parts.next(), parts.next()) {
            (None, _) | (Some("status"), _) => {
                println!("theme: {}", state.theme.name());
                println!(
                    "color output: {}",
                    if state.theme.color_enabled() {
                        "active"
                    } else {
                        "disabled by terminal or environment"
                    }
                );
            }
            (Some("list"), _) => {
                for name in theme::preset_names() {
                    let marker = if *name == state.theme.name() { "*" } else { " " };
                    println!("{marker} {name}");
                }
            }
            (Some("preview"), _) => state.theme.preview(),
            (Some("use"), Some(name)) => match state.theme.use_preset(name) {
                Ok(()) => {
                    println!("theme: {}", state.theme.accent(state.theme.name()));
                    state.theme.preview();
                }
                Err(error) => println!("{}", state.theme.error(error)),
            },
            (Some("off"), _) => match state.theme.use_preset("off") {
                Ok(()) => println!("theme: off"),
                Err(error) => println!("{}", state.theme.error(error)),
            },
            _ => println!("usage: /theme list | /theme use <name> | /theme preview | /theme status | /theme off"),
        },
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
        "/status" => print_status(state),
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
            (None, _) | (Some("status"), _) => print_model_status(state),
            (Some("list"), _) => {
                if state.registry.models().is_empty() {
                    println!("No compatible GGUF models were discovered.");
                }
                for model in state.registry.models() {
                    let marker = if model.id == state.profile.id { "*" } else { " " };
                    println!("{marker} {}  [{}]", model.id, model.label);
                }
            }
            (Some("use"), Some(selector)) => match state.registry.use_model(selector) {
                Ok(model) => {
                    state.profile = model.clone();
                    println!("active model: {}", state.profile.label);
                }
                Err(error) => println!("{error}"),
            },
            _ => println!("usage: /model list | /model use <id> | /model status"),
        },
        "/diagnostics" => match parts.next() {
            Some("on") => {
                state.diagnostics = true;
                println!("planner diagnostics: on (sanitized and bounded)");
            }
            Some("off") => {
                state.diagnostics = false;
                println!("planner diagnostics: off");
            }
            _ => println!(
                "planner diagnostics: {}",
                if state.diagnostics { "on" } else { "off" }
            ),
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
        "/approve" => approve_pending(state),
        "/cancel" => cancel_pending(state),
        _ => println!("unknown slash command. Try /help."),
    }
    false
}

fn print_status(state: &ProviderState) {
    let settings = logging::read_settings();
    println!("creator: {CREATOR}");
    println!("copyright: {COPYRIGHT}");
    println!("version: {}", updater::current_version());
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
    println!("theme: {}", state.theme.name());
    println!("color_output: {}", state.theme.color_enabled());
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShellInputRoute {
    SlashCommand,
    ForcedLiteral,
    LiteralCommand,
    NaturalLanguage,
}

fn classify_shell_input(input: &str, mode: &ProviderInputMode) -> ShellInputRoute {
    classify_shell_input_with(input, mode, shell_resolves_command)
}

fn classify_shell_input_with(
    input: &str,
    mode: &ProviderInputMode,
    lookup: impl FnMut(&str) -> bool,
) -> ShellInputRoute {
    if input.starts_with("//") {
        ShellInputRoute::ForcedLiteral
    } else if input.starts_with('/') {
        ShellInputRoute::SlashCommand
    } else if *mode == ProviderInputMode::Normal || looks_like_command_attempt_with(input, lookup) {
        ShellInputRoute::LiteralCommand
    } else {
        ShellInputRoute::NaturalLanguage
    }
}

fn handle_headless_route() -> bool {
    let args = env::args().collect::<Vec<_>>();
    let inline_index = args.iter().position(|arg| arg == "--route-json");
    let file_index = args.iter().position(|arg| arg == "--route-json-file");
    if inline_index.is_none() && file_index.is_none() {
        return false;
    }
    if inline_index.is_some() && file_index.is_some() {
        eprintln!("use either --route-json or --route-json-file, not both");
        std::process::exit(2);
    }
    let index = inline_index.or(file_index).expect("route mode index");
    let Some(source) = args.get(index + 1).filter(|value| !value.trim().is_empty()) else {
        eprintln!(
            "usage: aish --route-json <input> | --route-json - | --route-json-file <path>"
        );
        std::process::exit(2);
    };

    let inputs = if file_index.is_some() {
        match std::fs::read_to_string(source) {
            Ok(content) => content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>(),
            Err(error) => {
                eprintln!("failed to read routing input file: {error}");
                std::process::exit(2);
            }
        }
    } else if source == "-" {
        io::stdin()
            .lock()
            .lines()
            .map_while(Result::ok)
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
    } else {
        vec![source.to_string()]
    };
    let mut resolution_cache = HashMap::new();
    for input in inputs {
        let route = classify_shell_input_with(&input, &ProviderInputMode::AiRun, |head| {
            if let Some(resolved) = resolution_cache.get(head) {
                return *resolved;
            }
            let resolved = shell_resolves_command(head);
            resolution_cache.insert(head.to_string(), resolved);
            resolved
        });
        let output = serde_json::json!({
            "input": input,
            "route": shell_input_route_name(route),
            "executed": false,
            "model_invoked": false,
        });
        println!("{output}");
    }
    true
}

fn shell_input_route_name(route: ShellInputRoute) -> &'static str {
    match route {
        ShellInputRoute::SlashCommand => "slash_command",
        ShellInputRoute::ForcedLiteral => "forced_literal",
        ShellInputRoute::LiteralCommand => "literal_command",
        ShellInputRoute::NaturalLanguage => "natural_language",
    }
}

fn handle_headless_plan(state: &ProviderState) -> bool {
    let args = env::args().collect::<Vec<_>>();
    let plan_index = args.iter().position(|arg| arg == "--plan-json");
    let recovery_index = args.iter().position(|arg| arg == "--recover-json");
    if plan_index.is_none() && recovery_index.is_none() {
        return false;
    }
    if plan_index.is_some() && recovery_index.is_some() {
        eprintln!("use either --plan-json or --recover-json, not both");
        std::process::exit(2);
    }
    let index = plan_index.or(recovery_index).expect("headless mode index");
    let Some(intent) = args.get(index + 1).filter(|value| !value.trim().is_empty()) else {
        eprintln!("usage: aish --plan-json <intent> | --recover-json <command> [--exit-code <code>] [--stderr <text>] [--model <id>] [--diagnostics]");
        std::process::exit(2);
    };
    let selected_model = cli_arg_value(&args, "--model");
    let profile = match selected_model {
        Some(selector) => match state.registry.model(selector) {
            Ok(profile) => profile.clone(),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        },
        None => state.profile.clone(),
    };
    eprintln!(
        "AiSH planner: loading {} (timeout: {}s)...",
        profile.label, profile.timeout_seconds
    );
    let diagnostics = args.iter().any(|arg| arg == "--diagnostics");
    let context = provider_context(state);
    let plan = if recovery_index.is_some() {
        let exit_code = cli_arg_value(&args, "--exit-code").and_then(|value| value.parse().ok());
        let stderr = cli_arg_value(&args, "--stderr").unwrap_or_default();
        plan_failed_command_recovery(
            intent,
            exit_code,
            stderr,
            "provider_shell_recovery_evaluation".to_string(),
            context,
            Some(profile),
            diagnostics,
        )
    } else {
        plan_provider_input(ProviderPlanRequest {
            mode: ProviderInputMode::AiRun,
            surface: "provider_shell_evaluation".to_string(),
            input: intent.to_string(),
            context_json: context,
            profile: Some(profile),
            diagnostics,
        })
    };
    match serde_json::to_string(&plan) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("failed to serialize planner result: {error}");
            std::process::exit(1);
        }
    }
    true
}

fn cli_arg_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}

fn print_model_status(state: &ProviderState) {
    println!("active model: {}", state.profile.label);
    println!("model id: {}", state.profile.id);
    println!("family: {}", state.profile.family);
    println!("model path: {}", state.profile.model_path);
    println!("runtime: {}", state.profile.llama_cli_path);
    println!(
        "acceleration: {}",
        crate::runtime_bootstrap::acceleration_status(Path::new(&state.profile.llama_cli_path))
    );
    println!("structured output: {}", state.profile.structured_output_strategy);
    println!("context tokens: {}", state.profile.context_tokens);
    println!("maximum output tokens: {}", state.profile.max_tokens);
    println!("discovered models: {}", state.registry.models().len());
}

fn approve_pending(state: &mut ProviderState) {
    if let Some(pending) = state.pending.take() {
        println!(
            "{} {} ({})",
            state.theme.success("approved:"),
            state.theme.command(&pending.command),
            state.theme.warning(&pending.risk)
        );
        println!("{} {}", state.theme.muted("reason:"), pending.reason);
        let outcome = run_planned_commands(&pending.command);
        print_empty_outcome(&outcome, false);
        logging::record_command(
            pending.intent.as_deref(),
            Some(&pending.command),
            if outcome.success { "success" } else { "failed" },
            Some(&pending.risk),
            Some(&pending.reason),
            if outcome.success {
                None
            } else {
                Some("command exited unsuccessfully")
            },
        );
        state.session.record_command(
            pending.intent.as_deref(),
            &pending.command,
            if outcome.success { "success" } else { "failed" },
            Some(&pending.reason),
        );
        state.session.record_turn(
            pending.intent.as_deref().unwrap_or(&pending.command),
            if outcome.success {
                "approved shell action completed successfully"
            } else {
                "approved shell action failed"
            },
        );
    } else {
        println!("{}", state.theme.warning("no pending command"));
    }
}

fn cancel_pending(state: &mut ProviderState) {
    if let Some(pending) = state.pending.take() {
        logging::record_command(
            pending.intent.as_deref(),
            Some(&pending.command),
            "cancelled",
            Some(&pending.risk),
            Some(&pending.reason),
            None,
        );
        state.session.record_turn(
            pending.intent.as_deref().unwrap_or(&pending.command),
            "pending shell action was cancelled",
        );
        println!("{}", state.theme.warning("pending command cancelled"));
    } else {
        println!("{}", state.theme.warning("no pending command"));
    }
}

fn print_help(theme: &Theme) {
    println!("{}", theme.accent("AiSH slash commands:"));
    println!("  /mode                  show current mode");
    println!("  /mode normal           pass input through as shell commands");
    println!("  /mode ai               treat non-command input as AI Run requests");
    println!("  /ai                    shortcut for /mode ai");
    println!("  /normal                shortcut for /mode normal");
    println!("  /complete [prefix]     show shared command completions");
    println!("  /model                 show current model");
    println!("  /model list            list enabled models");
    println!("  /model use <id>        persist and activate a discovered model");
    println!("  /model status          show model and runtime configuration");
    println!("  /theme                 show active color theme");
    println!("  /theme list            list cross-platform color themes");
    println!("  /theme use <name>      persist and activate a theme");
    println!("  /theme preview         preview semantic terminal colors");
    println!("  /theme off             disable AiSH colors");
    println!("  /diagnostics on|off    toggle sanitized planner diagnostics");
    println!("  /version               show installed AiSH version");
    println!("  /update                check latest release and install after approval");
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
    println!("  //command              force a literal shell command while AI mode is active");
}

fn set_mode(state: &mut ProviderState, mode: ProviderInputMode) {
    state.session.mode = mode;
    state.pending = None;
    println!("mode: {}", describe_provider_mode(&state.session.mode));
}

fn handle_plan(plan: ProviderPlan, state: &mut ProviderState) {
    if state.session.show_trace || state.diagnostics {
        print_plan_trace(&plan);
    }

    match &plan.action {
        ProviderPlanAction::Noop => {}
        ProviderPlanAction::Error => {
            let error = plan.error.as_deref().unwrap_or(&plan.reason);
            println!("{}", state.theme.error(format!("AiSH error: {error}")));
            state.session.record_turn(&plan.intent, error);
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
            state.session.record_turn(&plan.intent, message);
            logging::record_command(
                Some(&plan.intent),
                None,
                "fallback",
                Some(risk_label(&plan.risk)),
                Some(message),
                None,
            );
        }
        ProviderPlanAction::ChangeDirectory => {
            let Some(target) = plan.target.as_deref() else {
                println!(
                    "{}",
                    state
                        .theme
                        .error("AiSH could not resolve a directory for that request.")
                );
                return;
            };
            match env::set_current_dir(target) {
                Ok(()) => {
                    let current =
                        env::current_dir().unwrap_or_else(|_| PathBuf::from(target));
                    let visible = user_facing_path(&current);
                    println!(
                        "{} {}",
                        state.theme.success("directory:"),
                        state.theme.command(visible.display().to_string())
                    );
                    state.session.record_command(
                        Some(&plan.intent),
                        &visible.display().to_string(),
                        "success",
                        Some(&plan.reason),
                    );
                    state.session.record_turn(
                        &plan.intent,
                        &format!("changed directory to {}", visible.display()),
                    );
                }
                Err(error) => {
                    println!(
                        "{}",
                        state
                            .theme
                            .error(format!("Could not enter that directory: {error}"))
                    );
                    state
                        .session
                        .record_turn(&plan.intent, &format!("directory change failed: {error}"));
                }
            }
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

            println!(
                "{} {}",
                state.theme.warning("AiSH needs approval:"),
                state.theme.warning(risk_label(&plan.risk))
            );
            println!("{} {}", state.theme.muted("reason:"), plan.reason);
            println!(
                "{} {}",
                state.theme.muted("command:"),
                state.theme.command(command)
            );
            println!("{}", state.theme.accent("type /approve or /cancel"));
            state
                .session
                .record_turn(&plan.intent, &format!("approval required for: {command}"));

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

            let outcome = run_planned_commands(command);
            print_empty_outcome(&outcome, true);
            logging::record_command(
                Some(&plan.intent),
                Some(command),
                if outcome.success { "success" } else { "failed" },
                Some(risk_label(&plan.risk)),
                Some(&plan.reason),
                if outcome.success {
                    None
                } else {
                    Some("command exited unsuccessfully")
                },
            );
            state.session.record_command(
                Some(&plan.intent),
                command,
                if outcome.success { "success" } else { "failed" },
                Some(&plan.reason),
            );
            state.session.record_turn(
                &plan.intent,
                if outcome.success {
                    "shell action completed successfully"
                } else {
                    "shell action failed"
                },
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
    let mut project = inspect_current_project();
    project.cwd = user_facing_path(Path::new(&project.cwd))
        .display()
        .to_string();
    let mut base = serde_json::to_value(project).unwrap_or_else(|_| serde_json::json!({}));
    if let Some(object) = base.as_object_mut() {
        object.insert(
            "known_folders".to_string(),
            serde_json::to_value(&state.known_folders).unwrap_or_else(|_| serde_json::json!({})),
        );
    }
    build_provider_context(base, &state.session)
}

fn run_user_command_or_recover(command: &str, state: &mut ProviderState) {
    let outcome = run_shell_command(command);
    print_empty_success(&outcome);
    logging::record_command(
        None,
        Some(command),
        if outcome.success { "success" } else { "failed" },
        Some("user"),
        Some("User-entered command."),
        if outcome.success {
            None
        } else {
            Some("command exited unsuccessfully")
        },
    );
    state.session.record_command(
        None,
        command,
        if outcome.success { "success" } else { "failed" },
        Some("User-entered command."),
    );
    state.session.record_turn(
        command,
        if outcome.success {
            "literal command completed successfully"
        } else {
            "literal command failed"
        },
    );
    if outcome.success {
        return;
    }
    println!("AiSH detected that command failed. Trying to diagnose or correct it...");
    let recovery = plan_failed_command_recovery(
        command,
        outcome.exit_code,
        &outcome.stderr,
        "provider_shell".to_string(),
        provider_context(state),
        Some(state.profile.clone()),
        state.diagnostics,
    );
    handle_plan(recovery, state);
}

fn looks_like_command_attempt_with(
    input: &str,
    mut resolves_command: impl FnMut(&str) -> bool,
) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.ends_with('?') {
        return false;
    }

    if has_explicit_shell_syntax(trimmed) {
        return true;
    }

    let words = trimmed.split_whitespace().collect::<Vec<_>>();
    let first = words.first().copied().unwrap_or_default();
    // Resolution is performed without executing the user's input. Once a
    // command-shaped input has a resolvable head, its eventual exit status
    // cannot reclassify it as natural language; failures use recovery instead.
    if !resolves_command(first) {
        return false;
    }
    if words.len() <= 2
        || words.iter().skip(1).any(|word| {
            word.starts_with('-') || word == &"|"
        })
    {
        return true;
    }
    !has_natural_language_connectors(&words)
}

fn has_natural_language_connectors(words: &[&str]) -> bool {
    words.iter().skip(1).any(|word| {
        matches!(
            word.trim_matches(|character: char| !character.is_alphanumeric())
                .to_ascii_lowercase()
                .as_str(),
            "a" | "an"
                | "the"
                | "to"
                | "into"
                | "from"
                | "in"
                | "on"
                | "at"
                | "for"
                | "with"
                | "this"
                | "that"
                | "these"
                | "those"
                | "my"
                | "our"
                | "here"
                | "one"
                | "every"
                | "all"
                | "which"
                | "what"
                | "why"
                | "how"
        )
    })
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
        || [".exe", ".cmd", ".bat", ".ps1", ".sh"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

fn shell_resolves_command(first: &str) -> bool {
    if first.is_empty() {
        return false;
    }
    if env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|value| value.to_string_lossy().to_string()))
        .is_some_and(|name| name.eq_ignore_ascii_case(first))
    {
        return true;
    }
    if env::consts::OS == "windows" {
        Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "if (Get-Command -Name $env:AISH_LOOKUP -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }",
            ])
            .env("AISH_LOOKUP", first)
            .status()
            .is_ok_and(|status| status.success())
    } else {
        Command::new(shell_path())
            .args(["-lc", "command -v -- \"$AISH_LOOKUP\" >/dev/null 2>&1"])
            .env("AISH_LOOKUP", first)
            .status()
            .is_ok_and(|status| status.success())
    }
}

fn run_shell_command(command: &str) -> CommandOutcome {
    if let Some(ok) = handle_cd(command) {
        return CommandOutcome {
            success: ok,
            exit_code: Some(if ok { 0 } else { 1 }),
            stderr: String::new(),
            had_output: false,
        };
    }
    let mut process = if env::consts::OS == "windows" {
        let mut process = Command::new("powershell.exe");
        process.args(["-NoLogo", "-NoProfile", "-Command", command]);
        process
    } else {
        let mut process = Command::new(shell_path());
        process.args(["-lc", command]);
        process
    };
    if let Ok(cwd) = env::current_dir() {
        process.current_dir(child_working_directory(&cwd));
    };
    let output = process.output();
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            print!("{stdout}");
            eprint!("{stderr}");
            CommandOutcome {
                success: output.status.success(),
                exit_code: output.status.code(),
                stderr: stderr.chars().take(4000).collect(),
                had_output: !stdout.trim().is_empty() || !stderr.trim().is_empty(),
            }
        }
        Err(error) => {
            eprintln!("failed to run command: {error}");
            CommandOutcome {
                success: false,
                exit_code: None,
                stderr: error.to_string(),
                had_output: true,
            }
        }
    }
}

fn run_planned_commands(command: &str) -> CommandOutcome {
    let commands = split_planned_commands(command);
    if commands.len() <= 1 {
        return run_shell_command(command);
    }
    println!("AiSH plan: {} steps.", commands.len());
    let mut aggregate = CommandOutcome {
        success: true,
        exit_code: Some(0),
        stderr: String::new(),
        had_output: false,
    };
    for (index, step) in commands.iter().enumerate() {
        println!("step {}/{}: {step}", index + 1, commands.len());
        let outcome = run_shell_command(step);
        aggregate.had_output |= outcome.had_output;
        if !outcome.stderr.is_empty() {
            if !aggregate.stderr.is_empty() {
                aggregate.stderr.push('\n');
            }
            aggregate.stderr.push_str(&outcome.stderr);
            aggregate.stderr = aggregate.stderr.chars().take(4000).collect();
        }
        aggregate.exit_code = outcome.exit_code;
        if !outcome.success {
            aggregate.success = false;
            println!(
                "AiSH stopped the plan after step {} failed; later steps were not run.",
                index + 1
            );
            return aggregate;
        }
    }
    aggregate
}

fn print_empty_success(outcome: &CommandOutcome) {
    if outcome.success && !outcome.had_output {
        println!("Command completed successfully; it returned no output.");
    }
}

fn print_empty_outcome(outcome: &CommandOutcome, result_query: bool) {
    if let Some(message) = empty_outcome_message(outcome, result_query) {
        println!("{message}");
    }
}

fn empty_outcome_message(outcome: &CommandOutcome, result_query: bool) -> Option<String> {
    if outcome.had_output {
        return None;
    }
    if result_query {
        return Some(if outcome.success {
            "No matching results were found.".to_string()
        } else if let Some(exit_code) = outcome.exit_code {
            format!("No matching results were found (query exited with code {exit_code}).")
        } else {
            "The query failed without returning any results.".to_string()
        });
    }
    Some(if outcome.success {
        "Command completed successfully; it returned no output.".to_string()
    } else if let Some(exit_code) = outcome.exit_code {
        format!("Command returned no output and exited with code {exit_code}.")
    } else {
        "Command failed without returning output.".to_string()
    })
}

fn handle_cd(command: &str) -> Option<bool> {
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

fn default_profile() -> ModelProfile {
    default_model_profile()
}

fn prompt_cwd(mode: &ProviderInputMode) -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    format!(
        "aish:{} {}",
        describe_provider_mode(mode),
        user_facing_path(&cwd).display()
    )
}

fn user_facing_path(path: &Path) -> PathBuf {
    if !cfg!(windows) {
        return path.to_path_buf();
    }
    PathBuf::from(strip_windows_verbatim_path(&path.to_string_lossy()))
}

fn child_working_directory(path: &Path) -> PathBuf {
    user_facing_path(path)
}

fn strip_windows_verbatim_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = path.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        path.to_string()
    }
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

fn unquote(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        child_working_directory, classify_shell_input_with, cli_arg_value, empty_outcome_message,
        expand_shell_path, looks_like_command_attempt_with, run_planned_commands,
        run_shell_command,
        strip_windows_verbatim_path, ProviderInputMode, ShellInputRoute,
    };

    fn test_command_lookup(command: &str) -> bool {
        [
            "git",
            "Get-ChildItem",
            "go",
            "aish",
            "cargo",
            "npm",
            "open",
            "find",
        ]
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(command))
    }

    #[test]
    fn ai_mode_routes_navigation_language_to_the_planner() {
        assert!(!looks_like_command_attempt_with(
            "go to nebula-482",
            test_command_lookup
        ));
        assert!(!looks_like_command_attempt_with(
            "navigate to the nebula-482 folder",
            test_command_lookup
        ));
        assert!(!looks_like_command_attempt_with(
            "list the top-level folders",
            test_command_lookup
        ));
    }

    #[test]
    fn ai_mode_keeps_high_confidence_commands_literal() {
        for input in [
            "git pull",
            "git pulll",
            "git status",
            "Get-ChildItem -Directory",
            "go version",
            "aish --version",
            "aish -v",
            "npm run build",
            "git remote add upstream origin",
            ".\\tools\\build.ps1",
        ] {
            assert!(
                looks_like_command_attempt_with(input, test_command_lookup),
                "{input}"
            );
        }
    }

    #[test]
    fn resolved_command_failures_remain_literal_while_sentences_use_the_planner() {
        for input in ["git pull", "git pulll", "cargo tset", "npm isntall"] {
            assert_eq!(
                classify_shell_input_with(input, &ProviderInputMode::AiRun, test_command_lookup),
                ShellInputRoute::LiteralCommand,
                "{input}"
            );
        }
        for input in [
            "pull the latest updates",
            "test the entire workspace",
            "install the project dependencies",
            "go to the nearest folder",
            "find large files in this project",
            "open the folder containing manifest.json",
            "move one directory up",
        ] {
            assert_eq!(
                classify_shell_input_with(input, &ProviderInputMode::AiRun, test_command_lookup),
                ShellInputRoute::NaturalLanguage,
                "{input}"
            );
        }
    }

    #[test]
    fn navigation_expands_common_home_forms() {
        let home = super::home_dir();
        assert_eq!(
            expand_shell_path("$HOME\\Workspace Sample"),
            home.join("Workspace Sample")
        );
        assert_eq!(
            expand_shell_path("$env:USERPROFILE\\Workspace Sample"),
            home.join("Workspace Sample")
        );
    }

    #[test]
    fn strips_windows_verbatim_prefixes_from_user_visible_paths() {
        assert_eq!(
            strip_windows_verbatim_path(r"\\?\D:\workspace\crates"),
            r"D:\workspace\crates"
        );
        assert_eq!(
            strip_windows_verbatim_path(r"\\?\UNC\server\share\folder"),
            r"\\server\share\folder"
        );
        assert_eq!(
            strip_windows_verbatim_path(r"D:\workspace\crates"),
            r"D:\workspace\crates"
        );
    }

    #[cfg(windows)]
    #[test]
    fn child_processes_receive_a_drive_path_instead_of_a_verbatim_path() {
        assert_eq!(
            child_working_directory(std::path::Path::new(r"\\?\D:\workspace\crates")),
            std::path::PathBuf::from(r"D:\workspace\crates")
        );
    }

    #[test]
    fn detects_successful_commands_that_return_no_output() {
        let command = if cfg!(windows) { "exit 0" } else { "true" };
        let outcome = run_shell_command(command);
        assert!(outcome.success);
        assert!(!outcome.had_output);
    }

    #[test]
    fn detects_failed_commands_that_return_no_output() {
        let outcome = run_shell_command("exit 7");
        assert!(!outcome.success);
        assert_eq!(outcome.exit_code, Some(7));
        assert!(!outcome.had_output);
        assert_eq!(
            empty_outcome_message(&outcome, true).as_deref(),
            Some("No matching results were found (query exited with code 7).")
        );
    }

    #[test]
    fn multi_step_execution_stops_after_the_first_failure() {
        let outcome = run_planned_commands("exit 7; exit 0");
        assert!(!outcome.success);
        assert_eq!(outcome.exit_code, Some(7));
    }

    #[test]
    fn reads_headless_evaluation_arguments_without_shell_parsing() {
        let args = vec![
            "aish".to_string(),
            "--recover-json".to_string(),
            "tool --bad-flag".to_string(),
            "--exit-code".to_string(),
            "127".to_string(),
        ];
        assert_eq!(cli_arg_value(&args, "--exit-code"), Some("127"));
        assert_eq!(cli_arg_value(&args, "--stderr"), None);
    }

    #[test]
    fn shell_router_intercepts_slash_controls_and_forced_literals_first() {
        for input in [
            "/version",
            "/update",
            "/model list",
            "/theme preview",
            "/diagnostics on",
        ] {
            assert_eq!(
                classify_shell_input_with(input, &ProviderInputMode::AiRun, test_command_lookup),
                ShellInputRoute::SlashCommand,
                "{input}"
            );
        }
        assert_eq!(
            classify_shell_input_with(
                "//Get-ChildItem -Force",
                &ProviderInputMode::AiRun,
                test_command_lookup,
            ),
            ShellInputRoute::ForcedLiteral
        );
    }

    #[test]
    fn shell_router_handles_cross_platform_literals_and_natural_language() {
        for input in [
            "Get-ChildItem -Force",
            ".\\tools\\build.ps1",
            "./tools/build.sh --check",
            "cargo test",
            "npm install",
        ] {
            assert_eq!(
                classify_shell_input_with(input, &ProviderInputMode::AiRun, test_command_lookup),
                ShellInputRoute::LiteralCommand,
                "{input}"
            );
        }
        for input in [
            "show hidden files here",
            "check which process is using port 3000",
            "explain why the previous command failed",
        ] {
            assert_eq!(
                classify_shell_input_with(input, &ProviderInputMode::AiRun, test_command_lookup),
                ShellInputRoute::NaturalLanguage,
                "{input}"
            );
        }
    }
}
