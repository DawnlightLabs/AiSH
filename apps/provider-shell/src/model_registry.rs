use aish_ai::ModelProfile;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModelRegistry {
    models: Vec<ModelProfile>,
    active_id: String,
    selection_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct SavedSelection {
    model_id: String,
}

impl ModelRegistry {
    pub fn discover(runtime_path: &str) -> Self {
        let mut directories = Vec::new();
        if let Ok(path) = env::var("AISH_MODELS_DIR") {
            directories.push(PathBuf::from(path));
        }
        if let Ok(path) = env::var("AISH_MODEL_PATH") {
            if let Some(parent) = Path::new(&path).parent() {
                directories.push(parent.to_path_buf());
            }
        }
        directories.push(install_root().join("models"));
        if let Ok(cwd) = env::current_dir() {
            directories.push(cwd.join("models"));
        }

        let mut seen = HashSet::new();
        let mut models = Vec::new();
        for directory in directories {
            let Ok(entries) = fs::read_dir(directory) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_none_or(|value| !value.eq_ignore_ascii_case("gguf"))
                    || !is_gguf(&path)
                {
                    continue;
                }
                let canonical = fs::canonicalize(&path).unwrap_or(path);
                if !seen.insert(normalize_path(&canonical)) {
                    continue;
                }
                models.push(profile_for(&canonical, runtime_path));
            }
        }
        deduplicate_profiles_by_id(&mut models);
        models.sort_by(|left, right| left.label.to_lowercase().cmp(&right.label.to_lowercase()));

        let selection_path = install_root().join("state").join("model-selection.json");
        let saved = read_selection(&selection_path);
        let configured = env::var("AISH_MODEL_PATH").ok().map(PathBuf::from);
        let active_id = select_active_id(&models, saved.as_deref(), configured.as_deref());

        Self {
            models,
            active_id,
            selection_path,
        }
    }

    pub fn models(&self) -> &[ModelProfile] {
        &self.models
    }

    pub fn active(&self) -> Option<&ModelProfile> {
        self.models.iter().find(|model| model.id == self.active_id)
    }

    pub fn use_model(&mut self, selector: &str) -> Result<&ModelProfile, String> {
        let id = self.resolve_model_id(selector)?;
        self.active_id = id;
        persist_selection(&self.selection_path, &self.active_id)?;
        Ok(self.active().expect("selected model exists"))
    }

    pub fn model(&self, selector: &str) -> Result<&ModelProfile, String> {
        let id = self.resolve_model_id(selector)?;
        Ok(self
            .models
            .iter()
            .find(|model| model.id == id)
            .expect("resolved model exists"))
    }

    fn resolve_model_id(&self, selector: &str) -> Result<String, String> {
        let selector = selector.trim();
        let matches = self
            .models
            .iter()
            .filter(|model| {
                model.id.eq_ignore_ascii_case(selector)
                    || Path::new(&model.model_path)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .is_some_and(|name| name.eq_ignore_ascii_case(selector))
            })
            .map(|model| model.id.clone())
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [] => Err(format!("No discovered model matches '{selector}'.")),
            [id] => Ok(id.clone()),
            _ => Err(format!(
                "'{selector}' matches more than one model; use the full model id."
            )),
        }
    }
}

fn deduplicate_profiles_by_id(models: &mut Vec<ModelProfile>) {
    let mut seen = HashSet::new();
    models.retain(|model| seen.insert(model.id.to_ascii_lowercase()));
}

fn select_active_id(
    models: &[ModelProfile],
    saved: Option<&str>,
    configured: Option<&Path>,
) -> String {
    configured
        .and_then(|path| {
            let normalized = normalize_path(path);
            models
                .iter()
                .find(|model| normalize_path(Path::new(&model.model_path)) == normalized)
                .map(|model| model.id.clone())
        })
        .or_else(|| {
            saved
                .filter(|id| models.iter().any(|model| model.id == *id))
                .map(str::to_string)
        })
        .or_else(|| choose_default(models).map(|model| model.id.clone()))
        .unwrap_or_default()
}

