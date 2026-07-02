use aish_core::RiskLevel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskDecision {
    pub risk: RiskLevel,
    pub needs_confirmation: bool,
    pub reason: String,
}

pub fn classify_risk(command: &str) -> RiskDecision {
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
        " > ",
        ">>",
        " set-content ",
        " add-content ",
        " out-file ",
        " move-item ",
        " rename-item ",
        " copy-item ",
        " mkdir ",
        "npm install",
        "pnpm install",
        "yarn install",
        "bun install",
        "pip install",
        "uv pip install",
        "cargo install",
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

    if mutating.iter().any(|pattern| normalized.contains(pattern)) {
        return RiskDecision {
            risk: RiskLevel::Medium,
            needs_confirmation: true,
            reason: "May modify local dependencies, services, or remote/cloud state.".to_string(),
        };
    }

    let trimmed = normalized.trim();
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
    ];

    if read_only
        .iter()
        .any(|prefix| trimmed == *prefix || trimmed.starts_with(prefix))
    {
        RiskDecision {
            risk: RiskLevel::Low,
            needs_confirmation: false,
            reason: "Recognized read-only or low-risk command.".to_string(),
        }
    } else {
        RiskDecision {
            risk: RiskLevel::Low,
            needs_confirmation: false,
            reason: "No destructive or mutating pattern detected.".to_string(),
        }
    }
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
            "docker compose down",
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
        ] {
            assert_risk(command, RiskLevel::Low, false);
        }
    }

    #[test]
    fn unknown_neutral_commands_do_not_require_approval() {
        assert_risk("custom-tool inspect --json", RiskLevel::Low, false);
    }
}
