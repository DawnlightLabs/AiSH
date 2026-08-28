use aish_ai::{ModelBackend, ModelProfile};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviders {
    pub active: Option<CloudProviderConfig>,
    #[serde(skip)]
    path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderConfig {
    pub name: String,
    pub backend: ModelBackend,
    pub endpoint: String,
    pub model: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderPreset {
    pub name: &'static str,
    pub backend: ModelBackend,
    pub endpoint: &'static str,
    pub model: &'static str,
    pub api_key_env: Option<&'static str>,
}

const PRESETS: &[ProviderPreset] = &[
    ProviderPreset {
        name: "openai",
        backend: ModelBackend::OpenAiCompatible,
        endpoint: "https://api.openai.com/v1",
        model: "gpt-4.1-mini",
        api_key_env: Some("OPENAI_API_KEY"),
    },
    ProviderPreset {
        name: "anthropic",
        backend: ModelBackend::Anthropic,
        endpoint: "https://api.anthropic.com/v1",
        model: "claude-sonnet-4-5",
        api_key_env: Some("ANTHROPIC_API_KEY"),
    },
    ProviderPreset {
        name: "gemini",
        backend: ModelBackend::Gemini,
        endpoint: "https://generativelanguage.googleapis.com/v1beta",
        model: "gemini-2.5-flash",
        api_key_env: Some("GEMINI_API_KEY"),
    },
    ProviderPreset {
        name: "openrouter",
        backend: ModelBackend::OpenAiCompatible,
        endpoint: "https://openrouter.ai/api/v1",
        model: "google/gemini-2.5-flash",
        api_key_env: Some("OPENROUTER_API_KEY"),
    },
    ProviderPreset {
        name: "groq",
        backend: ModelBackend::OpenAiCompatible,
        endpoint: "https://api.groq.com/openai/v1",
        model: "llama-3.3-70b-versatile",
        api_key_env: Some("GROQ_API_KEY"),
    },
    ProviderPreset {
        name: "ollama",
        backend: ModelBackend::Ollama,
        endpoint: "http://127.0.0.1:11434",
        model: "llama3.2",
        api_key_env: None,
    },
    ProviderPreset {
        name: "mistral",
        backend: ModelBackend::OpenAiCompatible,
        endpoint: "https://api.mistral.ai/v1",
        model: "mistral-small-latest",
        api_key_env: Some("MISTRAL_API_KEY"),
    },
    ProviderPreset {
        name: "together",
        backend: ModelBackend::OpenAiCompatible,
        endpoint: "https://api.together.xyz/v1",
        model: "meta-llama/Llama-3.3-70B-Instruct-Turbo",
        api_key_env: Some("TOGETHER_API_KEY"),
    },
    ProviderPreset {
        name: "deepseek",
        backend: ModelBackend::OpenAiCompatible,
        endpoint: "https://api.deepseek.com/v1",
        model: "deepseek-chat",
        api_key_env: Some("DEEPSEEK_API_KEY"),
    },
    ProviderPreset {
        name: "xai",
        backend: ModelBackend::OpenAiCompatible,
        endpoint: "https://api.x.ai/v1",
        model: "grok-3-mini",
        api_key_env: Some("XAI_API_KEY"),
    },
    ProviderPreset {
        name: "perplexity",
        backend: ModelBackend::OpenAiCompatible,
        endpoint: "https://api.perplexity.ai",
        model: "sonar",
        api_key_env: Some("PERPLEXITY_API_KEY"),
    },
    ProviderPreset {
        name: "fireworks",
        backend: ModelBackend::OpenAiCompatible,
        endpoint: "https://api.fireworks.ai/inference/v1",
        model: "accounts/fireworks/models/llama-v3p3-70b-instruct",
        api_key_env: Some("FIREWORKS_API_KEY"),
    },
];

impl CloudProviders {
    pub fn load() -> Self {
        let path = settings_path();
        let active = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<CloudProviderConfig>(&text).ok());
        Self { active, path }
    }

    pub fn presets() -> &'static [ProviderPreset] {
        PRESETS
    }

    pub fn select(&mut self, name: &str, model: Option<&str>) -> Result<ModelProfile, String> {
        let preset = PRESETS
            .iter()
            .find(|preset| {
                preset.name.eq_ignore_ascii_case(name)
                    || (preset.name == "groq" && name.eq_ignore_ascii_case("groqcloud"))
            })
            .ok_or_else(|| format!("Unknown provider '{name}'. Run /provider list."))?;
        self.active = Some(CloudProviderConfig {
            name: preset.name.to_string(),
            backend: preset.backend.clone(),
            endpoint: preset.endpoint.to_string(),
            model: model
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(preset.model)
                .to_string(),
            api_key_env: preset.api_key_env.map(str::to_string),
        });
        self.save()?;
        Ok(self.profile().expect("active configuration was set"))
    }

    pub fn configure_custom(
        &mut self,
        endpoint: &str,
        model: &str,
        api_key_env: &str,
    ) -> Result<ModelProfile, String> {
        let endpoint = endpoint.trim().trim_end_matches('/');
        if !(endpoint.starts_with("https://")
            || endpoint.starts_with("http://127.0.0.1")
            || endpoint.starts_with("http://localhost"))
        {
            return Err(
                "Custom OpenAI-compatible endpoints must use HTTPS (or a localhost URL)."
                    .to_string(),
            );
        }
        if model.trim().is_empty()
            || api_key_env.trim().is_empty()
            || !api_key_env
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
        {
            return Err(
                "Custom provider needs a model and an uppercase API-key environment variable name."
                    .to_string(),
            );
        }
        self.active = Some(CloudProviderConfig {
            name: "custom".to_string(),
            backend: ModelBackend::OpenAiCompatible,
            endpoint: endpoint.to_string(),
            model: model.trim().to_string(),
            api_key_env: Some(api_key_env.trim().to_string()),
        });
        self.save()?;
        Ok(self.profile().expect("active configuration was set"))
    }

    pub fn disable(&mut self) -> Result<(), String> {
        self.active = None;
        if self.path.exists() {
            fs::remove_file(&self.path)
                .map_err(|error| format!("Could not clear provider settings: {error}"))?;
        }
        Ok(())
    }

    pub fn profile(&self) -> Option<ModelProfile> {
        self.active.as_ref().map(|config| ModelProfile {
            id: config.model.clone(),
            label: format!("{} ({})", config.name, config.model),
            family: config.name.clone(),
            model_path: String::new(),
            llama_cli_path: String::new(),
            context_tokens: 16_384,
            max_tokens: 1024,
            temperature: 0.0,
            structured_output_strategy: "json".to_string(),
            chat_template: None,
            use_system_prompt: true,
            retry_count: 1,
            stop_sequences: Vec::new(),
            timeout_seconds: 90,
            backend: config.backend.clone(),
            endpoint: Some(config.endpoint.clone()),
            api_key_env: config.api_key_env.clone(),
        })
    }

    fn save(&self) -> Result<(), String> {
        let Some(active) = &self.active else {
            return Ok(());
        };
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("Could not create provider settings directory: {error}")
            })?;
        }
        let json = serde_json::to_string_pretty(active).map_err(|error| error.to_string())?;
        fs::write(&self.path, format!("{json}\n"))
            .map_err(|error| format!("Could not save provider settings: {error}"))
    }
}

fn settings_path() -> PathBuf {
    let root = if cfg!(windows) {
        env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home_dir())
            .join("AiSH")
    } else if cfg!(target_os = "macos") {
        home_dir().join("Applications").join("AiSH")
    } else {
        home_dir().join(".local").join("aish")
    };
    root.join("state").join("cloud-provider.json")
}

fn home_dir() -> PathBuf {
    env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn includes_requested_providers_and_uses_standard_byok_variables() {
        for name in [
            "openai",
            "anthropic",
            "gemini",
            "openrouter",
            "groq",
            "ollama",
        ] {
            assert!(CloudProviders::presets()
                .iter()
                .any(|preset| preset.name == name));
        }
        assert_eq!(
            CloudProviders::presets()
                .iter()
                .find(|preset| preset.name == "openai")
                .unwrap()
                .api_key_env,
            Some("OPENAI_API_KEY")
        );
    }
}
