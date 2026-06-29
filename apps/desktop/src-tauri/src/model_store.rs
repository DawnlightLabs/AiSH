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
    PathBuf::from(user_home()).join("Downloads").join("aish-model").join("models")
}

fn llama_cli_path() -> String {
    format!("{}/Downloads/llama.cpp/build/bin/Release/llama-cli.exe", user_home())
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("model_profiles.json"));
        paths.push(cwd.join("apps").join("desktop").join("model_profiles.json"));
    }

    let manifest = manifest_dir();
    paths.push(manifest.join("..").join("model_profiles.json"));
    paths.push(manifest.join("..").join("..").join("..").join("model_profiles.json"));

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

pub fn list_profiles() -> Result<Vec<ModelProfile>, String> {
    for path in candidate_paths() {
        if path.exists() {
            match read_profiles(&path) {
                Ok(profiles) if !profiles.is_empty() => return Ok(profiles),
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
    let path = store_path();
    let text = serde_json::to_string_pretty(&profiles).map_err(|error| error.to_string())?;
    fs::write(&path, text).map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
    Ok(profiles)
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
        .filter(|path| path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("gguf")))
        .map(profile_from_path)
        .collect();

    profiles.sort_by(|a, b| a.label.cmp(&b.label));
    profiles
}

fn profile_from_path(path: PathBuf) -> ModelProfile {
    let file_name = path.file_name().and_then(|name| name.to_str()).unwrap_or("model.gguf");
    let stem = path.file_stem().and_then(|name| name.to_str()).unwrap_or(file_name);
    let lower = stem.to_lowercase();

    ModelProfile {
        id: sanitize_id(stem),
        label: label_from_stem(stem),
        family: family_from_name(&lower),
        model_path: path.display().to_string().replace('\\', "/"),
        llama_cli_path: llama_cli_path(),
        context_tokens: context_tokens_for(&lower),
        max_tokens: 512,
        temperature: 0.1,
    }
}

fn expected_profiles() -> Vec<ModelProfile> {
    let root = models_dir().display().to_string().replace('\\', "/");
    let llama = llama_cli_path();

    vec![
        profile("qwen25-coder-05b-q4", "Qwen2.5 Coder 0.5B Instruct Q4_K_M", "qwen2.5-coder", &format!("{root}/qwen2.5-coder-0.5b-instruct-q4_k_m.gguf"), &llama, 32768),
        profile("qwen25-coder-15b-q4", "Qwen2.5 Coder 1.5B Instruct Q4_K_M", "qwen2.5-coder", &format!("{root}/qwen2.5-coder-1.5b-instruct-q4_k_m.gguf"), &llama, 32768),
        profile("qwen25-coder-3b-q4", "Qwen2.5 Coder 3B Instruct Q4_K_M", "qwen2.5-coder", &format!("{root}/qwen2.5-coder-3b-instruct-q4_k_m.gguf"), &llama, 32768),
        profile("qwen3-06b-q4", "Qwen3 0.6B Q4_K_M", "qwen3", &format!("{root}/qwen3-0.6b-q4_k_m.gguf"), &llama, 32768),
        profile("qwen3-17b-q4", "Qwen3 1.7B Q4_K_M", "qwen3", &format!("{root}/qwen3-1.7b-q4_k_m.gguf"), &llama, 32768),
        profile("qwen35-08b-q4", "Qwen3.5 0.8B Q4_K_M", "qwen3.5", &format!("{root}/qwen3.5-0.8b-q4_k_m.gguf"), &llama, 32768),
        profile("qwen35-2b-q4", "Qwen3.5 2B Q4_K_M", "qwen3.5", &format!("{root}/qwen3.5-2b-q4_k_m.gguf"), &llama, 32768),
        profile("deepseek-coder-13b-q4", "DeepSeek Coder 1.3B Instruct Q4_K_M", "deepseek-coder", &format!("{root}/deepseek-coder-1.3b-instruct-q4_k_m.gguf"), &llama, 16384),
        profile("codegemma-2b-q4", "CodeGemma 2B Q4_K_M", "codegemma", &format!("{root}/codegemma-2b-q4_k_m.gguf"), &llama, 8192),
    ]
}

fn profile(id: &str, label: &str, family: &str, model_path: &str, llama_cli_path: &str, context_tokens: usize) -> ModelProfile {
    ModelProfile {
        id: id.to_string(),
        label: label.to_string(),
        family: family.to_string(),
        model_path: model_path.to_string(),
        llama_cli_path: llama_cli_path.to_string(),
        context_tokens,
        max_tokens: 512,
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
    stem.replace(['_', '-'], " ")
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn family_from_name(name: &str) -> String {
    if name.contains("qwen2.5-coder") || name.contains("qwen25-coder") || name.contains("qwen-2.5-coder") {
        "qwen2.5-coder".to_string()
    } else if name.contains("qwen3.5") || name.contains("qwen35") {
        "qwen3.5".to_string()
    } else if name.contains("qwen3") {
        "qwen3".to_string()
    } else if name.contains("deepseek") {
        "deepseek-coder".to_string()
    } else if name.contains("codegemma") {
        "codegemma".to_string()
    } else if name.contains("stable-code") || name.contains("stablecode") {
        "stable-code".to_string()
    } else {
        "generic".to_string()
    }
}

fn context_tokens_for(name: &str) -> usize {
    if name.contains("codegemma") {
        8192
    } else if name.contains("deepseek") {
        16384
    } else {
        32768
    }
}
