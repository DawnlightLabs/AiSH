use aish_ai::ModelProfile;
use std::fs;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
    serde_json::from_str(&text)
        .map_err(|error| format!("Failed to parse {}: {error}", path.display()))
}

pub fn list_profiles() -> Result<Vec<ModelProfile>, String> {
    for path in candidate_paths() {
        if path.exists() {
            return read_profiles(&path);
        }
    }
    Ok(vec![ModelProfile::default()])
}

pub fn save_profiles(profiles: Vec<ModelProfile>) -> Result<Vec<ModelProfile>, String> {
    let path = store_path();
    let text = serde_json::to_string_pretty(&profiles).map_err(|error| error.to_string())?;
    fs::write(&path, text).map_err(|error| format!("Failed to write {}: {error}", path.display()))?;
    Ok(profiles)
}

pub fn find_profile(id: &str) -> Result<ModelProfile, String> {
    list_profiles()?
        .into_iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| format!("missing profile: {id}"))
}
