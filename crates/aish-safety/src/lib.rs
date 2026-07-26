use aish_core::RiskLevel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDecision {
    pub risk: RiskLevel,
    pub needs_confirmation: bool,
    pub reason: String,
}

pub fn classify_risk(command: &str) -> RiskDecision {
    if let Some(inner) = unwrap_managed_cmd_command(command) {
        return classify_risk(&inner);
    }
    let normalized = format!(
        " {} ",
        command
            .to_lowercase()
            .replace(['\n', '\r', '\t', ';'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    );

    let long_search = normalized.contains("get-childitem")
        && normalized.contains("-recurse")
        && (normalized.contains("-path d:")
            || normalized.contains("-path d:\\")
            || normalized.contains("-path c:")
            || normalized.contains("-path c:\\"));

    if long_search {
        return RiskDecision {
            risk: RiskLevel::Low,
            needs_confirmation: false,
            reason: "Read-only recursive drive search.".to_string(),
        };
    }

    let destructive = [
        " remove-item ",
        " del ",
        " erase ",
        " rmdir ",
        " rm ",
        " rm -rf ",
        " clear-content ",
        " -delete ",
        "git reset --hard",
        "git clean",
        "docker system prune",
        "kubectl delete",
        "terraform destroy",
        "drop table",
        "drop database",
        " truncate table",
        " format.com",
        " format c:",
        " format d:",
        " shutdown",
        " restart-computer",
    ];

    if destructive
        .iter()
        .any(|pattern| normalized.contains(pattern))
    {
        return RiskDecision {
            risk: RiskLevel::High,
            needs_confirmation: true,
            reason: "Deletion, destructive, or production-impacting command detected.".to_string(),
        };
    }

    let mutating = [
        " set-content ",
        " add-content ",
        " out-file ",
        " new-item ",
        " set-item ",
        " set-itemproperty ",
        " new-itemproperty ",
        " move-item ",
        " rename-item ",
        " copy-item ",
        " mkdir ",
        " md ",
        " touch ",
        " tee ",
        " sed -i ",
        "npm install",
        "pnpm install",
        "yarn install",
        "bun install",
        "pip install",
        "uv pip install",
        "cargo install",
        "winget install",
        "choco install",
        "scoop install",
        "brew install",
        "apt install",
        "apt-get install",
        "dnf install",
        "pacman -s",
        "npm publish",
        "git push",
        "git commit",
        "git checkout",
        "git switch",
        "git merge",
        "docker compose up",
        "docker compose down",
        "docker run",
        "vercel deploy",
        "firebase deploy",
        "netlify deploy",
        "wrangler deploy",
        "terraform apply",
        " chmod ",
        " chown ",
        " set-executionpolicy ",
        " setx ",
        "[environment]::setenvironmentvariable",
        " $profile ",
        "sudo ",
        "doas ",
        "runas ",
        " -verb runas",
        " kill ",
        " stop-process ",
        " stop-service ",
        " start-service ",
        " set-service ",
        " reg add ",
        " reg delete ",
        " set-acl ",
        " icacls ",
        " takeown ",
        "az ",
        "aws ",
        "gcloud ",
    ];

    if contains_output_redirection(command)
        || mutating.iter().any(|pattern| normalized.contains(pattern))
    {
        return RiskDecision {
            risk: RiskLevel::Medium,
            needs_confirmation: true,
            reason: "May modify local dependencies, services, or remote/cloud state.".to_string(),
        };
    }

    let read_only = [
        "cd",
        "pwd",
        "ls",
        "dir",
        "echo",
        "cat",
        "type ",
        "head ",
        "tail ",
        "grep ",
        "find ",
        "which ",
        "where ",
        "where.exe ",
        "get-childitem",
        "get-location",
        "set-location",
        "select-string",
        "where-object",
        "select-object",
        "sort-object",
        "format-table",
        "format-list",
        "get-process",
        "get-service",
        "get-psdrive",
        "get-host",
        "get-command",
        "test-path",
        "write-output",
        "get-nettcpconnection",
        "test-netconnection",
        "resolve-dnsname",
        "measure-object",
        "compare-object",
        "lsof ",
        "ps ",
        "du ",
        "df ",
        "stat ",
        "test ",
        "wc ",
        "netstat ",
        "findstr ",
        "git status",
        "git log",
        "git diff",
        "git branch --show-current",
        "docker compose ps",
        "docker compose logs",
        "npm list",
        "node --version",
        "node -v",
        "npm --version",
        "npm -v",
        "python --version",
        "pip --version",
        "cargo --version",
        "cargo metadata",
        "rustc --version",
    ];
    let read_only_exact = [
        "git branch",
        "powershell -command \"$psversiontable.psversion\"",
        "powershell.exe -command \"$psversiontable.psversion\"",
    ];

    if is_read_only_pipeline(command, &read_only, &read_only_exact)
        || is_read_only_powershell_conditional(command, &read_only, &read_only_exact)
    {
        RiskDecision {
            risk: RiskLevel::Low,
            needs_confirmation: false,
            reason: "Recognized read-only or low-risk command.".to_string(),
        }
    } else {
        RiskDecision {
            risk: RiskLevel::Medium,
            needs_confirmation: true,
            reason: "Unrecognized generated command requires approval before execution."
                .to_string(),
        }
    }
}

fn is_read_only_powershell_conditional(
    command: &str,
    read_only: &[&str],
    read_only_exact: &[&str],
) -> bool {
    let trimmed = command.trim_start();
    if !trimmed
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("if"))
        || !trimmed
            .chars()
            .nth(2)
            .is_some_and(|ch| ch.is_whitespace() || ch == '(')
    {
        return false;
    }
    let segments = split_unquoted_control_blocks(command);
    let mut executable_segments = 0;
    for segment in segments {
        let normalized = segment.trim().to_ascii_lowercase();
        if normalized.is_empty() || matches!(normalized.as_str(), "if" | "else" | "elseif") {
            continue;
        }
        if !is_read_only_pipeline(&segment, read_only, read_only_exact) {
            return false;
        }
        executable_segments += 1;
    }
    executable_segments > 0
}

fn split_unquoted_control_blocks(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    for ch in command.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if (ch == '\\' && !single_quoted) || (ch == '`' && !single_quoted) {
            current.push(ch);
            escaped = true;
            continue;
        }
        match ch {
            '\'' if !double_quoted => {
                single_quoted = !single_quoted;
                current.push(ch);
            }
            '"' if !single_quoted => {
                double_quoted = !double_quoted;
                current.push(ch);
            }
            '(' | ')' | '{' | '}' | ';' if !single_quoted && !double_quoted => {
                let segment = current.trim();
                if !segment.is_empty() {
                    segments.push(segment.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let segment = current.trim();
    if !segment.is_empty() {
        segments.push(segment.to_string());
    }
    segments
}

fn unwrap_managed_cmd_command(command: &str) -> Option<String> {
    let trimmed = command.trim();
    let inner = trimmed
        .strip_prefix("cmd.exe /d /s /c '")
        .or_else(|| trimmed.strip_prefix("cmd /d /s /c '"))?
        .strip_suffix('\'')?;
    Some(inner.replace("''", "'"))
}

fn is_read_only_pipeline(command: &str, read_only: &[&str], read_only_exact: &[&str]) -> bool {
    let segments = split_unquoted_pipeline(command);
    !segments.is_empty()
        && segments.iter().all(|segment| {
            let normalized = segment
                .to_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            read_only_exact.contains(&normalized.as_str())
                || read_only.iter().any(|prefix| {
                    let prefix = prefix.trim_end();
                    normalized == prefix
                        || normalized.strip_prefix(prefix).is_some_and(|suffix| {
                            suffix.chars().next().is_some_and(char::is_whitespace)
                        })
                })
        })
}

fn split_unquoted_pipeline(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    for ch in command.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if (ch == '\\' && !single_quoted) || (ch == '`' && !single_quoted) {
            current.push(ch);
            escaped = true;
            continue;
        }
        match ch {
            '\'' if !double_quoted => {
                single_quoted = !single_quoted;
                current.push(ch);
            }
            '"' if !single_quoted => {
                double_quoted = !double_quoted;
                current.push(ch);
            }
            '|' | ';' | '&' if !single_quoted && !double_quoted => {
                let segment = current.trim();
                if segment.is_empty() {
                    return Vec::new();
                }
                segments.push(segment.to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let segment = current.trim();
    if segment.is_empty() {
        return Vec::new();
    }
    segments.push(segment.to_string());
    segments
}

fn contains_output_redirection(command: &str) -> bool {
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    for ch in command.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && !single_quoted {
            escaped = true;
            continue;
        }
        match ch {
            '\'' if !double_quoted => single_quoted = !single_quoted,
            '"' if !single_quoted => double_quoted = !double_quoted,
            '>' if !single_quoted && !double_quoted => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_risk(command: &str, risk: RiskLevel, confirmation: bool) {
        let decision = classify_risk(command);
        assert_eq!(decision.risk, risk, "{command}");
        assert_eq!(decision.needs_confirmation, confirmation, "{command}");
    }

    #[test]
    fn destructive_commands_are_high_risk() {
        for command in [
            "Remove-Item temp.txt",
            "git clean -fd",
            "find . -name '*.tmp' -delete",
            "del temp.txt",
        ] {
            assert_risk(command, RiskLevel::High, true);
        }
    }

    #[test]
    fn mutations_require_approval() {
        for command in [
            "npm install",
            "git push origin main",
            "echo value > file.txt",
            "echo value>file.txt",
            "docker compose down",
            "New-Item -ItemType Directory archive",
            "Rename-Item old.txt new.txt",
            "Set-ExecutionPolicy RemoteSigned",
            "setx PATH value",
            "sudo apt install tool",
        ] {
            assert_risk(command, RiskLevel::Medium, true);
        }
    }

    #[test]
    fn known_inspection_commands_are_low_risk() {
        for command in [
            "pwd",
            "ls -la",
            "git status --short",
            "git log --format=oneline",
            "Set-Location C:\\Users",
            "Get-ChildItem -Path D:\\ -Recurse -Filter foo",
            "Get-ChildItem -File | Where-Object { $_.Length -gt 10MB }",
            "netstat -ano | findstr :3000",
            "rustc --version",
            "Get-PSDrive -PSProvider FileSystem | Select-Object Name,Free",
            "Test-Path package.json",
            "Get-Command -Name rustc",
            "Get-Host | Select-Object Version",
            "git branch",
            "cargo metadata --format-version 1 --no-deps",
            "test -f package.json",
        ] {
            assert_risk(command, RiskLevel::Low, false);
        }
    }

    #[test]
    fn unknown_generated_commands_fail_closed_to_approval() {
        assert_risk("custom-tool inspect --json", RiskLevel::Medium, true);
        assert_risk(
            "netstat -ano | custom-tool inspect",
            RiskLevel::Medium,
            true,
        );
        assert_risk("git branch-delete", RiskLevel::Medium, true);
        assert_risk("rustc --version-and-write", RiskLevel::Medium, true);
        assert_risk("dir /a & custom-tool run", RiskLevel::Medium, true);
    }

    #[test]
    fn managed_cmd_wrappers_preserve_inner_host_risk() {
        assert_risk("cmd.exe /d /s /c 'dir /a /s'", RiskLevel::Low, false);
        assert_risk(
            "cmd.exe /d /s /c 'del /q important.txt'",
            RiskLevel::High,
            true,
        );
    }

    #[test]
    fn redirection_inside_quotes_is_not_treated_as_an_overwrite() {
        assert_risk("echo 'a > b'", RiskLevel::Low, false);
    }

    #[test]
    fn read_only_powershell_conditionals_are_low_risk_only_when_every_branch_is_known() {
        assert_risk(
            "if (Test-Path .\\package.json) { Write-Output 'exists' } else { Write-Output 'missing' }",
            RiskLevel::Low,
            false,
        );
        assert_risk(
            "if (Test-Path .\\package.json) { custom-tool inspect }",
            RiskLevel::Medium,
            true,
        );
        assert_risk(
            "if (Test-Path .\\package.json) { Set-Content result.txt 'yes' }",
            RiskLevel::Medium,
            true,
        );
    }
}
