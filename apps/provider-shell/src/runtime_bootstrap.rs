use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const MODEL_FILENAME: &str = "Qwen2.5-Coder-1.5B-Instruct-Q4_K_M.gguf";
const RELEASE_DOWNLOAD_BASE: &str = "https://github.com/DawnlightLabs/AiSH/releases/download";

pub fn configure() {
    let install_root = default_install_root();
    let model_path = configured_model_path(&install_root);
    env::set_var("AISH_MODEL_PATH", &model_path);

    let runtime_path = configured_runtime_path(&install_root);
    if !runtime_path.is_file() && is_managed_runtime_path(&runtime_path, &install_root) {
        if let Err(error) = install_runtime_from_current_release(&install_root) {
            eprintln!("AiSH runtime setup warning: {error}");
        }
    }
    env::set_var("AISH_LLAMA_CLI", &runtime_path);
}

fn configured_model_path(install_root: &Path) -> PathBuf {
    match env::var("AISH_MODEL_PATH") {
        Ok(value) if !value.trim().is_empty() => {
            let path = PathBuf::from(value);
            if is_legacy_model_path(&path) {
                managed_model_path(install_root)
            } else {
                path
            }
        }
        _ => managed_model_path(install_root),
    }
}

fn configured_runtime_path(install_root: &Path) -> PathBuf {
    if let Ok(value) = env::var("AISH_LLAMA_CLI") {
        if !value.trim().is_empty() {
            let path = PathBuf::from(value);
            if !is_legacy_runtime_path(&path) {
                return path;
            }
        }
    }

    if let Ok(current) = env::current_exe() {
        if let Some(parent) = current.parent() {
            let bundled = parent.join("runtime").join(runtime_filename());
            if bundled.is_file() {
                return bundled;
            }
        }
    }

    managed_runtime_path(install_root)
}

fn managed_model_path(install_root: &Path) -> PathBuf {
    install_root.join("models").join(MODEL_FILENAME)
}

fn managed_runtime_path(install_root: &Path) -> PathBuf {
    install_root.join("runtime").join(runtime_filename())
}

fn is_managed_runtime_path(path: &Path, install_root: &Path) -> bool {
    path == managed_runtime_path(install_root)
}

fn is_legacy_model_path(path: &Path) -> bool {
    let normalized = normalize_path(path);
    normalized.ends_with(&format!(
        "/downloads/aish-model/models/{}",
        MODEL_FILENAME.to_lowercase()
    ))
}

fn is_legacy_runtime_path(path: &Path) -> bool {
    let normalized = normalize_path(path);
    normalized.contains("/downloads/llama.cpp/")
        && (normalized.ends_with("/llama-cli") || normalized.ends_with("/llama-cli.exe"))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

fn runtime_filename() -> &'static str {
    if cfg!(target_os = "windows") {
        "llama-cli.exe"
    } else {
        "llama-cli"
    }
}

fn default_install_root() -> PathBuf {
    if cfg!(target_os = "windows") {
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

fn install_runtime_from_current_release(install_root: &Path) -> Result<(), String> {
    let asset = platform_asset()?;
    let tag = format!("v{}", env!("CARGO_PKG_VERSION"));
    let url = format!("{RELEASE_DOWNLOAD_BASE}/{tag}/{asset}");
    let work_dir = env::temp_dir().join(format!(
        "aish-runtime-bootstrap-{}-{}",
        env!("CARGO_PKG_VERSION"),
        std::process::id()
    ));
    let extract_dir = work_dir.join("extract");
    let archive_path = work_dir.join(asset);

    if work_dir.exists() {
        let _ = fs::remove_dir_all(&work_dir);
    }
    fs::create_dir_all(&extract_dir).map_err(|error| error.to_string())?;

    eprintln!("AiSH is installing its bundled local model runtime...");
    download_to_file(&url, &archive_path)?;
    extract_archive(asset, &archive_path, &extract_dir)?;

    let runtime = find_file(&extract_dir, runtime_filename())
        .ok_or_else(|| format!("release archive did not contain {}", runtime_filename()))?;
    let source_dir = runtime
        .parent()
        .ok_or_else(|| "bundled runtime path had no parent directory".to_string())?;
    let target_dir = install_root.join("runtime");
    let staged_dir = install_root.join("runtime.new");

    if staged_dir.exists() {
        fs::remove_dir_all(&staged_dir).map_err(|error| error.to_string())?;
    }
    copy_dir_contents(source_dir, &staged_dir)?;
    make_executable(&staged_dir.join(runtime_filename()))?;

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)
            .map_err(|error| format!("failed to replace {}: {error}", target_dir.display()))?;
    }
    fs::rename(&staged_dir, &target_dir)
        .map_err(|error| format!("failed to install {}: {error}", target_dir.display()))?;

    let _ = fs::remove_dir_all(&work_dir);
    eprintln!("AiSH local model runtime is ready.");
    Ok(())
}

