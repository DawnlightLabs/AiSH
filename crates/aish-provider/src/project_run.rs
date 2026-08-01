use aish_ai::{SemanticPlan, SemanticPlanKind};
use aish_context::{ProjectRunCandidate, ProjectRunnerKind};
use std::path::{Component, Path, PathBuf};

pub(crate) fn compile_project_run(
    input: &str,
    context: &serde_json::Value,
) -> Option<SemanticPlan> {
    if !requests_project_run(input) {
        return None;
    }

    let candidates = context
        .get("run_candidates")
        .cloned()
        .and_then(|value| serde_json::from_value::<Vec<ProjectRunCandidate>>(value).ok())
        .unwrap_or_else(|| legacy_node_candidates(context));
    if candidates.is_empty() {
        return Some(clarification(
            "I could not find a supported runnable entrypoint in this folder. Tell me which command or entrypoint this project uses.",
        ));
    }

    let cwd = context
        .get("cwd")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let selected = match select_candidate(input, &candidates) {
        Selection::One(candidate) => candidate,
        Selection::Ambiguous(names) => {
            return Some(clarification(format!(
                "This project has multiple runnable targets: {}. Which one should I run?",
                names.join(", ")
            )))
        }
    };
    let command = match render_candidate(selected, context, &cwd) {
        Some(command) => command,
        None => {
            return Some(clarification(
                "I found a run target, but its task name or entrypoint was not safe to use. Specify the exact project task to run.",
            ))
        }
    };

    Some(SemanticPlan {
        kind: SemanticPlanKind::ShellCommand,
        payload: Some(command),
        target: None,
        scope: None,
        message: None,
        operation: None,
        destination: None,
    })
}

enum Selection<'a> {
    One(&'a ProjectRunCandidate),
    Ambiguous(Vec<String>),
}

fn select_candidate<'a>(input: &str, candidates: &'a [ProjectRunCandidate]) -> Selection<'a> {
    let words = words(input);
    let explicitly_named = candidates
        .iter()
        .filter(|candidate| {
            candidate.name != "run"
                && candidate.name != "serve"
                && words
                    .iter()
                    .any(|word| word.eq_ignore_ascii_case(&candidate.name))
        })
        .collect::<Vec<_>>();
    if explicitly_named.len() == 1 {
        return Selection::One(explicitly_named[0]);
    }

    let wants_stack = contains_any(&words, &["stack", "services", "containers", "compose"]);
    if wants_stack {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.kind == ProjectRunnerKind::DockerCompose)
        {
            return Selection::One(candidate);
        }
    }

    let wants_web = contains_any(
        &words,
        &[
            "website", "site", "server", "api", "web", "frontend", "backend",
        ],
    );
    let preferred = candidates
        .iter()
        .filter(|candidate| !wants_web || is_web_candidate(candidate))
        .collect::<Vec<_>>();
    let pool = if preferred.is_empty() {
        candidates.iter().collect::<Vec<_>>()
    } else {
        preferred
    };

    for task in ["dev", "start", "serve", "preview"] {
        if let Some(candidate) = pool
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case(task))
        {
            return Selection::One(candidate);
        }
    }
    if pool.len() == 1 {
        return Selection::One(pool[0]);
    }

    let primary = pool
        .iter()
        .copied()
        .filter(|candidate| candidate.kind != ProjectRunnerKind::DockerCompose)
        .collect::<Vec<_>>();
    if primary.len() == 1 {
        return Selection::One(primary[0]);
    }
    Selection::Ambiguous(
        pool.iter()
            .map(|candidate| candidate_label(candidate))
            .collect(),
    )
}

