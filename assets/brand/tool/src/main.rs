//! Rasterises the Artosia brand SVGs. Run from the repo root:
//! cargo run --release --target-dir target/agent-logo --manifest-path assets/brand/tool/Cargo.toml
//! Optional argument: the brand folder (default `assets/brand`).
use resvg::{tiny_skia, usvg};
use std::{fs, path::Path};
use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

const SIZES: [u32; 8] = [16, 24, 32, 48, 64, 128, 256, 512];
const ICO_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];
const DARK: (u8, u8, u8) = (0x12, 0x14, 0x18);
const BLUE: &str = "#5599f3";
const BLUE_DEEP: &str = "#276ed2";

fn options() -> usvg::Options<'static> {
    let mut o = usvg::Options::default();
    o.fontdb_mut().load_system_fonts();
    o
}

fn render(svg: &str, size: u32, opt: &usvg::Options) -> Pixmap {
    let tree = usvg::Tree::from_str(svg, opt).expect("svg parses");
    let mut pm = Pixmap::new(size, size).unwrap();
    let s = tree.size();
    let t = Transform::from_scale(size as f32 / s.width(), size as f32 / s.height());
    resvg::render(&tree, t, &mut pm.as_mut());
    pm
}

fn solid(w: u32, h: u32, c: (u8, u8, u8)) -> Pixmap {
    let mut pm = Pixmap::new(w, h).unwrap();
    pm.fill(Color::from_rgba8(c.0, c.1, c.2, 255));
    pm
}

fn put(dst: &mut Pixmap, src: &Pixmap, x: i32, y: i32) {
    dst.draw_pixmap(x, y, src.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
}

/// Nearest-neighbour enlargement, so the owner can see the 16 px pixels.
fn zoom(src: &Pixmap, k: u32) -> Pixmap {
    let mut out = Pixmap::new(src.width() * k, src.height() * k).unwrap();
    let ow = out.width();
    for y in 0..out.height() {
        for x in 0..out.width() {
            let p = src.pixels()[((y / k) * src.width() + x / k) as usize];
            out.pixels_mut()[(y * ow + x) as usize] = p;
        }
    }
    out
}

fn label(text: &str, w: u32, h: u32, ink: &str, opt: &usvg::Options) -> Pixmap {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}"><text x="0" y="{y}" font-family="Segoe UI, Arial, sans-serif" font-size="15" font-weight="600" fill="{ink}">{text}</text></svg>"#,
        y = h - 6
    );
    let tree = usvg::Tree::from_str(&svg, opt).unwrap();
    let mut pm = Pixmap::new(w, h).unwrap();
    resvg::render(&tree, Transform::identity(), &mut pm.as_mut());
    pm
}

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "assets/brand".into());
    let dir = Path::new(&dir);
    let opt = options();
    let read = |p: &str| fs::read_to_string(dir.join(p)).unwrap_or_else(|e| panic!("{p}: {e}"));

    // App icon PNGs and ICO.
    let icon = read("artosia-icon.svg");
    let mut entries = ico::IconDir::new(ico::ResourceType::Icon);
    for s in SIZES {
        let pm = render(&icon, s, &opt);
        pm.save_png(dir.join(format!("artosia-icon-{s}.png"))).unwrap();
        if ICO_SIZES.contains(&s) {
            let mut rgba = Vec::with_capacity((s * s * 4) as usize);
            for p in pm.pixels() {
                let c = p.demultiply();
                rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
            }
            let img = ico::IconImage::from_rgba_data(s, s, rgba);
            entries.add_entry(ico::IconDirEntry::encode(&img).unwrap());
        }
    }
    entries.write(fs::File::create(dir.join("artosia.ico")).unwrap()).unwrap();

    // Contact sheet: rows are dark and white, columns are the three directions.
    let dirs = [
        ("A  Loaf (chosen)", read("artosia-mark.svg")),
        ("B  Broken loaf", read("explorations/direction-b.svg")),
        ("C  Lines of text", read("explorations/direction-c.svg")),
    ];
    let (cell_w, row_h, head) = (400u32, 330u32, 40u32);
    let (w, h) = (cell_w * 3, head + row_h * 2);
    let mut sheet = solid(w, h, DARK);
    put(&mut sheet, &solid(w, row_h, (255, 255, 255)), 0, (head + row_h) as i32);
    for (i, (name, svg)) in dirs.iter().enumerate() {
        let x0 = (i as u32 * cell_w) as i32;
        put(&mut sheet, &label(name, 360, 28, "#f3f5f8", &opt), x0 + 20, 8);
        for (row, bg) in [DARK, (255, 255, 255)].iter().enumerate() {
            // The mark colour is the brand blue on dark and the deeper blue on white.
            let s = if row == 0 { svg.clone() } else { svg.replace(BLUE, BLUE_DEEP).replace("#9ec7fe", "#6aa4ee") };
            let y0 = (head + row as u32 * row_h) as i32;
            let _ = bg;
            let big = render(&s, 256, &opt);
            put(&mut sheet, &big, x0 + 16, y0 + 16);
            let m32 = render(&s, 32, &opt);
            let m16 = render(&s, 16, &opt);
            put(&mut sheet, &m32, x0 + 16, y0 + 284);
            put(&mut sheet, &m16, x0 + 64, y0 + 292);
            put(&mut sheet, &zoom(&m16, 4), x0 + 100, y0 + 260);
        }
    }
    sheet.save_png(dir.join("explorations/sheet.png")).unwrap();
    if let Ok(out) = std::env::var("BRAND_PREVIEW") {
        // Throwaway check of the wordmark and the mono glyph on the dark background.
        let wm = read("artosia-wordmark.svg").replace("#14161a", "#f3f5f8");
        let tree = usvg::Tree::from_str(&wm, &opt).unwrap();
        let mut pm = solid(792, 240, DARK);
        resvg::render(&tree, Transform::from_scale(3.0, 3.0), &mut pm.as_mut());
        pm.save_png(Path::new(&out).join("wordmark.png")).unwrap();
        let mono = read("artosia-mark-mono.svg").replace("currentColor", "#f3f5f8");
        let mut row = solid(16 * 8 * 2 + 48, 16 * 8 + 32, DARK);
        put(&mut row, &zoom(&render(&mono, 16, &opt), 8), 16, 16);
        put(&mut row, &render(&mono, 128, &opt), 16 * 8 + 32, 16);
        row.save_png(Path::new(&out).join("mono.png")).unwrap();
    }
    println!("done");
}
