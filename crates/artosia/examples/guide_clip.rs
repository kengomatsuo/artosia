//! Makes one guide clip: frames recorded headless from the real program (`--script ... --out`),
//! with the pointer drawn along the script's path and a ring where it presses, cropped to the
//! part that moves, scaled, and written as one animated PNG that Rune plays by itself.
//!
//! cargo run -p artosia --example guide_clip -- --frames <dir> --out <clip.png> --crop x,y,w,h
//!     --width 400 --start 0.2 --step 0.1 --path "0.3:700,500;0.9:1232,126" --press 1.4 [--pointer 36]
//!
//! `tools/record-guide.py` records every step and calls this. Nothing here ships.

use std::path::PathBuf;

use image::{Rgba, RgbaImage, imageops};

fn arg(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn numbers(text: &str) -> Vec<f32> {
    text.split(',').filter_map(|n| n.trim().parse().ok()).collect()
}

/// Where the pointer is at `t`: on the path's straight pieces, eased so it slows into the target.
fn pointer_at(path: &[(f32, f32, f32)], t: f32) -> Option<(f32, f32)> {
    let first = path.first()?;
    if t <= first.0 {
        return Some((first.1, first.2));
    }
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if t <= b.0 {
            let u = ((t - a.0) / (b.0 - a.0)).clamp(0.0, 1.0);
            let eased = 1.0 - (1.0 - u) * (1.0 - u) * (1.0 - u);
            return Some((a.1 + (b.1 - a.1) * eased, a.2 + (b.2 - a.2) * eased));
        }
    }
    path.last().map(|p| (p.1, p.2))
}

fn inside(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut odd = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            odd = !odd;
        }
        j = i;
    }
    odd
}

/// An arrow pointer with its tip at (x, y): white with a dark edge, as the system draws it.
fn draw_pointer(img: &mut RgbaImage, x: f32, y: f32, size: f32) {
    let shape = [(0.0, 0.0), (0.0, 16.0), (4.0, 12.5), (7.0, 19.0), (9.5, 18.0), (6.5, 11.5), (12.0, 11.5)];
    let scaled: Vec<(f32, f32)> = shape.iter().map(|(px, py)| (x + px * size / 19.0, y + py * size / 19.0)).collect();
    let (w, h) = (img.width() as i32, img.height() as i32);
    for py in (y as i32 - 2)..(y as i32 + size as i32 + 3) {
        for px in (x as i32 - 2)..(x as i32 + size as i32 + 3) {
            if px < 0 || py < 0 || px >= w || py >= h {
                continue;
            }
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let edge = [(-1.2, 0.0), (1.2, 0.0), (0.0, -1.2), (0.0, 1.2)].iter().any(|(dx, dy)| inside(&scaled, fx + dx, fy + dy));
            if inside(&scaled, fx, fy) {
                img.put_pixel(px as u32, py as u32, Rgba([255, 255, 255, 255]));
            } else if edge {
                img.put_pixel(px as u32, py as u32, Rgba([20, 20, 20, 255]));
            }
        }
    }
}

/// A ring that grows and fades where the pointer pressed.
fn draw_press(img: &mut RgbaImage, x: f32, y: f32, age: f32) {
    let radius = 8.0 + age * 60.0;
    let alpha = (1.0 - age / 0.5).clamp(0.0, 1.0);
    let (w, h) = (img.width() as i32, img.height() as i32);
    let reach = radius as i32 + 3;
    for py in (y as i32 - reach)..(y as i32 + reach) {
        for px in (x as i32 - reach)..(x as i32 + reach) {
            if px < 0 || py < 0 || px >= w || py >= h {
                continue;
            }
            let d = ((px as f32 - x).powi(2) + (py as f32 - y).powi(2)).sqrt();
            if (d - radius).abs() < 2.0 {
                let under = *img.get_pixel(px as u32, py as u32);
                let mix = |a: u8, b: u8| (a as f32 * (1.0 - alpha) + b as f32 * alpha) as u8;
                img.put_pixel(px as u32, py as u32, Rgba([mix(under[0], 77), mix(under[1], 163), mix(under[2], 255), 255]));
            }
        }
    }
}