fn render_candidate(
    candidate: &ProjectRunCandidate,
    context: &serde_json::Value,
    cwd: &Path,
) -> Option<String> {
    if !safe_name(&candidate.name) {
        return None;
    }
    let entrypoint = match candidate.entrypoint.as_deref() {
        Some(value) => Some(validated_entrypoint(value, cwd)?),
        None => None,
    };
    let quoted_entrypoint = entrypoint.as_deref().map(shell_quote);
    let python = if cfg!(windows) { "python" } else { "python3" };

    let command = match candidate.kind {
        ProjectRunnerKind::NodeScript => render_node(candidate, context),
        ProjectRunnerKind::NodeEntrypoint => format!("node {}", quoted_entrypoint?),
        ProjectRunnerKind::CargoRun => "cargo run".to_string(),
        ProjectRunnerKind::PythonScript => {
            format!("{python} {}", quoted_entrypoint?)
        }
        ProjectRunnerKind::Django => {
            format!("{python} {} runserver", quoted_entrypoint?)
        }
        ProjectRunnerKind::FastApi => {
            let module = Path::new(entrypoint.as_deref()?)
                .file_stem()
                .and_then(|value| value.to_str())?;
            if !safe_name(module) {
                return None;
            }
            format!("{python} -m uvicorn {module}:app --reload")
        }
        ProjectRunnerKind::Streamlit => {
            format!("{python} -m streamlit run {}", quoted_entrypoint?)
        }
        ProjectRunnerKind::GoRun => "go run .".to_string(),
        ProjectRunnerKind::DotnetRun => {
            format!("dotnet run --project {}", quoted_entrypoint?)
        }
        ProjectRunnerKind::MavenSpring => {
            if cfg!(windows) && cwd.join("mvnw.cmd").is_file() {
                ".\\mvnw.cmd spring-boot:run".to_string()
            } else if !cfg!(windows) && cwd.join("mvnw").is_file() {
                "./mvnw spring-boot:run".to_string()
            } else {
                "mvn spring-boot:run".to_string()
            }
        }
        ProjectRunnerKind::GradleBoot | ProjectRunnerKind::GradleRun => {
            let task = if candidate.kind == ProjectRunnerKind::GradleBoot {
                "bootRun"
            } else {
                "run"
            };
            if cfg!(windows) && cwd.join("gradlew.bat").is_file() {
                format!(".\\gradlew.bat {task}")
            } else if !cfg!(windows) && cwd.join("gradlew").is_file() {
                format!("./gradlew {task}")
            } else {
                format!("gradle {task}")
            }
        }
        ProjectRunnerKind::PhpArtisan => "php artisan serve".to_string(),
        ProjectRunnerKind::PhpBuiltin => "php -S localhost:8000 -t .".to_string(),
        ProjectRunnerKind::Rails => "bundle exec rails server".to_string(),
        ProjectRunnerKind::RubyScript => format!("ruby {}", quoted_entrypoint?),
        ProjectRunnerKind::FlutterRun => "flutter run".to_string(),
        ProjectRunnerKind::DockerCompose => "docker compose up".to_string(),
        ProjectRunnerKind::SwiftRun => "swift run".to_string(),
        ProjectRunnerKind::DenoTask => format!("deno task {}", candidate.name),
        ProjectRunnerKind::DenoRun => format!("deno run {}", quoted_entrypoint?),
        ProjectRunnerKind::MakeRun => "make run".to_string(),
        ProjectRunnerKind::JustRun => "just run".to_string(),
        ProjectRunnerKind::StaticSite => format!("{python} -m http.server 8000"),
        ProjectRunnerKind::ShellScript => render_script(entrypoint.as_deref()?)?,
        ProjectRunnerKind::ElixirMix => "mix run --no-halt".to_string(),
        ProjectRunnerKind::Phoenix => "mix phx.server".to_string(),
        ProjectRunnerKind::HaskellStack => "stack run".to_string(),
        ProjectRunnerKind::HaskellCabal => "cabal run".to_string(),
        ProjectRunnerKind::ZigBuild => "zig build run".to_string(),
        ProjectRunnerKind::RScript => format!("Rscript {}", quoted_entrypoint?),
        ProjectRunnerKind::Shiny => "R -e \"shiny::runApp('.')\"".to_string(),
        ProjectRunnerKind::LuaScript => format!("lua {}", quoted_entrypoint?),
        ProjectRunnerKind::PerlScript => format!("perl {}", quoted_entrypoint?),
        ProjectRunnerKind::DartRun => "dart run".to_string(),
        ProjectRunnerKind::JavaJar => format!("java -jar {}", quoted_entrypoint?),
    };
    Some(command)
}

fn render_node(candidate: &ProjectRunCandidate, context: &serde_json::Value) -> String {
    let manager = context
        .get("package_manager")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("npm");
    let run = match manager {
        "yarn" => format!("yarn {}", candidate.name),
        "pnpm" => format!("pnpm run {}", candidate.name),
        "bun" => format!("bun run {}", candidate.name),
        _ => format!("npm run {}", candidate.name),
    };
    if context
        .get("dependencies_installed")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
    {
        let install = match manager {
            "yarn" => "yarn install",
            "pnpm" => "pnpm install",
            "bun" => "bun install",
            _ => "npm install",
        };
        format!("{install}; {run}")
    } else {
        run
    }
}

fn render_script(entrypoint: &str) -> Option<String> {
    let quoted = shell_quote(entrypoint);
    let extension = Path::new(entrypoint)
        .extension()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase();
    match extension.as_str() {
        "ps1" => Some(format!(
            "powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File {quoted}"
        )),
        "cmd" | "bat" if cfg!(windows) => Some(format!("& {quoted}")),
        "sh" if cfg!(windows) => Some(format!("bash {quoted}")),
        "sh" => Some(format!("sh {quoted}")),
        _ => None,
    }
}

