use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CliToolSpec {
    pub name: &'static str,
    pub safe_templates: &'static [&'static str],
    pub confirmation_templates: &'static [&'static str],
    pub help_commands: &'static [&'static str],
}

pub fn built_in_registry() -> Vec<CliToolSpec> {
    vec![
        CliToolSpec {
            name: "npm",
            safe_templates: &["npm run", "npm run dev", "npm run build", "npm test"],
            confirmation_templates: &["npm install", "npm publish"],
            help_commands: &["npm --help", "npm help"],
        },
        CliToolSpec {
            name: "pnpm",
            safe_templates: &["pnpm run dev", "pnpm run build", "pnpm test"],
            confirmation_templates: &["pnpm install", "pnpm publish"],
            help_commands: &["pnpm --help"],
        },
        CliToolSpec {
            name: "git",
            safe_templates: &[
                "git status --short",
                "git branch --show-current",
                "git log --oneline -5",
                "git diff --stat",
            ],
            confirmation_templates: &["git reset --hard", "git clean -fd"],
            help_commands: &["git help", "git --version"],
        },
        CliToolSpec {
            name: "docker compose",
            safe_templates: &["docker compose ps", "docker compose logs --tail 100"],
            confirmation_templates: &["docker compose up --build"],
            help_commands: &["docker compose --help"],
        },
    ]
}