/// The smallest rectangle holding every pixel that differs, as (x0, y0, x1, y1).
fn changed(a: &RgbaImage, b: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, pixel) in b.enumerate_pixels() {
        if a.get_pixel(x, y) != pixel {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + 1);
            y1 = y1.max(y + 1);
        }
    }
    (x1 > x0).then_some((x0, y0, x1, y1))
}

fn main() {
    let dir = PathBuf::from(arg("--frames").expect("--frames <dir>"));
    let out = PathBuf::from(arg("--out").expect("--out <clip.png>"));
    let crop = numbers(&arg("--crop").expect("--crop x,y,w,h"));
    let width: u32 = arg("--width").and_then(|w| w.parse().ok()).unwrap_or(400);
    let start: f32 = arg("--start").and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let step: f32 = arg("--step").and_then(|v| v.parse().ok()).unwrap_or(0.1);
    let press: Option<f32> = arg("--press").and_then(|v| v.parse().ok());
    let pointer: f32 = arg("--pointer").and_then(|v| v.parse().ok()).unwrap_or(36.0);
    let path: Vec<(f32, f32, f32)> = arg("--path")
        .unwrap_or_default()
        .split(';')
        .filter_map(|point| {
            let (t, xy) = point.split_once(':')?;
            let xy = numbers(xy);
            Some((t.parse().ok()?, *xy.first()?, *xy.get(1)?))
        })
        .collect();

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).expect("frames folder").filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "png")).collect();
    files.sort();
    let (cx, cy, cw, ch) = (crop[0] as u32, crop[1] as u32, crop[2] as u32, crop[3] as u32);
    let height = (width as f32 * ch as f32 / cw as f32).round() as u32;
    let mut frames: Vec<RgbaImage> = Vec::new();
    for (i, file) in files.iter().enumerate() {
        let mut img = image::open(file).expect("a frame").to_rgba8();
        let t = start + i as f32 * step;
        if let Some(at) = press.filter(|at| t >= *at && t < at + 0.5) {
            if let Some((x, y)) = pointer_at(&path, at) {
                draw_press(&mut img, x, y, t - at);
            }
        }
        if let Some((x, y)) = pointer_at(&path, t) {
            draw_pointer(&mut img, x, y, pointer);
        }
        let cut = imageops::crop_imm(&img, cx, cy, cw, ch).to_image();
        frames.push(imageops::resize(&cut, width, height, imageops::FilterType::Lanczos3));
    }
    // Identical frames become one frame shown longer, and the last one holds before the loop.
    let mut shown: Vec<(RgbaImage, u16)> = Vec::new();
    for frame in frames {
        match shown.last_mut() {
            Some((last, count)) if *last == frame => *count += 1,
            _ => shown.push((frame, 1)),
        }
    }
    if let Some((_, count)) = shown.last_mut() {
        *count += 8;
    }

    let file = std::fs::File::create(&out).expect("the clip file");
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Best);
    encoder.set_animated(shown.len() as u32, 0).expect("animated");
    let mut writer = encoder.write_header().expect("header");
    let mut previous: Option<&RgbaImage> = None;
    for (frame, count) in &shown {
        writer.set_frame_delay((step * 1000.0) as u16 * count, 1000).expect("delay");
        // After the first, a frame carries only the rectangle that changed since the one before.
        let (x0, y0, x1, y1) = match previous {
            None => (0, 0, width, height),
            Some(before) => changed(before, frame).unwrap_or((0, 0, 1, 1)),
        };
        writer.set_frame_position(0, 0).expect("position");
        writer.set_frame_dimension(x1 - x0, y1 - y0).expect("dimension");
        writer.set_frame_position(x0, y0).expect("position");
        let part = imageops::crop_imm(frame, x0, y0, x1 - x0, y1 - y0).to_image();
        writer.write_image_data(part.as_raw()).expect("a frame");
        previous = Some(frame);
    }
    writer.finish().expect("finish");
    println!("{} frames, {}x{}, {} bytes -> {}", shown.len(), width, height, std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0), out.display());
}
