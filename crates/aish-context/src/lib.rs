use serde::{Deserialize, Serialize};
use std::path::Path;

mod project_runs;
pub use project_runs::{ProjectRunCandidate, ProjectRunnerKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub cwd: String,
    pub project_type: Option<String>,
    pub package_manager: Option<String>,
    pub dependencies_installed: Option<bool>,
    pub detected_files: Vec<String>,
    pub available_tools: Vec<String>,
    pub available_tasks: Vec<String>,
    pub run_candidates: Vec<ProjectRunCandidate>,
}

pub fn inspect_current_project() -> ProjectContext {
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    inspect_project(cwd)
}

pub fn inspect_project(cwd: impl AsRef<Path>) -> ProjectContext {
    let cwd = cwd.as_ref();
    let candidates = [
        "package.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "package-lock.json",
        "bun.lockb",
        "Dockerfile",
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
        "pubspec.yaml",
        "firebase.json",
        "vercel.json",
        "netlify.toml",
        "wrangler.toml",
        "Cargo.toml",
        "pyproject.toml",
        "requirements.txt",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "Package.swift",
        "deno.json",
        "deno.jsonc",
        "composer.json",
        "Gemfile",
        "Makefile",
        "justfile",
        "mix.exs",
        "stack.yaml",
        "build.zig",
    ];

    let detected_files: Vec<String> = candidates
        .iter()
        .filter(|name| cwd.join(name).exists())
        .map(|name| (*name).to_string())
        .collect();

    let package_manager = if detected_files.iter().any(|file| file == "pnpm-lock.yaml") {
        Some("pnpm".to_string())
    } else if detected_files.iter().any(|file| file == "yarn.lock") {
        Some("yarn".to_string())
    } else if detected_files.iter().any(|file| file == "bun.lockb") {
        Some("bun".to_string())
    } else if detected_files
        .iter()
        .any(|file| file == "package-lock.json" || file == "package.json")
    {
        Some("npm".to_string())
    } else {
        None
    };

    let project_type = if detected_files.iter().any(|file| file == "package.json") {
        Some("node".to_string())
    } else if detected_files.iter().any(|file| file == "Cargo.toml") {
        Some("rust".to_string())
    } else if detected_files
        .iter()
        .any(|file| file == "pyproject.toml" || file == "requirements.txt")
    {
        Some("python".to_string())
    } else if detected_files
        .iter()
        .any(|file| file == "docker-compose.yml" || file == "docker-compose.yaml")
    {
        Some("docker_compose".to_string())
    } else {
        None
    };
    let mut available_tasks = if detected_files.iter().any(|file| file == "package.json") {
        std::fs::read_to_string(cwd.join("package.json"))
            .ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .and_then(|package| {
                package
                    .get("scripts")
                    .and_then(|scripts| scripts.as_object())
                    .cloned()
            })
            .map(|scripts| scripts.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    available_tasks.sort_by_key(|task| task.to_ascii_lowercase());
    let run_candidates =
        project_runs::detect_run_candidates(cwd, &detected_files, &available_tasks);

    ProjectContext {
        cwd: cwd.display().to_string(),
        project_type,
        package_manager,
        dependencies_installed: detected_files
            .iter()
            .any(|file| file == "package.json")
            .then(|| cwd.join("node_modules").is_dir()),
        detected_files,
        available_tools: Vec::new(),
        available_tasks,
        run_candidates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn discovers_declared_node_tasks_without_inventing_any() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("aish-context-tasks-{unique}"));
        std::fs::create_dir_all(&root).expect("fixture directory");
        std::fs::write(
            root.join("package.json"),
            r#"{"scripts":{"test":"vitest","dev":"vite"}}"#,
        )
        .expect("fixture package");

        let context = inspect_project(&root);
        assert_eq!(context.project_type.as_deref(), Some("node"));
        assert_eq!(context.package_manager.as_deref(), Some("npm"));
        assert_eq!(context.dependencies_installed, Some(false));
        assert_eq!(context.available_tasks, ["dev", "test"]);
        assert_eq!(context.run_candidates.len(), 2);
        assert!(context.run_candidates.iter().any(|candidate| {
            candidate.kind == ProjectRunnerKind::NodeScript && candidate.name == "dev"
        }));

        std::fs::create_dir(root.join("node_modules")).expect("dependency directory");
        let ready = inspect_project(&root);
        assert_eq!(ready.dependencies_installed, Some(true));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn discovers_typed_runners_across_project_ecosystems() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("aish-context-runners-{unique}"));

        let rust = root.join("rust-app");
        std::fs::create_dir_all(rust.join("src")).expect("rust fixture");
        std::fs::write(
            rust.join("Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\n",
        )
        .expect("cargo manifest");
        std::fs::write(rust.join("src/main.rs"), "fn main() {}\n").expect("rust entrypoint");
        assert_eq!(
            inspect_project(&rust).run_candidates,
            [ProjectRunCandidate {
                kind: ProjectRunnerKind::CargoRun,
                name: "run".to_string(),
                entrypoint: None,
            }]
        );

        let python = root.join("python-app");
        std::fs::create_dir_all(&python).expect("python fixture");
        std::fs::write(
            python.join("app.py"),
            "from fastapi import FastAPI\napp = FastAPI()\n",
        )
        .expect("python entrypoint");
        assert_eq!(
            inspect_project(&python).run_candidates[0].kind,
            ProjectRunnerKind::FastApi
        );

        let dotnet = root.join("dotnet-app");
        std::fs::create_dir_all(&dotnet).expect("dotnet fixture");
        std::fs::write(dotnet.join("Fixture.csproj"), "<Project />\n").expect("dotnet project");
        assert_eq!(
            inspect_project(&dotnet).run_candidates[0].kind,
            ProjectRunnerKind::DotnetRun
        );

        let static_site = root.join("static-site");
        std::fs::create_dir_all(&static_site).expect("static fixture");
        std::fs::write(static_site.join("index.html"), "<!doctype html>\n")
            .expect("static entrypoint");
        assert_eq!(
            inspect_project(&static_site).run_candidates[0].kind,
            ProjectRunnerKind::StaticSite
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