fn profile_for(path: &Path, runtime_path: &str) -> ModelProfile {
    let filename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("local.gguf");
    let lower = filename.to_ascii_lowercase();
    let (family, context_tokens, max_tokens, strategy, stop_sequences) =
        if lower.contains("qwen3.5") {
            (
                "qwen3.5",
                8192,
                256,
                "grammar",
                vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
            )
        } else if lower.contains("qwen2.5") && lower.contains("coder") {
            (
                "qwen2.5-coder",
                4096,
                192,
                "grammar",
                vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
            )
        } else if lower.contains("qwen3") {
            (
                "qwen3",
                8192,
                256,
                "grammar",
                vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
            )
        } else if lower.contains("minicpm5") {
            (
                "minicpm5",
                8192,
                256,
                "grammar",
                vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
            )
        } else if lower.contains("internlm2_5") || lower.contains("internlm2-5") {
            (
                "internlm2.5",
                8192,
                256,
                "grammar",
                vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
            )
        } else if lower.contains("lfm2.5") || lower.contains("lfm2-5") {
            (
                "lfm2.5",
                8192,
                320,
                "grammar",
                vec!["<|im_start|>".to_string(), "<|im_end|>".to_string()],
            )
        } else {
            ("generic-gguf", 4096, 192, "auto", Vec::new())
        };
    ModelProfile {
        id: stable_id(filename),
        label: filename.trim_end_matches(".gguf").to_string(),
        family: family.to_string(),
        model_path: path.display().to_string(),
        llama_cli_path: runtime_path.to_string(),
        context_tokens,
        max_tokens,
        temperature: 0.0,
        structured_output_strategy: strategy.to_string(),
        chat_template: None,
        use_system_prompt: false,
        retry_count: 1,
        stop_sequences,
        timeout_seconds: 60,
        backend: aish_ai::ModelBackend::LocalGguf,
        endpoint: None,
        api_key_env: None,
    }
}

fn choose_default(models: &[ModelProfile]) -> Option<&ModelProfile> {
    models.iter().min_by_key(|model| {
        let lower = model.label.to_ascii_lowercase().replace('-', "_");
        let family_rank = if lower.contains("qwen2.5") { 0 } else { 1 };
        let quant_rank = if lower.contains("q6_k") {
            0
        } else if lower.contains("q5_k_m") {
            1
        } else if lower.contains("q8_0") {
            2
        } else {
            3
        };
        (family_rank, quant_rank)
    })
}

fn stable_id(filename: &str) -> String {
    filename
        .trim_end_matches(".gguf")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn is_gguf(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut magic = [0_u8; 4];
    file.read_exact(&mut magic).is_ok() && &magic == b"GGUF"
}

fn read_selection(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<SavedSelection>(&text)
        .ok()
        .map(|saved| saved.model_id)
}

fn persist_selection(path: &Path, model_id: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(&SavedSelection {
        model_id: model_id.to_string(),
    })
    .map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| error.to_string())
}

fn install_root() -> PathBuf {
    if cfg!(windows) {
        env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir())
            .join("AiSH")
    } else if cfg!(target_os = "macos") {
        home_dir().join("Applications").join("AiSH")
    } else {
        home_dir().join(".local").join("aish")
    }
}

