//! Rasterises the Artosia brand SVGs into every icon file the app and its installers use.
//! Run from the repo root:
//! cargo run --release --target-dir target/agent-logo --manifest-path assets/brand/tool/Cargo.toml
//! Optional argument: the brand folder (default `assets/brand`).
//!
//! Each icon size draws `artosia-icon-<n>.svg` when that file exists, else `artosia-icon.svg`.
use resvg::{tiny_skia, usvg};
use std::{fs, path::Path};
use tiny_skia::{Pixmap, Transform};

/// `artosia-icon-<n>.png`; the app loads the 256 as its window icon.
const PNG_SIZES: [u32; 8] = [16, 24, 32, 48, 64, 128, 256, 512];
/// Every size Windows 11 asks for at its scale factors (Microsoft Learn, app-icon-construction).
const ICO_SIZES: [u32; 14] = [16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256];
/// freedesktop hicolor theme sizes.
const LINUX_SIZES: [u32; 9] = [16, 22, 24, 32, 48, 64, 128, 256, 512];

fn render(svg: &str, size: u32) -> Pixmap {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).expect("svg parses");
    let (w, h) = (tree.size().width(), tree.size().height());
    let scale = size as f32 / w.max(h);
    let (dx, dy) = ((size as f32 - w * scale) / 2.0, (size as f32 - h * scale) / 2.0);
    let mut pm = Pixmap::new(size, size).unwrap();
    resvg::render(&tree, Transform::from_scale(scale, scale).post_translate(dx, dy), &mut pm.as_mut());
    pm
}

fn rgba(pm: &Pixmap) -> Vec<u8> {
    pm.pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "assets/brand".into());
    let dir = Path::new(&dir);
    let read = |p: &str| fs::read_to_string(dir.join(p)).unwrap_or_else(|e| panic!("{p}: {e}"));
    let master = read("artosia-icon.svg");
    let icon = |size: u32| {
        let own = dir.join(format!("artosia-icon-{size}.svg"));
        let svg = fs::read_to_string(&own).unwrap_or_else(|_| master.clone());
        render(&svg, size)
    };

    for s in PNG_SIZES {
        icon(s).save_png(dir.join(format!("artosia-icon-{s}.png"))).unwrap();
    }

    // Small frames as BMP for older readers, the 256 as PNG, the usual Windows layout.
    let mut ico = ico::IconDir::new(ico::ResourceType::Icon);
    for s in ICO_SIZES {
        let image = ico::IconImage::from_rgba_data(s, s, rgba(&icon(s)));
        let entry = if s >= 256 { ico::IconDirEntry::encode_as_png(&image) } else { ico::IconDirEntry::encode_as_bmp(&image) };
        ico.add_entry(entry.unwrap());
    }
    ico.write(fs::File::create(dir.join("artosia.ico")).unwrap()).unwrap();

    let linux = dir.join("icons/linux/hicolor");
    for s in LINUX_SIZES {
        let apps = linux.join(format!("{s}x{s}/apps"));
        fs::create_dir_all(&apps).unwrap();
        icon(s).save_png(apps.join("artosia.png")).unwrap();
    }
    fs::create_dir_all(linux.join("scalable/apps")).unwrap();
    fs::write(linux.join("scalable/apps/artosia.svg"), &master).unwrap();

    // macOS draws its own plated art: `icons/macos/artosia-macos-<n>.svg` where it exists, else
    // `icons/macos/artosia-macos.svg`.
    use icns::IconType::*;
    let mac_master = read("icons/macos/artosia-macos.svg");
    let mac = |px: u32| {
        let own = dir.join(format!("icons/macos/artosia-macos-{px}.svg"));
        render(&fs::read_to_string(own).unwrap_or_else(|_| mac_master.clone()), px)
    };
    let mut family = icns::IconFamily::new();
    for (px, kind) in [
        (16, RGBA32_16x16),
        (32, RGBA32_16x16_2x),
        (32, RGBA32_32x32),
        (64, RGBA32_32x32_2x),
        (128, RGBA32_128x128),
        (256, RGBA32_128x128_2x),
        (256, RGBA32_256x256),
        (512, RGBA32_256x256_2x),
        (512, RGBA32_512x512),
        (1024, RGBA32_512x512_2x),
    ] {
        let image = icns::Image::from_data(icns::PixelFormat::RGBA, px, px, rgba(&mac(px))).unwrap();
        family.add_icon_with_type(&image, kind).unwrap();
    }
    family.write(fs::File::create(dir.join("icons/artosia.icns")).unwrap()).unwrap();

    // `png/<variant>/<name>-<width>.png`: the full-colour, black and white SVGs at set widths.
    for (name, widths) in [
        ("artosia-mark", &[16, 32, 64, 128, 256, 512, 1024][..]),
        ("artosia-wordmark", &[256, 512, 1024, 2048][..]),
        ("artosia-lockup-horizontal", &[256, 512, 1024, 2048][..]),
        ("artosia-lockup-stacked", &[256, 512, 1024, 2048][..]),
    ] {
        for (variant, suffix) in [("full-color", ""), ("black", "-black"), ("white", "-white")] {
            let svg = read(&format!("{name}{suffix}.svg"));
            let out = dir.join("png").join(variant);
            fs::create_dir_all(&out).unwrap();
            for &w in widths {
                width(&svg, w).save_png(out.join(format!("{name}-{w}.png"))).unwrap();
            }
        }
    }
}

/// `svg` scaled to `w` pixels wide, its height in proportion.
fn width(svg: &str, w: u32) -> Pixmap {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).expect("svg parses");
    let scale = w as f32 / tree.size().width();
    let h = (tree.size().height() * scale).ceil() as u32;
    let mut pm = Pixmap::new(w, h).unwrap();
    resvg::render(&tree, Transform::from_scale(scale, scale), &mut pm.as_mut());
    pm
}
