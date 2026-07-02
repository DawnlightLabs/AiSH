use aish_core::{CommandTrace, RiskLevel};
use aish_safety::classify_risk;
use std::process::Command;
use std::time::Instant;

pub fn run_shell_command(command: String, allow_medium_risk: bool) -> Result<CommandTrace, String> {
    let risk = classify_risk(&command);
    if risk.needs_confirmation && !allow_medium_risk {
        return Ok(CommandTrace {
            intent: command.clone(),
            card_type: "command".to_string(),
            risk: risk.risk,
            context_used: vec!["shell".to_string(), "cwd".to_string()],
            commands: vec![command],
            exit_code: None,
            duration_ms: None,
            safety_decision: risk.reason,
            output: String::new(),
            error: "Confirmation required before execution.".to_string(),
        });
    }

    let started = Instant::now();
    let output = Command::new("powershell.exe")
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-Command")
        .arg(&command)
        .output()
        .map_err(|error| format!("Failed to run PowerShell command: {error}"))?;

    let exit_code = output.status.code();
    Ok(CommandTrace {
        intent: command.clone(),
        card_type: "command".to_string(),
        risk: if risk.needs_confirmation {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        },
        context_used: vec!["shell".to_string(), "cwd".to_string()],
        commands: vec![command],
        exit_code,
        duration_ms: Some(started.elapsed().as_millis()),
        safety_decision: risk.reason,
        output: String::from_utf8_lossy(&output.stdout).to_string(),
        error: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}
