use aish_ai::ModelProfile;
use std::fs;
use std::path::PathBuf;

fn store_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("model_profiles.json")
}

pub fn list_profiles() -> Result<Vec<ModelProfile>, String> {
    let path = store_path();
    if !path.exists() {
        return Ok(vec![ModelProfile::default()]);
    }
    let text = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

pub fn save_profiles(profiles: Vec<ModelProfile>) -> Result<Vec<ModelProfile>, String> {
    let text = serde_json::to_string_pretty(&profiles).map_err(|error| error.to_string())?;
    fs::write(store_path(), text).map_err(|error| error.to_string())?;
    Ok(profiles)
}

pub fn find_profile(id: &str) -> Result<ModelProfile, String> {
    list_profiles()?
        .into_iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| format!("missing profile: {id}"))
}
