use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRunnerKind {
    NodeScript,
    NodeEntrypoint,
    CargoRun,
    PythonScript,
    Django,
    FastApi,
    Streamlit,
    GoRun,
    DotnetRun,
    MavenSpring,
    GradleBoot,
    GradleRun,
    PhpArtisan,
    PhpBuiltin,
    Rails,
    RubyScript,
    FlutterRun,
    DockerCompose,
    SwiftRun,
    DenoTask,
    DenoRun,
    MakeRun,
    JustRun,
    StaticSite,
    ShellScript,
    ElixirMix,
    Phoenix,
    HaskellStack,
    HaskellCabal,
    ZigBuild,
    RScript,
    Shiny,
    LuaScript,
    PerlScript,
    DartRun,
    JavaJar,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectRunCandidate {
    pub kind: ProjectRunnerKind,
    pub name: String,
    pub entrypoint: Option<String>,
}

pub fn detect_run_candidates(
    cwd: &Path,
    detected_files: &[String],
    available_tasks: &[String],
) -> Vec<ProjectRunCandidate> {
    let mut candidates = Vec::new();
    let has = |name: &str| detected_files.iter().any(|file| file == name);

    if has("package.json") {
        for task in available_tasks {
            candidates.push(candidate(ProjectRunnerKind::NodeScript, task, None));
        }
    }
    if !has("package.json") && cwd.join("main.js").is_file() {
        candidates.push(candidate(
            ProjectRunnerKind::NodeEntrypoint,
            "run",
            Some("main.js"),
        ));
    }
    if has("Cargo.toml") && cwd.join("src/main.rs").is_file() {
        candidates.push(candidate(ProjectRunnerKind::CargoRun, "run", None));
    }
    detect_python(cwd, &mut candidates);
    if has("go.mod") && cwd.join("main.go").is_file() {
        candidates.push(candidate(ProjectRunnerKind::GoRun, "run", None));
    }
    detect_dotnet(cwd, &mut candidates);
    detect_java(cwd, &mut candidates);
    detect_php(cwd, &mut candidates);
    detect_ruby(cwd, &mut candidates);
    if has("pubspec.yaml") && file_contains(&cwd.join("pubspec.yaml"), "flutter") {
        candidates.push(candidate(ProjectRunnerKind::FlutterRun, "run", None));
    }
    if [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ]
    .iter()
    .any(|name| has(name))
    {
        candidates.push(candidate(ProjectRunnerKind::DockerCompose, "compose", None));
    }
    if cwd.join("Package.swift").is_file() {
        candidates.push(candidate(ProjectRunnerKind::SwiftRun, "run", None));
    }
    detect_deno(cwd, &mut candidates);
    detect_task_files(cwd, &mut candidates);
    detect_additional_runtimes(cwd, &mut candidates);
    if !has("package.json")
        && cwd.join("index.html").is_file()
        && !candidates.iter().any(is_web_candidate)
    {
        candidates.push(candidate(ProjectRunnerKind::StaticSite, "serve", None));
    }

    candidates
}

fn detect_python(cwd: &Path, candidates: &mut Vec<ProjectRunCandidate>) {
    if cwd.join("manage.py").is_file() {
        candidates.push(candidate(
            ProjectRunnerKind::Django,
            "serve",
            Some("manage.py"),
        ));
        return;
    }
    for entrypoint in ["main.py", "app.py"] {
        let path = cwd.join(entrypoint);
        if !path.is_file() {
            continue;
        }
        let content = read_bounded(&path);
        let kind = if content.contains("FastAPI(") {
            ProjectRunnerKind::FastApi
        } else if content.contains("streamlit") || content.contains("st.") {
            ProjectRunnerKind::Streamlit
        } else {
            ProjectRunnerKind::PythonScript
        };
        candidates.push(candidate(kind, "run", Some(entrypoint)));
    }
}

fn detect_dotnet(cwd: &Path, candidates: &mut Vec<ProjectRunCandidate>) {
    let Ok(entries) = fs::read_dir(cwd) else {
        return;
    };
    for entry in entries.flatten().take(256) {
        let path = entry.path();
        let extension = path.extension().and_then(|value| value.to_str());
        if !matches!(extension, Some("csproj" | "fsproj" | "vbproj")) {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
            candidates.push(candidate(ProjectRunnerKind::DotnetRun, "run", Some(name)));
        }
    }
}

fn detect_java(cwd: &Path, candidates: &mut Vec<ProjectRunCandidate>) {
    let pom = cwd.join("pom.xml");
    if pom.is_file() && file_contains(&pom, "spring-boot") {
        candidates.push(candidate(ProjectRunnerKind::MavenSpring, "serve", None));
    }
    let gradle = if cwd.join("build.gradle.kts").is_file() {
        cwd.join("build.gradle.kts")
    } else {
        cwd.join("build.gradle")
    };
    if gradle.is_file() {
        let content = read_bounded(&gradle);
        let kind = if content.contains("org.springframework.boot") {
            ProjectRunnerKind::GradleBoot
        } else {
            ProjectRunnerKind::GradleRun
        };
        candidates.push(candidate(kind, "run", None));
    }
}

fn detect_php(cwd: &Path, candidates: &mut Vec<ProjectRunCandidate>) {
    if cwd.join("artisan").is_file() {
        candidates.push(candidate(
            ProjectRunnerKind::PhpArtisan,
            "serve",
            Some("artisan"),
        ));
    } else if cwd.join("index.php").is_file() {
        candidates.push(candidate(
            ProjectRunnerKind::PhpBuiltin,
            "serve",
            Some("index.php"),
        ));
    }
}

fn detect_ruby(cwd: &Path, candidates: &mut Vec<ProjectRunCandidate>) {
    if cwd.join("bin/rails").is_file() {
        candidates.push(candidate(
            ProjectRunnerKind::Rails,
            "serve",
            Some("bin/rails"),
        ));
    } else if cwd.join("app.rb").is_file() {
        candidates.push(candidate(
            ProjectRunnerKind::RubyScript,
            "run",
            Some("app.rb"),
        ));
    }
}

fn detect_deno(cwd: &Path, candidates: &mut Vec<ProjectRunCandidate>) {
    let manifest = ["deno.json", "deno.jsonc"]
        .iter()
        .map(|name| cwd.join(name))
        .find(|path| path.is_file());
    let has_manifest = manifest.is_some();
    if let Some(manifest) = manifest {
        if let Ok(content) = fs::read_to_string(manifest) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(tasks) = value.get("tasks").and_then(serde_json::Value::as_object) {
                    for task in tasks.keys() {
                        candidates.push(candidate(ProjectRunnerKind::DenoTask, task, None));
                    }
                }
            }
        }
    }
    if has_manifest
        && candidates
            .iter()
            .all(|item| item.kind != ProjectRunnerKind::DenoTask)
    {
        for entrypoint in ["main.ts", "main.js"] {
            if cwd.join(entrypoint).is_file() {
                candidates.push(candidate(
                    ProjectRunnerKind::DenoRun,
                    "run",
                    Some(entrypoint),
                ));
            }
        }
    }
}

