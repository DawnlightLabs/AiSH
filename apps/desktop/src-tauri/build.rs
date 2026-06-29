use image::{imageops::FilterType, ImageFormat, Rgb, RgbImage, RgbaImage};
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
    println!("cargo:rerun-if-changed=app-icon.png");
    ensure_icons();

    let base = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let art_dir = base.join("installer");
    fs::create_dir_all(&art_dir).expect("failed to create artwork directory");

    let logo = load_app_icon(&base);
    create_banner_bmp(&art_dir.join("wix-banner.bmp"), logo.as_ref());
    create_dialog_bmp(&art_dir.join("wix-dialog.bmp"), logo.as_ref());

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

fn load_app_icon(base: &Path) -> Option<RgbaImage> {
    let path = base.join("app-icon.png");
    image::open(&path).ok().map(|image| image.to_rgba8())
}

fn create_banner_bmp(path: &Path, logo: Option<&RgbaImage>) {
    let mut image = RgbImage::new(493, 58);
    fill_light(&mut image);
    draw_separator(&mut image, 57, Rgb([210, 201, 184]));
    if let Some(logo) = logo {
        paste_logo(&mut image, logo, 418, 8, 42);
    } else {
        draw_fallback_mark(&mut image, 420, 10, 38, true);
    }
    save_bmp(path, &image);
}

fn create_dialog_bmp(path: &Path, logo: Option<&RgbaImage>) {
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

    if let Some(logo) = logo {
        paste_logo(&mut image, logo, 44, 68, 92);
    } else {
        draw_fallback_mark(&mut image, 54, 72, 74, false);
    }

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

fn draw_fallback_mark(image: &mut RgbImage, ox: u32, oy: u32, size: u32, small: bool) {
    let light = Rgb([246, 239, 222]);
    let dark = Rgb([45, 39, 32]);
    for y in oy..oy + size {
        for x in ox..ox + size {
            let lx = x - ox;
            let ly = y - oy;
            let edge = lx < 3 || ly < 3 || lx > size - 4 || ly > size - 4;
            let eye = (ly > size / 2 - 4 && ly < size / 2 + 4)
                && ((lx > size / 3 - 4 && lx < size / 3 + 4)
                    || (lx > size * 2 / 3 - 4 && lx < size * 2 / 3 + 4));
            let cursor = ly > size * 2 / 3 && ly < size * 2 / 3 + 4 && lx > size / 2 - 12 && lx < size / 2 + 12;
            image.put_pixel(x, y, if edge || eye || cursor { light } else { dark });
        }
    }

    if !small {
        draw_separator(image, oy + size + 28, Rgb([72, 62, 52]));
    }
}

fn save_bmp(path: &Path, image: &RgbImage) {
    image
        .save_with_format(path, ImageFormat::Bmp)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
}

fn write_if_missing(path: &Path, bytes: &[u8]) {
    if path.exists() {
        return;
    }
    fs::write(path, bytes).unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
}
