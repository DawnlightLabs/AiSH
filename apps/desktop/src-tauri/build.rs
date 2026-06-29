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
    let base = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let art_dir = base.join("installer");
    fs::create_dir_all(&art_dir).expect("failed to create artwork directory");
    create_bmp(&art_dir.join("wix-banner.bmp"), 493, 58, true);
    create_bmp(&art_dir.join("wix-dialog.bmp"), 493, 312, false);
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

fn create_bmp(path: &Path, width: usize, height: usize, small: bool) {
    let stride = (width * 3 + 3) & !3;
    let image_size = stride * height;
    let file_size = 54 + image_size;
    let mut data = Vec::with_capacity(file_size);
    data.extend_from_slice(b"BM");
    data.extend_from_slice(&(file_size as u32).to_le_bytes());
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&(54u32).to_le_bytes());
    data.extend_from_slice(&(40u32).to_le_bytes());
    data.extend_from_slice(&(width as i32).to_le_bytes());
    data.extend_from_slice(&(height as i32).to_le_bytes());
    data.extend_from_slice(&(1u16).to_le_bytes());
    data.extend_from_slice(&(24u16).to_le_bytes());
    data.extend_from_slice(&(0u32).to_le_bytes());
    data.extend_from_slice(&(image_size as u32).to_le_bytes());
    data.extend_from_slice(&(2835u32).to_le_bytes());
    data.extend_from_slice(&(2835u32).to_le_bytes());
    data.extend_from_slice(&(0u32).to_le_bytes());
    data.extend_from_slice(&(0u32).to_le_bytes());

    for y in (0..height).rev() {
        let row_start = data.len();
        for x in 0..width {
            let (r, g, b) = art_pixel(x, y, width, height, small);
            data.extend_from_slice(&[b, g, r]);
        }
        while data.len() - row_start < stride {
            data.push(0);
        }
    }
    fs::write(path, data).unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
}

fn art_pixel(x: usize, y: usize, width: usize, height: usize, small: bool) -> (u8, u8, u8) {
    let t = x as f32 / width as f32;
    let shade = y as f32 / height as f32;
    let mut r = (248.0 - 32.0 * t - 8.0 * shade) as u8;
    let mut g = (244.0 - 35.0 * t - 11.0 * shade) as u8;
    let mut b = (234.0 - 42.0 * t - 14.0 * shade) as u8;

    if !small && x < 172 {
        r = (58.0 - 30.0 * shade) as u8;
        g = (50.0 - 28.0 * shade) as u8;
        b = (42.0 - 24.0 * shade) as u8;
    }

    let (ox, oy, size) = if small { (420, 10, 38) } else { (54, 44, 74) };
    if x >= ox && x < ox + size && y >= oy && y < oy + size {
        let lx = x - ox;
        let ly = y - oy;
        let edge = lx < 3 || ly < 3 || lx > size - 4 || ly > size - 4;
        let eye = (ly > size / 2 - 4 && ly < size / 2 + 4) && ((lx > size / 3 - 4 && lx < size / 3 + 4) || (lx > size * 2 / 3 - 4 && lx < size * 2 / 3 + 4));
        let cursor = ly > size * 2 / 3 && ly < size * 2 / 3 + 4 && lx > size / 2 - 12 && lx < size / 2 + 12;
        if edge || eye || cursor {
            return (245, 239, 222);
        }
        return (35, 30, 25);
    }

    (r, g, b)
}

fn write_if_missing(path: &Path, bytes: &[u8]) {
    if path.exists() {
        return;
    }
    fs::write(path, bytes).unwrap_or_else(|error| panic!("failed to write {}: {error}", path.display()));
}
