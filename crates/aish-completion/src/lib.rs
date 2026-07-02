use aish_core::RiskLevel;
use aish_provider::CompletionItem;

pub fn demo_suggestions(prefix: &str) -> Vec<CompletionItem> {
    let all = vec![
        item(
            "npm run dev",
            "Start the npm dev script",
            "package_scripts",
            0.92,
        ),
        item(
            "npm run build",
            "Build the npm project",
            "package_scripts",
            0.72,
        ),
        item("npm test", "Run npm tests", "package_scripts", 0.68),
        item("git status --short", "Show concise Git status", "git", 0.84),
        item(
            "git branch --show-current",
            "Show current Git branch",
            "git",
            0.70,
        ),
        item(
            "docker compose ps",
            "List Compose services",
            "docker_compose",
            0.65,
        ),
        item("python -m pytest", "Run Python tests", "python", 0.62),
    ];

    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        return all.into_iter().take(5).collect();
    }

    all.into_iter()
        .filter(|candidate| candidate.command.starts_with(trimmed))
        .collect()
}

fn item(command: &str, description: &str, source: &str, score: f32) -> CompletionItem {
    CompletionItem {
        kind: "command".to_string(),
        command: command.to_string(),
        display: command.to_string(),
        description: description.to_string(),
        source: source.to_string(),
        score,
        risk: RiskLevel::Low,
        needs_confirmation: false,
    }
}