fn detect_task_files(cwd: &Path, candidates: &mut Vec<ProjectRunCandidate>) {
    let makefile = cwd.join("Makefile");
    if makefile.is_file() && has_task(&makefile, "run") {
        candidates.push(candidate(ProjectRunnerKind::MakeRun, "run", None));
    }
    let justfile = cwd.join("justfile");
    if justfile.is_file() && has_task(&justfile, "run") {
        candidates.push(candidate(ProjectRunnerKind::JustRun, "run", None));
    }
}

fn detect_additional_runtimes(cwd: &Path, candidates: &mut Vec<ProjectRunCandidate>) {
    for entrypoint in [
        "run.ps1",
        "start.ps1",
        "run.sh",
        "start.sh",
        "run.cmd",
        "start.cmd",
        "run.bat",
        "start.bat",
    ] {
        if cwd.join(entrypoint).is_file() {
            candidates.push(candidate(
                ProjectRunnerKind::ShellScript,
                "run",
                Some(entrypoint),
            ));
        }
    }

    let mix = cwd.join("mix.exs");
    if mix.is_file() {
        let kind = if file_contains(&mix, "phoenix") {
            ProjectRunnerKind::Phoenix
        } else {
            ProjectRunnerKind::ElixirMix
        };
        candidates.push(candidate(kind, "run", None));
    }
    if cwd.join("stack.yaml").is_file() {
        candidates.push(candidate(ProjectRunnerKind::HaskellStack, "run", None));
    } else if first_root_file_with_extension(cwd, "cabal").is_some() {
        candidates.push(candidate(ProjectRunnerKind::HaskellCabal, "run", None));
    }
    if cwd.join("build.zig").is_file() {
        candidates.push(candidate(ProjectRunnerKind::ZigBuild, "run", None));
    }
    for (entrypoint, kind) in [
        ("app.R", ProjectRunnerKind::Shiny),
        ("main.R", ProjectRunnerKind::RScript),
        ("main.lua", ProjectRunnerKind::LuaScript),
        ("app.lua", ProjectRunnerKind::LuaScript),
        ("main.pl", ProjectRunnerKind::PerlScript),
        ("app.pl", ProjectRunnerKind::PerlScript),
    ] {
        if cwd.join(entrypoint).is_file() {
            candidates.push(candidate(kind, "run", Some(entrypoint)));
        }
    }
    if cwd.join("pubspec.yaml").is_file()
        && !file_contains(&cwd.join("pubspec.yaml"), "flutter")
        && cwd.join("bin").is_dir()
    {
        candidates.push(candidate(ProjectRunnerKind::DartRun, "run", None));
    }
    let jars = root_files_with_extension(cwd, "jar", 2);
    if jars.len() == 1 {
        candidates.push(candidate(ProjectRunnerKind::JavaJar, "run", Some(&jars[0])));
    }
}

