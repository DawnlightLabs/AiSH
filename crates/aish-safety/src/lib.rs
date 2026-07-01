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
        && (normalized.contains("-path d:") || normalized.contains("-path d:\\") || normalized.contains("-path c:") || normalized.contains("-path c:\\"));

    if long_search {
        return RiskDecision {
            risk: RiskLevel::Medium,
            needs_confirmation: true,
            reason: "Broad recursive drive searches can take a long time in the current prototype.".to_string(),
        };
    }

    let destructive = [
        " rm ",
        " rmdir ",
        " unlink ",
        " shred ",
        " del ",
        " erase ",
        " remove-item ",
        " clear-content ",
        " -delete ",
        "rm -rf",
        "del /s /q",
        "rmdir /s",
        "git reset --hard",
        "git clean",
        "docker system prune",
        "kubectl delete",
        "terraform destroy",
        "drop table",
        "drop database",
        " truncate table",
        " mkfs",
        " format.com",
        " format c:",
        " format d:",
        " dd if=",
        " shutdown",
        " restart-computer",
    ];

    if destructive.iter().any(|pattern| normalized.contains(pattern)) {
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
        " mv ",
        " cp ",
        " mkdir ",
        " touch ",
        "npm install",
        "pnpm install",
        "yarn install",
        "pip install",
        "cargo install",
        "npm publish",
        "git push",
        "git commit",
        "git checkout",
        "git switch",
        "git merge",
        "docker compose up",
        "docker compose down",
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

    if read_only.iter().any(|prefix| trimmed == *prefix || trimmed.starts_with(prefix)) {
        RiskDecision {
            risk: RiskLevel::Low,
            needs_confirmation: false,
            reason: "Recognized read-only or low-risk command.".to_string(),
        }
    } else {
        RiskDecision {
            risk: RiskLevel::Medium,
            needs_confirmation: true,
            reason: "Command is not recognized as read-only; approval is required.".to_string(),
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
    fn deletion_commands_are_high_risk() {
        for command in [
            "rm file.txt",
            "rm -rf ./build",
            "Remove-Item temp.txt",
            "del temp.txt",
            "git clean -fd",
            "find . -name '*.tmp' -delete",
        ] {
            assert_risk(command, RiskLevel::High, true);
        }
    }

    #[test]
    fn mutations_require_approval() {
        for command in [
            "npm install",
            "git push origin main",
            "mv old.txt new.txt",
            "echo value > file.txt",
            "docker compose down",
        ] {
            assert_risk(command, RiskLevel::Medium, true);
        }
    }

    #[test]
    fn known_inspection_commands_are_low_risk() {
        for command in ["pwd", "ls -la", "git status --short", "git log --format=oneline"] {
            assert_risk(command, RiskLevel::Low, false);
        }
    }

    #[test]
    fn unknown_commands_fail_closed() {
        assert_risk("custom-tool do-something", RiskLevel::Medium, true);
    }
}
