use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    pub cwd: String,
    pub project_type: Option<String>,
    pub package_manager: Option<String>,
    pub detected_files: Vec<String>,
    pub available_tools: Vec<String>,
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

    ProjectContext {
        cwd: cwd.display().to_string(),
        project_type,
        package_manager,
        detected_files,
        available_tools: Vec::new(),
    }
}
