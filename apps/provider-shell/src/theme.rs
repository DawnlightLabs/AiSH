use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

const RESET: &str = "\x1b[0m";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreset {
    Dawnlight,
    Ocean,
    Forest,
    Ember,
    Violet,
    Mono,
    Off,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThemeSettings {
    #[serde(default = "default_preset")]
    preset: ThemePreset,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            preset: default_preset(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    preset: ThemePreset,
    color_enabled: bool,
}

#[derive(Debug, Clone, Copy)]
struct Palette {
    accent: &'static str,
    prompt: &'static str,
    success: &'static str,
    warning: &'static str,
    error: &'static str,
    muted: &'static str,
    command: &'static str,
}

impl Theme {
    pub fn load() -> Self {
        let preset = read_settings(&settings_path()).preset;
        Self::new(preset, terminal_supports_color())
    }

    fn new(preset: ThemePreset, terminal_color: bool) -> Self {
        Self {
            preset,
            color_enabled: terminal_color && preset != ThemePreset::Off,
        }
    }

    pub fn use_preset(&mut self, value: &str) -> Result<(), String> {
        let preset = parse_preset(value)
            .ok_or_else(|| format!("unknown theme '{value}'. Use /theme list."))?;
        write_settings(&settings_path(), &ThemeSettings { preset })?;
        self.preset = preset;
        self.color_enabled = terminal_supports_color() && preset != ThemePreset::Off;
        Ok(())
    }

    pub fn name(&self) -> &'static str {
        preset_name(self.preset)
    }

    pub fn color_enabled(&self) -> bool {
        self.color_enabled
    }

    pub fn accent(&self, value: impl AsRef<str>) -> String {
        self.paint(self.palette().accent, value)
    }

    pub fn prompt(&self, value: impl AsRef<str>) -> String {
        self.paint(self.palette().prompt, value)
    }

    pub fn success(&self, value: impl AsRef<str>) -> String {
        self.paint(self.palette().success, value)
    }

    pub fn warning(&self, value: impl AsRef<str>) -> String {
        self.paint(self.palette().warning, value)
    }

    pub fn error(&self, value: impl AsRef<str>) -> String {
        self.paint(self.palette().error, value)
    }

    pub fn muted(&self, value: impl AsRef<str>) -> String {
        self.paint(self.palette().muted, value)
    }

    pub fn command(&self, value: impl AsRef<str>) -> String {
        self.paint(self.palette().command, value)
    }

    pub fn preview(&self) {
        println!(
            "{}  {}  {}  {}  {}  {}",
            self.accent("AiSH"),
            self.prompt("prompt"),
            self.success("success"),
            self.warning("approval"),
            self.error("error"),
            self.muted("details")
        );
    }

    fn paint(&self, color: &str, value: impl AsRef<str>) -> String {
        let value = value.as_ref();
        if !self.color_enabled {
            return value.to_string();
        }
        format!("\x1b[{color}m{value}{RESET}")
    }

    fn palette(&self) -> Palette {
        match self.preset {
            // Dawnlight Labs "First Light": gold, dawn blue, and warm ivory.
            ThemePreset::Dawnlight => Palette::new(
                "38;2;211;161;75",
                "38;2;117;168;206",
                "38;2;145;185;215",
                "38;2;240;197;107",
                "38;2;224;108;117",
                "38;2;201;192;175",
                "38;2;252;248;245",
            ),
            ThemePreset::Ocean => Palette::new("94", "96", "92", "93", "91", "90", "97"),
            ThemePreset::Forest => Palette::new("92", "96", "32", "93", "91", "90", "97"),
            ThemePreset::Ember => Palette::new("93", "91", "92", "33", "31", "90", "97"),
            ThemePreset::Violet => Palette::new("95", "94", "92", "93", "91", "90", "97"),
            ThemePreset::Mono | ThemePreset::Off => {
                Palette::new("97", "97", "97", "97", "97", "90", "97")
            }
        }
    }
}

impl Palette {
    const fn new(
        accent: &'static str,
        prompt: &'static str,
        success: &'static str,
        warning: &'static str,
        error: &'static str,
        muted: &'static str,
        command: &'static str,
    ) -> Self {
        Self {
            accent,
            prompt,
            success,
            warning,
            error,
            muted,
            command,
        }
    }
}

pub fn preset_names() -> &'static [&'static str] {
    &[
        "dawnlight",
        "ocean",
        "forest",
        "ember",
        "violet",
        "mono",
        "off",
    ]
}

