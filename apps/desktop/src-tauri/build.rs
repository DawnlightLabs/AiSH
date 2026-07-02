use image::{imageops::FilterType, DynamicImage, ImageFormat, Rgb, RgbImage, RgbaImage};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=app-icon.png");

    let base =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let logo = load_app_icon(&base);

    ensure_icons(&base, &logo);
    ensure_provider_sidecar_for_tauri_check(&base);

    let art_dir = base.join("installer");
    fs::create_dir_all(&art_dir).expect("failed to create artwork directory");
    create_banner_bmp(&art_dir.join("wix-banner.bmp"), &logo);
    create_dialog_bmp(&art_dir.join("wix-dialog.bmp"), &logo);

    tauri_build::build();
}

fn ensure_provider_sidecar_for_tauri_check(base: &Path) {
    let Some(target) = std::env::var("TARGET").ok() else {
        return;
    };

    let ext = if target.contains("windows") { ".exe" } else { "" };
    let sidecar_dir = base.join("binaries");
    let sidecar = sidecar_dir.join(format!("aish-provider-shell-{target}{ext}"));
    if sidecar.exists() {
        return;
    }

    let repo_root = base
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .unwrap_or(base);
    let release_provider = repo_root
        .join("target")
        .join("release")
        .join(if target.contains("windows") { "aish.exe" } else { "aish" });

    fs::create_dir_all(&sidecar_dir).expect("failed to create sidecar directory");
    if release_provider.exists() {
        fs::copy(&release_provider, &sidecar)
            .unwrap_or_else(|error| panic!("failed to copy provider sidecar: {error}"));
    } else {
        fs::write(
            &sidecar,
            b"AiSH provider sidecar placeholder for cargo check. Run npm run provider:sidecar before packaging.\n",
        )
        .unwrap_or_else(|error| panic!("failed to write sidecar placeholder: {error}"));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&sidecar)
            .expect("sidecar metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&sidecar, perms).expect("sidecar executable permissions");
    }
}

fn load_app_icon(base: &Path) -> RgbaImage {
    let path = base.join("app-icon.png");
    image::open(&path)
        .unwrap_or_else(|error| {
            panic!(
                "failed to load AiSH app icon at {}. Put the real square PNG at apps/desktop/src-tauri/app-icon.png before building: {error}",
                path.display()
            )
        })
        .to_rgba8()
}

fn ensure_icons(base: &Path, logo: &RgbaImage) {
    let icons_dir = base.join("icons");
    fs::create_dir_all(&icons_dir).expect("failed to create icons directory");

    let png32 = resized_png(logo, 32);
    let png128 = resized_png(logo, 128);
    let png256 = resized_png(logo, 256);

    write_bytes(&icons_dir.join("32x32.png"), &png32);
    write_bytes(&icons_dir.join("128x128.png"), &png128);
    write_bytes(&icons_dir.join("128x128@2x.png"), &png256);
    write_bytes(&icons_dir.join("icon.png"), &png256);
    write_png_ico(&icons_dir.join("icon.ico"), &png256);
    write_icns(&icons_dir.join("icon.icns"), &png128, &png256);
}

fn resized_png(logo: &RgbaImage, size: u32) -> Vec<u8> {
    let resized = image::imageops::resize(logo, size, size, FilterType::Lanczos3);
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(resized)
        .write_to(&mut cursor, ImageFormat::Png)
        .expect("failed to encode resized PNG icon");
    cursor.into_inner()
}

