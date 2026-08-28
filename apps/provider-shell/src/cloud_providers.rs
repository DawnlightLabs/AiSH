use aish_ai::{ModelBackend, ModelProfile};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

const KEYRING_SERVICE: &str = "AiSH BYOK";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviders {
    pub active: Option<CloudProviderConfig>,
    #[serde(default)]
    pub enabled: Vec<CloudProviderConfig>,
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

#[derive(Debug, Serialize, Deserialize)]
struct SavedProviderSettings {
    #[serde(default)]
    enabled: Vec<CloudProviderConfig>,
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
        let text = fs::read_to_string(&path).ok();
        let enabled = text
            .as_deref()
            .and_then(|text| serde_json::from_str::<SavedProviderSettings>(text).ok())
            .map(|settings| settings.enabled)
            .or_else(|| {
                text.as_deref()
                    .and_then(|text| serde_json::from_str::<CloudProviderConfig>(text).ok())
                    .map(|config| vec![config])
            })
            .unwrap_or_default();
        let active = enabled.first().cloned();
        Self {
            active,
            enabled,
            path,
        }
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
        self.enabled = self.active.iter().cloned().collect();
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
        self.enabled = self.active.iter().cloned().collect();
        self.save()?;
        Ok(self.profile().expect("active configuration was set"))
    }

    pub fn disable(&mut self) -> Result<(), String> {
        self.active = None;
        self.enabled.clear();
        if self.path.exists() {
            fs::remove_file(&self.path)
                .map_err(|error| format!("Could not clear provider settings: {error}"))?;
        }
        Ok(())
    }

    pub fn profile(&self) -> Option<ModelProfile> {
        let mut profiles = self.enabled.iter().map(profile_for).collect::<Vec<_>>();
        let mut primary = profiles.first().cloned()?;
        primary.fallback_profiles = profiles.drain(1..).collect();
        Some(primary)
    }

    pub fn configure_interactively(&mut self) -> Result<Option<ModelProfile>, String> {
        if !prompt_yes_no("Enable cloud BYOK providers", false) {
            self.disable()?;
            println!("Cloud BYOK disabled. AiSH will use your local GGUF model.");
            return Ok(None);
        }
        println!(
            "Select one or more providers in fallback order (for example: openrouter,gemini,groq)."
        );
        println!("Enabled providers are tried left-to-right whenever a request fails.");
        for preset in PRESETS {
            println!("  {} — default model: {}", preset.name, preset.model);
        }
        println!("  custom — OpenAI-compatible endpoint");
        let selected = prompt_line("Providers", "");
        let mut enabled = Vec::new();
        for requested in selected
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if requested.eq_ignore_ascii_case("custom") {
                let endpoint = prompt_line("Custom endpoint", "https://");
                let model = prompt_line("Custom model", "");
                let key_env = prompt_line("Custom key environment variable", "CUSTOM_API_KEY");
                let config = custom_config(&endpoint, &model, &key_env)?;
                capture_key(&config)?;
                enabled.push(config);
                continue;
            }
            let preset =
                find_preset(requested).ok_or_else(|| format!("Unknown provider '{requested}'."))?;
            let model = prompt_line(&format!("{} model", preset.name), preset.model);
            let config = CloudProviderConfig {
                name: preset.name.to_string(),
                backend: preset.backend.clone(),
                endpoint: preset.endpoint.to_string(),
                model,
                api_key_env: preset.api_key_env.map(str::to_string),
            };
            capture_key(&config)?;
            enabled.push(config);
        }
        if enabled.is_empty() {
            return Err("Choose at least one provider, or answer no to disable BYOK.".to_string());
        }
        self.enabled = enabled;
        self.active = self.enabled.first().cloned();
        self.save()?;
        Ok(self.profile())
    }

    pub fn enabled_names(&self) -> Vec<String> {
        self.enabled
            .iter()
            .map(|config| format!("{} ({})", config.name, config.model))
            .collect()
    }

    fn save(&self) -> Result<(), String> {
        if self.enabled.is_empty() {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("Could not create provider settings directory: {error}")
            })?;
        }
        let json = serde_json::to_string_pretty(&SavedProviderSettings {
            enabled: self.enabled.clone(),
        })
        .map_err(|error| error.to_string())?;
        fs::write(&self.path, format!("{json}\n"))
            .map_err(|error| format!("Could not save provider settings: {error}"))
    }
}

fn profile_for(config: &CloudProviderConfig) -> ModelProfile {
    ModelProfile {
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
        api_key_service: config
            .api_key_env
            .as_ref()
            .map(|_| KEYRING_SERVICE.to_string()),
        fallback_profiles: Vec::new(),
    }
}

fn find_preset(name: &str) -> Option<&'static ProviderPreset> {
    PRESETS.iter().find(|preset| {
        preset.name.eq_ignore_ascii_case(name)
            || (preset.name == "groq" && name.eq_ignore_ascii_case("groqcloud"))
    })
}

fn custom_config(
    endpoint: &str,
    model: &str,
    api_key_env: &str,
) -> Result<CloudProviderConfig, String> {
    let endpoint = endpoint.trim().trim_end_matches('/');
    if !(endpoint.starts_with("https://")
        || endpoint.starts_with("http://127.0.0.1")
        || endpoint.starts_with("http://localhost"))
    {
        return Err(
            "Custom OpenAI-compatible endpoints must use HTTPS (or a localhost URL).".to_string(),
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
    Ok(CloudProviderConfig {
        name: "custom".to_string(),
        backend: ModelBackend::OpenAiCompatible,
        endpoint: endpoint.to_string(),
        model: model.trim().to_string(),
        api_key_env: Some(api_key_env.trim().to_string()),
    })
}

fn capture_key(config: &CloudProviderConfig) -> Result<(), String> {
    let Some(variable) = &config.api_key_env else {
        return Ok(());
    };
    println!(
        "{} uses {variable}. Leave this blank to supply it through the environment later.",
        config.name
    );
    let key = prompt_line(&format!("{} API key", config.name), "");
    if key.trim().is_empty() {
        return Ok(());
    }
    let entry = keyring::Entry::new(KEYRING_SERVICE, &config.name)
        .map_err(|error| format!("Could not access OS credential vault: {error}"))?;
    entry.set_password(key.trim()).map_err(|error| {
        format!(
            "Could not save {} key in OS credential vault: {error}",
            config.name
        )
    })
}

fn prompt_yes_no(label: &str, default_yes: bool) -> bool {
    let suffix = if default_yes { "Y/n" } else { "y/N" };
    print!("{label} [{suffix}]: ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return default_yes;
    }
    match input.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default_yes,
    }
}

fn prompt_line(label: &str, default: &str) -> String {
    if default.is_empty() {
        print!("{label}: ");
    } else {
        print!("{label} [{default}]: ");
    }
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return default.to_string();
    }
    let value = input.trim();
    if value.is_empty() {
        default.to_string()
    } else {
        value.to_string()
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