fn validated_entrypoint(value: &str, cwd: &Path) -> Option<String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
        || !cwd.join(path).is_file()
    {
        return None;
    }
    Some(value.to_string())
}

fn legacy_node_candidates(context: &serde_json::Value) -> Vec<ProjectRunCandidate> {
    if context
        .get("project_type")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|value| !value.eq_ignore_ascii_case("node"))
    {
        return Vec::new();
    }
    context
        .get("available_tasks")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(|task| ProjectRunCandidate {
            kind: ProjectRunnerKind::NodeScript,
            name: task.to_string(),
            entrypoint: None,
        })
        .collect()
}

fn requests_project_run(input: &str) -> bool {
    let words = words(input);
    if contains_any(&words, &["script", "file"]) {
        return false;
    }
    let action = contains_any(
        &words,
        &["run", "start", "launch", "serve", "execute", "boot"],
    );
    let project = contains_any(
        &words,
        &[
            "website",
            "site",
            "app",
            "application",
            "project",
            "server",
            "api",
            "frontend",
            "backend",
            "stack",
            "service",
            "services",
            "program",
            "code",
        ],
    );
    let situated = contains_any(&words, &["this", "it", "current", "here"]);
    action && (project || situated)
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, ':' | '_' | '-')
        })
}

fn shell_quote(value: &str) -> String {
    if cfg!(windows) {
        format!("'{}'", value.replace('\'', "''"))
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn is_web_candidate(candidate: &ProjectRunCandidate) -> bool {
    matches!(
        candidate.kind,
        ProjectRunnerKind::NodeScript
            | ProjectRunnerKind::Django
            | ProjectRunnerKind::FastApi
            | ProjectRunnerKind::Streamlit
            | ProjectRunnerKind::MavenSpring
            | ProjectRunnerKind::GradleBoot
            | ProjectRunnerKind::PhpArtisan
            | ProjectRunnerKind::PhpBuiltin
            | ProjectRunnerKind::Rails
            | ProjectRunnerKind::DockerCompose
            | ProjectRunnerKind::StaticSite
            | ProjectRunnerKind::Phoenix
            | ProjectRunnerKind::Shiny
    )
}

fn candidate_label(candidate: &ProjectRunCandidate) -> String {
    candidate
        .entrypoint
        .as_ref()
        .map(|entrypoint| format!("{} ({entrypoint})", candidate.name))
        .unwrap_or_else(|| candidate.name.clone())
}

fn clarification(message: impl Into<String>) -> SemanticPlan {
    SemanticPlan {
        kind: SemanticPlanKind::Clarification,
        payload: None,
        target: None,
        scope: None,
        message: Some(message.into()),
        operation: None,
        destination: None,
    }
}

fn words(input: &str) -> Vec<String> {
    input
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

fn contains_any(words: &[String], candidates: &[&str]) -> bool {
    words
        .iter()
        .any(|word| candidates.iter().any(|candidate| word == candidate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("aish-provider-{name}-{unique}"));
        std::fs::create_dir_all(&root).expect("fixture root");
        root
    }

    fn context(root: &Path) -> serde_json::Value {
        serde_json::to_value(aish_context::inspect_project(root)).expect("project context")
    }

    #[test]
    fn compiles_common_language_projects_without_model_output() {
        let cases = [
            ("rust", "Cargo.toml", "src/main.rs", "cargo run"),
            ("go", "go.mod", "main.go", "go run ."),
            (
                "dotnet",
                "Fixture.csproj",
                "Program.cs",
                "dotnet run --project",
            ),
            ("ruby", "Gemfile", "app.rb", "ruby 'app.rb'"),
            ("php", "composer.json", "index.php", "php -S localhost:8000"),
            ("swift", "Package.swift", "Sources/main.swift", "swift run"),
        ];
        for (name, manifest, entrypoint, expected) in cases {
            let root = fixture_root(name);
            std::fs::write(root.join(manifest), "fixture\n").expect("manifest");
            let entry = root.join(entrypoint);
            std::fs::create_dir_all(entry.parent().expect("entry parent")).expect("entry parent");
            std::fs::write(entry, "fixture\n").expect("entrypoint");
            let plan = compile_project_run("run this app", &context(&root)).expect("run plan");
            assert!(
                plan.payload
                    .as_deref()
                    .is_some_and(|value| value.contains(expected)),
                "{name}: {:?}",
                plan.payload
            );
            let _ = std::fs::remove_dir_all(root);
        }
    }

    #[test]
    fn compiles_framework_and_static_web_projects() {
        let fastapi = fixture_root("fastapi");
        std::fs::write(
            fastapi.join("app.py"),
            "from fastapi import FastAPI\napp = FastAPI()\n",
        )
        .expect("fastapi app");
        let plan = compile_project_run("run this website please", &context(&fastapi))
            .expect("fastapi plan");
        assert_eq!(
            plan.payload.as_deref(),
            Some(if cfg!(windows) {
                "python -m uvicorn app:app --reload"
            } else {
                "python3 -m uvicorn app:app --reload"
            })
        );

        let site = fixture_root("static");
        std::fs::write(site.join("index.html"), "<!doctype html>\n").expect("index");
        let plan = compile_project_run("serve this site", &context(&site)).expect("site plan");
        assert!(plan
            .payload
            .as_deref()
            .is_some_and(|value| value.contains("-m http.server 8000")));
        let _ = std::fs::remove_dir_all(fastapi);
        let _ = std::fs::remove_dir_all(site);
    }

    #[test]
    fn compiles_additional_standard_runtime_projects() {
        let cases = [
            ("node-entry", "main.js", "fixture", "node 'main.js'"),
            (
                "elixir",
                "mix.exs",
                "defmodule Fixture do\nend",
                "mix run --no-halt",
            ),
            ("haskell", "stack.yaml", "resolver: lts-22.0", "stack run"),
            (
                "zig",
                "build.zig",
                "pub fn build() void {}",
                "zig build run",
            ),
            ("r", "main.R", "print('ok')", "Rscript 'main.R'"),
            ("lua", "main.lua", "print('ok')", "lua 'main.lua'"),
            ("perl", "main.pl", "print qq(ok);", "perl 'main.pl'"),
            (
                "java-jar",
                "fixture.jar",
                "fixture",
                "java -jar 'fixture.jar'",
            ),
        ];
        for (name, file, content, expected) in cases {
            let root = fixture_root(name);
            std::fs::write(root.join(file), content).expect("runtime fixture");
            let plan =
                compile_project_run("run this application", &context(&root)).expect("runtime plan");
            assert!(
                plan.payload
                    .as_deref()
                    .is_some_and(|value| value.contains(expected)),
                "{name}: {:?}",
                plan.payload
            );
            let _ = std::fs::remove_dir_all(root);
        }

        let script = fixture_root("script");
        let script_name = if cfg!(windows) { "run.ps1" } else { "run.sh" };
        std::fs::write(script.join(script_name), "fixture\n").expect("script fixture");
        let plan =
            compile_project_run("execute this program", &context(&script)).expect("script plan");
        assert!(plan
            .payload
            .as_deref()
            .is_some_and(|value| value.contains(script_name)));
        let _ = std::fs::remove_dir_all(script);
    }

    #[test]
    fn rejects_injected_tasks_and_entrypoints() {
        let root = fixture_root("unsafe");
        let unsafe_task = serde_json::json!({
            "cwd": root,
            "project_type": "node",
            "package_manager": "npm",
            "run_candidates": [{"kind":"node_script","name":"dev; Remove-Item *","entrypoint":null}]
        });
        let plan = compile_project_run("run this app", &unsafe_task).expect("clarification");
        assert_eq!(plan.kind, SemanticPlanKind::Clarification);

        let unsafe_entrypoint = serde_json::json!({
            "cwd": root,
            "run_candidates": [{"kind":"python_script","name":"run","entrypoint":"../outside.py"}]
        });
        let plan = compile_project_run("run this app", &unsafe_entrypoint).expect("clarification");
        assert_eq!(plan.kind, SemanticPlanKind::Clarification);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn leaves_explicit_script_requests_to_the_script_execution_pipeline() {
        let context = serde_json::json!({
            "cwd": ".",
            "project_type": "node",
            "package_manager": "npm",
            "available_tasks": ["dev"]
        });
        assert!(compile_project_run(
            "run the PowerShell script marker-task.ps1 in this folder",
            &context
        )
        .is_none());
    }

    #[test]
    fn asks_when_multiple_independent_targets_are_ambiguous() {
        let root = fixture_root("ambiguous");
        std::fs::write(root.join("One.csproj"), "<Project />").expect("first project");
        std::fs::write(root.join("Two.csproj"), "<Project />").expect("second project");
        let plan = compile_project_run("run this app", &context(&root)).expect("clarification");
        assert_eq!(plan.kind, SemanticPlanKind::Clarification);
        assert!(plan.message.as_deref().is_some_and(|message| {
            message.contains("One.csproj") && message.contains("Two.csproj")
        }));
        let _ = std::fs::remove_dir_all(root);
    }
}
