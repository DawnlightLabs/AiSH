use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(transparent)]
pub struct KnownFolders {
    paths: BTreeMap<String, String>,
}

impl KnownFolders {
    pub fn discover() -> Self {
        let mut folders = Self::default();
        discover_platform_folders(&mut folders);
        folders.insert_existing("temp", env::temp_dir());
        folders
    }

    fn insert_existing(&mut self, name: &str, path: PathBuf) {
        if path.is_dir() {
            self.paths
                .insert(name.to_string(), path.to_string_lossy().into_owned());
        }
    }
}

#[cfg(windows)]
fn discover_platform_folders(folders: &mut KnownFolders) {
    for (name, id) in [
        ("desktop", FOLDERID_DESKTOP),
        ("downloads", FOLDERID_DOWNLOADS),
        ("documents", FOLDERID_DOCUMENTS),
        ("pictures", FOLDERID_PICTURES),
        ("music", FOLDERID_MUSIC),
        ("videos", FOLDERID_VIDEOS),
    ] {
        if let Some(path) = windows_known_folder(id) {
            folders.insert_existing(name, path);
        }
    }
    if let Some(home) = env::var_os("USERPROFILE").map(PathBuf::from) {
        folders.insert_existing("home", home);
    }
}

#[cfg(target_os = "macos")]
fn discover_platform_folders(folders: &mut KnownFolders) {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    folders.insert_existing("home", home.clone());
    for (name, child) in [
        ("desktop", "Desktop"),
        ("downloads", "Downloads"),
        ("documents", "Documents"),
        ("pictures", "Pictures"),
        ("music", "Music"),
        ("videos", "Movies"),
    ] {
        folders.insert_existing(name, home.join(child));
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn discover_platform_folders(folders: &mut KnownFolders) {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    folders.insert_existing("home", home.clone());

    let configured = read_xdg_user_dirs(&home);
    for (name, child, key) in [
        ("desktop", "Desktop", "XDG_DESKTOP_DIR"),
        ("downloads", "Downloads", "XDG_DOWNLOAD_DIR"),
        ("documents", "Documents", "XDG_DOCUMENTS_DIR"),
        ("pictures", "Pictures", "XDG_PICTURES_DIR"),
        ("music", "Music", "XDG_MUSIC_DIR"),
        ("videos", "Videos", "XDG_VIDEOS_DIR"),
    ] {
        let path = configured
            .get(key)
            .cloned()
            .unwrap_or_else(|| home.join(child));
        folders.insert_existing(name, path);
    }
}

#[cfg(not(any(unix, windows)))]
fn discover_platform_folders(folders: &mut KnownFolders) {
    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        folders.insert_existing("home", home);
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn read_xdg_user_dirs(home: &std::path::Path) -> BTreeMap<String, PathBuf> {
    let path = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
        .join("user-dirs.dirs");
    let Ok(contents) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    parse_xdg_user_dirs(&contents, home)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn parse_xdg_user_dirs(contents: &str, home: &std::path::Path) -> BTreeMap<String, PathBuf> {
    let mut result = BTreeMap::new();
    for line in contents.lines() {
        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let value = raw_value.trim().trim_matches('"');
        let expanded = if let Some(rest) = value.strip_prefix("$HOME") {
            home.join(rest.trim_start_matches('/'))
        } else if let Some(rest) = value.strip_prefix("${HOME}") {
            home.join(rest.trim_start_matches('/'))
        } else {
            PathBuf::from(value)
        };
        if expanded.is_absolute() {
            result.insert(key.trim().to_string(), expanded);
        }
    }
    result
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    data1: u32,
    data2: u16,
    data3: u16,
    data4: [u8; 8],
}

#[cfg(windows)]
const FOLDERID_DESKTOP: Guid = Guid {
    data1: 0xb4bfcc3a,
    data2: 0xdb2c,
    data3: 0x424c,
    data4: [0xb0, 0x29, 0x7f, 0xe9, 0x9a, 0x87, 0xc6, 0x41],
};
#[cfg(windows)]
const FOLDERID_DOWNLOADS: Guid = Guid {
    data1: 0x374de290,
    data2: 0x123f,
    data3: 0x4565,
    data4: [0x91, 0x64, 0x39, 0xc4, 0x92, 0x5e, 0x46, 0x7b],
};
#[cfg(windows)]
const FOLDERID_DOCUMENTS: Guid = Guid {
    data1: 0xfdd39ad0,
    data2: 0x238f,
    data3: 0x46af,
    data4: [0xad, 0xb4, 0x6c, 0x85, 0x48, 0x03, 0x69, 0xc7],
};
#[cfg(windows)]
const FOLDERID_PICTURES: Guid = Guid {
    data1: 0x33e28130,
    data2: 0x4e1e,
    data3: 0x4676,
    data4: [0x83, 0x5a, 0x98, 0x39, 0x5c, 0x3b, 0xc3, 0xbb],
};
#[cfg(windows)]
const FOLDERID_MUSIC: Guid = Guid {
    data1: 0x4bd8d571,
    data2: 0x6d19,
    data3: 0x48d3,
    data4: [0xbe, 0x97, 0x42, 0x22, 0x20, 0x08, 0x0e, 0x43],
};
#[cfg(windows)]
const FOLDERID_VIDEOS: Guid = Guid {
    data1: 0x18989b1d,
    data2: 0x99b5,
    data3: 0x455b,
    data4: [0x84, 0x1c, 0xab, 0x7c, 0x74, 0xe4, 0xdd, 0xfc],
};

#[cfg(windows)]
fn windows_known_folder(id: Guid) -> Option<PathBuf> {
    use std::ffi::{c_void, OsString};
    use std::os::windows::ffi::OsStringExt;
    use std::ptr;

    #[link(name = "shell32")]
    extern "system" {
        fn SHGetKnownFolderPath(
            folder_id: *const Guid,
            flags: u32,
            token: *mut c_void,
            path: *mut *mut u16,
        ) -> i32;
    }
    #[link(name = "ole32")]
    extern "system" {
        fn CoTaskMemFree(memory: *mut c_void);
    }

    let mut raw = ptr::null_mut();
    let status = unsafe { SHGetKnownFolderPath(&id, 0, ptr::null_mut(), &mut raw) };
    if status < 0 || raw.is_null() {
        return None;
    }
    let length = unsafe {
        let mut length = 0;
        while *raw.add(length) != 0 {
            length += 1;
        }
        length
    };
    let path = unsafe { OsString::from_wide(std::slice::from_raw_parts(raw, length)) };
    unsafe { CoTaskMemFree(raw.cast()) };
    Some(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_only_returns_existing_directories() {
        let folders = KnownFolders::discover();
        assert!(folders
            .paths
            .values()
            .all(|path| std::path::Path::new(path).is_dir()));
        assert!(folders.paths.contains_key("temp"));
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn parses_xdg_paths_relative_to_home() {
        let home = std::path::Path::new("/home/example");
        let parsed = parse_xdg_user_dirs(
            "XDG_DESKTOP_DIR=\"$HOME/Desk Space\"\nXDG_DOWNLOAD_DIR=\"/data/downloads\"\n",
            home,
        );
        assert_eq!(parsed["XDG_DESKTOP_DIR"], home.join("Desk Space"));
        assert_eq!(parsed["XDG_DOWNLOAD_DIR"], PathBuf::from("/data/downloads"));
    }
}