fn platform_asset() -> Result<&'static str, String> {
    match (env::consts::OS, env::consts::ARCH) {
        ("windows", "x86_64") => Ok("aish-windows-x64.zip"),
        ("windows", "aarch64") => Ok("aish-windows-arm64.zip"),
        ("macos", "aarch64") => Ok("aish-macos-arm64.tar.gz"),
        ("linux", "x86_64") => Ok("aish-linux-x64.tar.gz"),
        ("linux", "aarch64") => Ok("aish-linux-arm64.tar.gz"),
        (os, arch) => Err(format!("local runtime is not available for {os}-{arch}")),
    }
}

fn download_to_file(url: &str, path: &Path) -> Result<(), String> {
    let response = ureq::get(url)
        .set("User-Agent", concat!("AiSH/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(120))
        .call()
        .map_err(|error| format!("failed to download bundled runtime: {error}"))?;
    let mut reader = response.into_reader();
    let mut file = File::create(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
    io::copy(&mut reader, &mut file).map_err(|error| error.to_string())?;
    file.flush().map_err(|error| error.to_string())?;
    Ok(())
}

fn extract_archive(asset: &str, archive_path: &Path, extract_dir: &Path) -> Result<(), String> {
    let status = if asset.ends_with(".zip") {
        Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Expand-Archive -LiteralPath $env:AISH_RUNTIME_ARCHIVE -DestinationPath $env:AISH_RUNTIME_EXTRACT -Force",
            ])
            .env("AISH_RUNTIME_ARCHIVE", archive_path)
            .env("AISH_RUNTIME_EXTRACT", extract_dir)
            .status()
    } else {
        Command::new("tar")
            .args([
                "-xzf",
                &archive_path.display().to_string(),
                "-C",
                &extract_dir.display().to_string(),
            ])
            .status()
    }
    .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("failed to extract {asset}"))
    }
}

fn find_file(root: &Path, filename: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.file_name().and_then(|value| value.to_str()) == Some(filename) {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file(&path, filename) {
                return Some(found);
            }
        }
    }
    None
}

fn copy_dir_contents(source: &Path, target: &Path) -> Result<(), String> {
    fs::create_dir_all(target)
        .map_err(|error| format!("failed to create {}: {error}", target.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("failed to read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_contents(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path).map_err(|error| {
                format!(
                    "failed to copy {} to {}: {error}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .map_err(|error| error.to_string())?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_legacy_model_path, is_legacy_runtime_path};
    use std::path::Path;

    #[test]
    fn detects_old_downloads_model_path() {
        assert!(is_legacy_model_path(Path::new(
            r"C:\Users\Amaan\Downloads\aish-model\models\Qwen2.5-Coder-1.5B-Instruct-Q4_K_M.gguf"
        )));
    }

    #[test]
    fn detects_old_llama_cpp_build_path() {
        assert!(is_legacy_runtime_path(Path::new(
            r"C:\Users\Amaan\Downloads\llama.cpp\build\bin\Release\llama-cli.exe"
        )));
    }

    #[test]
    fn preserves_non_legacy_paths() {
        assert!(!is_legacy_model_path(Path::new(r"D:\Models\custom.gguf")));
        assert!(!is_legacy_runtime_path(Path::new(
            r"D:\Tools\llama-cli.exe"
        )));
    }
}