fn first_root_file_with_extension(cwd: &Path, extension: &str) -> Option<String> {
    root_files_with_extension(cwd, extension, 1)
        .into_iter()
        .next()
}

fn root_files_with_extension(cwd: &Path, extension: &str, limit: usize) -> Vec<String> {
    let Ok(entries) = fs::read_dir(cwd) else {
        return Vec::new();
    };
    entries
        .flatten()
        .take(256)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file()
                || !path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case(extension))
            {
                return None;
            }
            path.file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
        })
        .take(limit)
        .collect()
}

fn candidate(
    kind: ProjectRunnerKind,
    name: impl Into<String>,
    entrypoint: Option<&str>,
) -> ProjectRunCandidate {
    ProjectRunCandidate {
        kind,
        name: name.into(),
        entrypoint: entrypoint.map(str::to_string),
    }
}

fn is_web_candidate(candidate: &ProjectRunCandidate) -> bool {
    matches!(
        candidate.kind,
        ProjectRunnerKind::Django
            | ProjectRunnerKind::FastApi
            | ProjectRunnerKind::Streamlit
            | ProjectRunnerKind::MavenSpring
            | ProjectRunnerKind::GradleBoot
            | ProjectRunnerKind::PhpArtisan
            | ProjectRunnerKind::PhpBuiltin
            | ProjectRunnerKind::Rails
            | ProjectRunnerKind::DockerCompose
    )
}

fn read_bounded(path: &Path) -> String {
    fs::read(path)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes[..bytes.len().min(128 * 1024)]).into_owned())
        .unwrap_or_default()
}

fn file_contains(path: &Path, needle: &str) -> bool {
    read_bounded(path)
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

fn has_task(path: &Path, task: &str) -> bool {
    read_bounded(path).lines().any(|line| {
        line.trim_start()
            .strip_prefix(task)
            .is_some_and(|suffix| suffix.trim_start().starts_with(':'))
    })
}
