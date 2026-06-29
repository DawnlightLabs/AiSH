use std::fs;
use std::path::{Path, PathBuf};

const APP_ICON_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 16, 0, 0, 0, 16, 8, 6, 0,
    0, 0, 31, 243, 255, 97, 0, 0, 0, 24, 73, 68, 65, 84, 120, 218, 99, 96, 8, 117, 248, 79, 17, 30,
    53, 96, 212, 128, 81, 3, 134, 139, 1, 0, 96, 13, 148, 16, 206, 244, 68, 43, 0, 0, 0, 0, 73, 69,
    78, 68, 174, 66, 96, 130,
];

const APP_ICON_ICO: &[u8] = &[
    0, 0, 1, 0, 1, 0, 16, 16, 0, 0, 1, 0, 32, 0, 81, 0, 0, 0, 22, 0, 0, 0, 137, 80, 78, 71, 13, 10,
    26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 16, 0, 0, 0, 16, 8, 6, 0, 0, 0, 31, 243, 255, 97,
    0, 0, 0, 24, 73, 68, 65, 84, 120, 218, 99, 96, 8, 117, 248, 79, 17, 30, 53, 96, 212, 128, 81, 3,
    134, 139, 1, 0, 96, 13, 148, 16, 206, 244, 68, 43, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

const APP_ICON_ICNS: &[u8] = &[
    105, 99, 110, 115, 0, 0, 0, 97, 105, 99, 112, 52, 0, 0, 0, 89, 137, 80, 78, 71, 13, 10, 26, 10,
    0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 16, 0, 0, 0, 16, 8, 6, 0, 0, 0, 31, 243, 255, 97, 0, 0, 0,
    24, 73, 68, 65, 84, 120, 218, 99, 96, 8, 117, 248, 79, 17, 30, 53, 96, 212, 128, 81, 3, 134,
    139, 1, 0, 96, 13, 148, 16, 206, 244, 68, 43, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn main() {
    ensure_icons();
    tauri_build::build();
}

fn ensure_icons() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let icons_dir = manifest_dir.join("icons");
    fs::create_dir_all(&icons_dir).expect("failed to create icons directory");

    write_if_missing(&icons_dir.join("icon.ico"), APP_ICON_ICO);
    write_if_missing(&icons_dir.join("icon.icns"), APP_ICON_ICNS);
    write_if_missing(&icons_dir.join("32x32.png"), APP_ICON_PNG);
    write_if_missing(&icons_dir.join("128x128.png"), APP_ICON_PNG);
    write_if_missing(&icons_dir.join("128x128@2x.png"), APP_ICON_PNG);
}

fn write_if_missing(path: &Path, bytes: &[u8]) {
    if path.exists() {
        return;
    }
    fs::write(path, bytes).unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
}