fn parse_preset(value: &str) -> Option<ThemePreset> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dawnlight" | "default" | "auto" | "system" => Some(ThemePreset::Dawnlight),
        "ocean" | "blue" => Some(ThemePreset::Ocean),
        "forest" | "green" => Some(ThemePreset::Forest),
        "ember" | "orange" | "warm" => Some(ThemePreset::Ember),
        "violet" | "purple" => Some(ThemePreset::Violet),
        "mono" | "monochrome" => Some(ThemePreset::Mono),
        "off" | "none" | "plain" => Some(ThemePreset::Off),
        _ => None,
    }
}

fn preset_name(preset: ThemePreset) -> &'static str {
    match preset {
        ThemePreset::Dawnlight => "dawnlight",
        ThemePreset::Ocean => "ocean",
        ThemePreset::Forest => "forest",
        ThemePreset::Ember => "ember",
        ThemePreset::Violet => "violet",
        ThemePreset::Mono => "mono",
        ThemePreset::Off => "off",
    }
}

fn default_preset() -> ThemePreset {
    ThemePreset::Dawnlight
}

fn terminal_supports_color() -> bool {
    if std::env::var_os("NO_COLOR").is_some()
        || std::env::var("TERM")
            .ok()
            .is_some_and(|value| value.eq_ignore_ascii_case("dumb"))
    {
        return false;
    }
    if std::env::var("CLICOLOR_FORCE")
        .ok()
        .is_some_and(|value| value != "0")
    {
        return true;
    }
    if !io::stdout().is_terminal() {
        return false;
    }
    enable_windows_virtual_terminal()
}

#[cfg(not(target_os = "windows"))]
fn enable_windows_virtual_terminal() -> bool {
    true
}

#[cfg(target_os = "windows")]
fn enable_windows_virtual_terminal() -> bool {
    use std::ffi::c_void;

    const STD_OUTPUT_HANDLE: i32 = -11;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    const INVALID_HANDLE_VALUE: isize = -1;

    #[link(name = "Kernel32")]
    extern "system" {
        fn GetStdHandle(standard_handle: i32) -> *mut c_void;
        fn GetConsoleMode(console_handle: *mut c_void, mode: *mut u32) -> i32;
        fn SetConsoleMode(console_handle: *mut c_void, mode: u32) -> i32;
    }

    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() || handle as isize == INVALID_HANDLE_VALUE {
            return false;
        }
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return false;
        }
        SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
    }
}

fn settings_path() -> PathBuf {
    aish_logging::app_data_dir().join("theme-settings.json")
}

fn read_settings(path: &Path) -> ThemeSettings {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_settings(path: &Path, settings: &ThemeSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    fs::write(path, text).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{parse_preset, read_settings, write_settings, Theme, ThemePreset, ThemeSettings};
    use std::fs;

    #[test]
    fn aliases_map_to_portable_presets() {
        assert_eq!(parse_preset("system"), Some(ThemePreset::Dawnlight));
        assert_eq!(parse_preset("purple"), Some(ThemePreset::Violet));
        assert_eq!(parse_preset("plain"), Some(ThemePreset::Off));
        assert_eq!(parse_preset("unknown"), None);
    }

    #[test]
    fn color_can_be_disabled_without_leaking_escape_sequences() {
        let theme = Theme::new(ThemePreset::Ocean, false);
        assert_eq!(theme.accent("AiSH"), "AiSH");
        let enabled = Theme::new(ThemePreset::Ocean, true);
        assert!(enabled.accent("AiSH").starts_with("\u{1b}["));
        assert!(enabled.accent("AiSH").ends_with("\u{1b}[0m"));
    }

    #[test]
    fn dawnlight_uses_the_first_light_brand_palette_without_purple() {
        let theme = Theme::new(ThemePreset::Dawnlight, true);
        assert!(theme.accent("AiSH").starts_with("\u{1b}[38;2;211;161;75m"));
        assert!(theme
            .prompt("prompt")
            .starts_with("\u{1b}[38;2;117;168;206m"));
        assert!(theme
            .command("command")
            .starts_with("\u{1b}[38;2;252;248;245m"));
        assert!(!theme.accent("AiSH").contains("[95m"));
        assert!(!theme.prompt("prompt").contains("[95m"));
    }

    #[test]
    fn settings_round_trip_and_invalid_files_fall_back() {
        let root = std::env::temp_dir().join(format!("aish-theme-test-{}", std::process::id()));
        let path = root.join("theme.json");
        write_settings(
            &path,
            &ThemeSettings {
                preset: ThemePreset::Forest,
            },
        )
        .unwrap();
        assert_eq!(read_settings(&path).preset, ThemePreset::Forest);
        fs::write(&path, "{invalid").unwrap();
        assert_eq!(read_settings(&path).preset, ThemePreset::Dawnlight);
        let _ = fs::remove_dir_all(root);
    }
}