fn home_dir() -> PathBuf {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn normalize_path(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::{
        deduplicate_profiles_by_id, persist_selection, profile_for, read_selection,
        select_active_id, stable_id, ModelProfile,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn derives_stable_ids_without_model_specific_tables() {
        assert_eq!(stable_id("Future Model Q7.gguf"), "future-model-q7");
    }

    #[test]
    fn creates_declarative_profiles_for_distinct_model_families() {
        let qwen25 = profile_for(
            PathBuf::from("qwen2.5-coder-1.5b-instruct-q5_k_m.gguf").as_path(),
            "llama-cli",
        );
        let qwen35 = profile_for(
            PathBuf::from("qwen3.5-2b-instruct-q8_0.gguf").as_path(),
            "llama-cli",
        );
        let qwen3 = profile_for(PathBuf::from("Qwen3-1.7B-Q8_0.gguf").as_path(), "llama-cli");
        let minicpm5 = profile_for(
            PathBuf::from("MiniCPM5-1B-Q8_0.gguf").as_path(),
            "llama-cli",
        );
        let internlm = profile_for(
            PathBuf::from("internlm2_5-1_8b-chat-q6_k.gguf").as_path(),
            "llama-cli",
        );
        let lfm = profile_for(
            PathBuf::from("LFM2.5-1.2B-Instruct-Q6_K.gguf").as_path(),
            "llama-cli",
        );
        let future = profile_for(
            PathBuf::from("future-general-model-q7.gguf").as_path(),
            "llama-cli",
        );

        assert_eq!(qwen25.family, "qwen2.5-coder");
        assert_eq!(qwen35.family, "qwen3.5");
        assert_eq!(qwen3.family, "qwen3");
        assert_eq!(minicpm5.family, "minicpm5");
        assert_eq!(internlm.family, "internlm2.5");
        assert_eq!(lfm.family, "lfm2.5");
        assert_eq!(lfm.max_tokens, 320);
        assert!(!qwen25.use_system_prompt);
        assert_eq!(future.family, "generic-gguf");
        assert_ne!(qwen25.max_tokens, qwen35.max_tokens);
        assert_eq!(qwen25.structured_output_strategy, "grammar");
        for profile in [&qwen3, &minicpm5, &internlm, &lfm] {
            assert_eq!(profile.structured_output_strategy, "grammar");
            assert_eq!(profile.context_tokens, 8192);
        }
        assert!(qwen25.stop_sequences.contains(&"<|im_start|>".to_string()));
        assert_eq!(future.structured_output_strategy, "auto");
    }

    #[test]
    fn duplicate_model_ids_from_multiple_roots_keep_the_first_profile() {
        let first = fixture_profile("planner-q6-k", "C:/selected/planner-q6.gguf");
        let duplicate = fixture_profile("planner-q6-k", "C:/installed/planner-q6.gguf");
        let mut models = vec![first.clone(), duplicate];

        deduplicate_profiles_by_id(&mut models);

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].model_path, first.model_path);
    }

    #[test]
    fn stale_selection_falls_back_and_configured_model_wins() {
        let q5 = fixture_profile("planner-q5-k-m", "C:/models/planner-q5.gguf");
        let q6 = fixture_profile("planner-q6-k", "C:/models/planner-q6.gguf");
        let models = vec![q5.clone(), q6.clone()];

        assert_eq!(
            select_active_id(&models, Some("missing-model"), None),
            q6.id
        );
        assert_eq!(
            select_active_id(
                &models,
                Some(&q6.id),
                Some(PathBuf::from(&q5.model_path).as_path())
            ),
            q5.id
        );
    }

    #[test]
    fn persists_user_selection_as_small_json_state() {
        let root = std::env::temp_dir().join(format!(
            "aish-model-selection-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = root.join("state").join("model-selection.json");
        persist_selection(&path, "future-model-q7").unwrap();
        assert_eq!(read_selection(&path).as_deref(), Some("future-model-q7"));
        fs::remove_dir_all(root).unwrap();
    }

    fn fixture_profile(id: &str, path: &str) -> ModelProfile {
        ModelProfile {
            id: id.to_string(),
            label: id.to_string(),
            family: "test".to_string(),
            model_path: path.to_string(),
            llama_cli_path: "llama-cli".to_string(),
            context_tokens: 4096,
            max_tokens: 320,
            temperature: 0.0,
            structured_output_strategy: "auto".to_string(),
            chat_template: None,
            use_system_prompt: false,
            retry_count: 1,
            stop_sequences: Vec::new(),
            timeout_seconds: 60,
            backend: aish_ai::ModelBackend::LocalGguf,
            endpoint: None,
            api_key_env: None,
        }
    }
}