fn write_png_ico(path: &Path, png256: &[u8]) {
    let mut bytes = Vec::with_capacity(22 + png256.len());
    bytes.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(&(png256.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&22u32.to_le_bytes());
    bytes.extend_from_slice(png256);
    write_bytes(path, &bytes);
}

fn write_icns(path: &Path, png128: &[u8], png256: &[u8]) {
    let total_len = 8 + 8 + png128.len() + 8 + png256.len();
    let mut bytes = Vec::with_capacity(total_len);
    bytes.extend_from_slice(b"icns");
    bytes.extend_from_slice(&(total_len as u32).to_be_bytes());
    push_icns_entry(&mut bytes, b"ic07", png128);
    push_icns_entry(&mut bytes, b"ic08", png256);
    write_bytes(path, &bytes);
}

fn push_icns_entry(bytes: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    bytes.extend_from_slice(kind);
    bytes.extend_from_slice(&((data.len() + 8) as u32).to_be_bytes());
    bytes.extend_from_slice(data);
}

fn create_banner_bmp(path: &Path, logo: &RgbaImage) {
    let mut image = RgbImage::new(493, 58);
    fill_light(&mut image);
    draw_separator(&mut image, 57, Rgb([210, 201, 184]));
    paste_logo(&mut image, logo, 418, 8, 42);
    save_bmp(path, &image);
}

fn create_dialog_bmp(path: &Path, logo: &RgbaImage) {
    let mut image = RgbImage::new(493, 312);
    for y in 0..image.height() {
        for x in 0..image.width() {
            let shade = y as f32 / image.height() as f32;
            let pixel = if x < 172 {
                Rgb([
                    (58.0 - 34.0 * shade) as u8,
                    (51.0 - 30.0 * shade) as u8,
                    (43.0 - 26.0 * shade) as u8,
                ])
            } else {
                let t = (x - 172) as f32 / 321.0;
                Rgb([
                    (242.0 - 18.0 * t - 9.0 * shade) as u8,
                    (236.0 - 20.0 * t - 10.0 * shade) as u8,
                    (222.0 - 24.0 * t - 12.0 * shade) as u8,
                ])
            };
            image.put_pixel(x, y, pixel);
        }
    }

    paste_logo(&mut image, logo, 28, 54, 116);
    save_bmp(path, &image);
}

fn fill_light(image: &mut RgbImage) {
    for y in 0..image.height() {
        for x in 0..image.width() {
            let t = x as f32 / image.width() as f32;
            let shade = y as f32 / image.height() as f32;
            image.put_pixel(
                x,
                y,
                Rgb([
                    (247.0 - 22.0 * t - 6.0 * shade) as u8,
                    (241.0 - 24.0 * t - 8.0 * shade) as u8,
                    (226.0 - 28.0 * t - 10.0 * shade) as u8,
                ]),
            );
        }
    }
}

fn draw_separator(image: &mut RgbImage, y: u32, color: Rgb<u8>) {
    if y >= image.height() {
        return;
    }
    for x in 0..image.width() {
        image.put_pixel(x, y, color);
    }
}

fn paste_logo(canvas: &mut RgbImage, logo: &RgbaImage, x: i32, y: i32, size: u32) {
    let resized = image::imageops::resize(logo, size, size, FilterType::Lanczos3);
    for py in 0..size {
        for px in 0..size {
            let tx = x + px as i32;
            let ty = y + py as i32;
            if tx < 0 || ty < 0 || tx >= canvas.width() as i32 || ty >= canvas.height() as i32 {
                continue;
            }

            let src = resized.get_pixel(px, py).0;
            let alpha = src[3] as f32 / 255.0;
            if alpha <= 0.0 {
                continue;
            }

            let dest = canvas.get_pixel_mut(tx as u32, ty as u32);
            let dst = dest.0;
            *dest = Rgb([
                blend(src[0], dst[0], alpha),
                blend(src[1], dst[1], alpha),
                blend(src[2], dst[2], alpha),
            ]);
        }
    }
}

fn blend(src: u8, dst: u8, alpha: f32) -> u8 {
    (src as f32 * alpha + dst as f32 * (1.0 - alpha)).round() as u8
}

fn save_bmp(path: &Path, image: &RgbImage) {
    image
        .save_with_format(path, ImageFormat::Bmp)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
}

fn write_bytes(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
}
