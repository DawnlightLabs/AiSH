use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not available"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is not available"));

    generate_legacy_main(&manifest_dir, &out_dir);

    let source_png = manifest_dir
        .join("../..")
        .join("assets/png/aish-app-icon-dark-256x256.png");
    println!("cargo:rerun-if-changed={}", source_png.display());

    let png = fs::read(&source_png)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", source_png.display()));
    let ico = png_to_single_image_ico(&png, &source_png);

    let icon_path = out_dir.join("aish.ico");
    fs::write(&icon_path, ico)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", icon_path.display()));

    if env::var("CARGO_CFG_TARGET_OS").ok().as_deref() == Some("windows") {
        let icon = icon_path
            .to_str()
            .expect("generated icon path is not valid UTF-8");
        let mut resource = winres::WindowsResource::new();
        resource
            .set_icon(icon)
            .set("FileDescription", "AiSH - Artificially Intelligent Shell")
            .set("ProductName", "AiSH")
            .set("InternalName", "aish.exe")
            .set("OriginalFilename", "aish.exe")
            .set("CompanyName", "Dawnlight Labs")
            .set("LegalCopyright", "Copyright (c) 2026 Dawnlight Labs");
        resource
            .compile()
            .expect("failed to compile AiSH Windows resources");
    }
}

fn generate_legacy_main(manifest_dir: &Path, out_dir: &Path) {
    let source_path = manifest_dir.join("src/main_setup.rs");
    println!("cargo:rerun-if-changed={}", source_path.display());

    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", source_path.display()));
    let generated = source
        .replacen("mod logging;", "use crate::logging;", 1)
        .replacen("mod setup;", "use crate::setup;", 1)
        .replacen("mod updater;", "use crate::updater;", 1)
        .replacen("fn main()", "pub fn run()", 1);

    if !generated.contains("pub fn run()") {
        panic!("failed to generate provider shell entrypoint");
    }

    let generated_path = out_dir.join("main_setup_generated.rs");
    fs::write(&generated_path, generated)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", generated_path.display()));
}

fn png_to_single_image_ico(png: &[u8], source: &Path) -> Vec<u8> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    assert!(
        png.starts_with(PNG_SIGNATURE),
        "{} is not a PNG file",
        source.display()
    );

    let image_size = u32::try_from(png.len()).expect("AiSH icon is too large for an ICO file");
    let mut ico = Vec::with_capacity(22 + png.len());

    // ICONDIR: reserved, image type (1 = icon), image count.
    ico.extend_from_slice(&0_u16.to_le_bytes());
    ico.extend_from_slice(&1_u16.to_le_bytes());
    ico.extend_from_slice(&1_u16.to_le_bytes());

    // ICONDIRENTRY. Width/height 0 represents 256 pixels in the ICO format.
    ico.push(0);
    ico.push(0);
    ico.push(0);
    ico.push(0);
    ico.extend_from_slice(&1_u16.to_le_bytes());
    ico.extend_from_slice(&32_u16.to_le_bytes());
    ico.extend_from_slice(&image_size.to_le_bytes());
    ico.extend_from_slice(&22_u32.to_le_bytes());
    ico.extend_from_slice(png);

    ico
}
