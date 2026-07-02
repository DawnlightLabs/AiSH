use aish_ai::ModelProfile;
use std::fs;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn user_home() -> String {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| "C:/Users/Amaan".to_string())
        .replace('\\', "/")
}

fn models_dir() -> PathBuf {
    if let Ok(model_path) = std::env::var("AISH_MODEL_PATH") {
        let path = PathBuf::from(model_path);
        if let Some(parent) = path.parent() {
            return parent.to_path_buf();
        }
    }

    PathBuf::from(user_home())
        .join("Downloads")
        .join("aish-model")
        .join("models")
}

fn llama_cli_path() -> String {
    if cfg!(target_os = "windows") {
        format!(
            "{}/Downloads/llama.cpp/build/bin/Release/llama-cli.exe",
            user_home()
        )
    } else {
        std::env::var("AISH_LLAMA_CLI").unwrap_or_else(|_| "llama-cli".to_string())
    }
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("model_profiles.json"));
        paths.push(cwd.join("apps").join("desktop").join("model_profiles.json"));
    }

    let manifest = manifest_dir();
    paths.push(manifest.join("..").join("model_profiles.json"));
    paths.push(
        manifest
            .join("..")
            .join("..")
            .join("..")
            .join("model_profiles.json"),
    );

    paths
}

fn store_path() -> PathBuf {
    candidate_paths()
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| manifest_dir().join("..").join("model_profiles.json"))
}

fn read_profiles(path: &Path) -> Result<Vec<ModelProfile>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read {}: {error}", path.display()))?;

    if text.trim().is_empty() {
        return Err(format!("{} is empty", path.display()));
    }

    serde_json::from_str(&text)
        .map_err(|error| format!("Failed to parse {}: {error}", path.display()))
}

fn active_profiles(profiles: Vec<ModelProfile>) -> Vec<ModelProfile> {
    profiles
        .into_iter()
        .filter(|profile| {
            is_qwen25_coder(&profile.id)
                || is_qwen25_coder(&profile.label)
                || is_qwen25_coder(&profile.model_path)
        })
        .collect()
}

pub fn list_profiles() -> Result<Vec<ModelProfile>, String> {
    for path in candidate_paths() {
        if path.exists() {
            match read_profiles(&path) {
                Ok(profiles) => {
                    let active = active_profiles(profiles);
                    if !active.is_empty() {
                        return Ok(active);
                    }
                }
                _ => continue,
            }
        }
    }

    let discovered = discover_gguf_profiles();
    if !discovered.is_empty() {
        return Ok(discovered);
    }

    Ok(expected_profiles())
}

pub fn save_profiles(profiles: Vec<ModelProfile>) -> Result<Vec<ModelProfile>, String> {
    let active = active_profiles(profiles);
    let path = store_path();
    let text = serde_json::to_string_pretty(&active).map_err(|error| error.to_string())?;
    fs::write(&path, text)
        .map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
    Ok(active)
}

pub fn find_profile(id: &str) -> Result<ModelProfile, String> {
    let profiles = list_profiles()?;
    profiles
        .into_iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| format!("missing profile: {id}"))
}

fn discover_gguf_profiles() -> Vec<ModelProfile> {
    let dir = models_dir();
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };

    let mut profiles: Vec<ModelProfile> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
        })
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_qwen25_coder)
        })
        .map(profile_from_path)
        .collect();

    profiles.sort_by_key(|profile| model_priority(&profile.id));
    profiles
}

fn profile_from_path(path: PathBuf) -> ModelProfile {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("model.gguf");
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or(file_name);

    ModelProfile {
        id: sanitize_id(stem),
        label: label_from_stem(stem),
        family: "qwen2.5-coder".to_string(),
        model_path: path.display().to_string().replace('\\', "/"),
        llama_cli_path: llama_cli_path(),
        context_tokens: 4096,
        max_tokens: 192,
        temperature: 0.1,
    }
}

fn expected_profiles() -> Vec<ModelProfile> {
    let root = models_dir().display().to_string().replace('\\', "/");
    let llama = llama_cli_path();

    vec![profile(
        "qwen2-5-coder-1-5b-instruct-q4-k-m",
        "Qwen2.5 Coder 1.5B Instruct Q4_K_M",
        "qwen2.5-coder",
        &format!("{root}/Qwen2.5-Coder-1.5B-Instruct-Q4_K_M.gguf"),
        &llama,
    )]
}

fn profile(
    id: &str,
    label: &str,
    family: &str,
    model_path: &str,
    llama_cli_path: &str,
) -> ModelProfile {
    ModelProfile {
        id: id.to_string(),
        label: label.to_string(),
        family: family.to_string(),
        model_path: model_path.to_string(),
        llama_cli_path: llama_cli_path.to_string(),
        context_tokens: 4096,
        max_tokens: 192,
        temperature: 0.1,
    }
}

fn sanitize_id(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn label_from_stem(stem: &str) -> String {
    stem.replace('-', " ")
        .replace("Q4 K M", "Q4_K_M")
        .replace("Q5 K M", "Q5_K_M")
        .replace("Q6 K", "Q6_K")
        .replace("Q8 0", "Q8_0")
}

fn is_qwen25_coder(value: &str) -> bool {
    let lower = value.to_lowercase();
    (lower.contains("qwen2.5-coder")
        || lower.contains("qwen2-5-coder")
        || lower.contains("qwen25-coder"))
        && (lower.contains("1.5b") || lower.contains("1-5b") || lower.contains("15b"))
}

fn model_priority(id: &str) -> usize {
    if is_qwen25_coder(id) {
        0
    } else {
        100
    }
}
