//! A Rune window with large white text over a gradient and a checkerboard, drawn through the
//! corner pin.
//!
//! `pin --corners 0.05,0.02,0.97,0.0,1.0,1.0,0.0,0.96 --brightness 0.1 --contrast 1.2
//! --saturation 0.8 [--filter bicubic] [--sweep] [--snap out.png --size 1920x1080 --scale 1]`
//! `--redraw` repaints 30 times a second. `--sweep` moves the corners from a second thread, to show a live setting and to measure it.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use artosia_warp::{Filter, Warp, WarpCompositor, WarpHandle};
use artosia_slides::look::rgb;
use rune::{App, AppSpec, Color, Cx, El, TextStyle, Tx, Waker, col, el, text};

struct Pin {
    seen: u64,
}

fn handle() -> &'static WarpHandle {
    static HANDLE: OnceLock<WarpHandle> = OnceLock::new();
    HANDLE.get_or_init(WarpHandle::default)
}

fn flag(name: &str) -> Option<String> {
    let args = rune::extra_args();
    args.iter().position(|a| a == name).and_then(|at| args.get(at + 1)).cloned()
}

fn has(name: &str) -> bool {
    rune::extra_args().iter().any(|a| a == name)
}

fn number(name: &str, default: f32) -> f32 {
    flag(name).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn warp_from_args() -> Warp {
    let mut warp = Warp { brightness: number("--brightness", 0.0), contrast: number("--contrast", 1.0), saturation: number("--saturation", 1.0), ..Warp::default() };
    if let Some(list) = flag("--corners") {
        let v: Vec<f32> = list.split(',').filter_map(|n| n.trim().parse().ok()).collect();
        if v.len() == 8 {
            warp.corners = [[v[0], v[1]], [v[2], v[3]], [v[4], v[5]], [v[6], v[7]]];
        } else {
            eprintln!("--corners needs 8 numbers: x,y for top left, top right, bottom right, bottom left");
        }
    }
    warp
}

fn setup(_: &mut Pin, waker: Waker) {
    handle().add_waker(waker);
}

fn poll(s: &mut Pin, _clock: f64, tx: &mut Tx) -> bool {
    // `--redraw` repaints at 30 Hz whatever the setting, to measure the cost of a frame.
    if has("--redraw") {
        tx.wake_after(std::time::Duration::from_millis(33));
        return true;
    }
    let version = handle().version();
    let changed = version != s.seen;
    s.seen = version;
    changed
}

fn view(_: &Pin, cx: &mut Cx<Pin>) -> El {
    let [w, h] = cx.layout_size("screen").unwrap_or([1920.0, 1080.0]);
    let (w, h) = (w.max(16.0), h.max(9.0));
    let mut root = el().key("screen").w_full().h_full().fill(Color::BLACK);
    root = root.child(el().key("bg").absolute(0.0, 0.0).size(w, h).gradient(rgb([15, 26, 76]), rgb([5, 8, 20])));
    // A checkerboard in the lower half, and thin frame lines, to judge the bend and the edges.
    let (cols, rows) = (16usize, 5usize);
    let (cw, ch) = (w / cols as f32, h * 0.3 / rows as f32);
    for r in 0..rows {
        for c in 0..cols {
            if (r + c) % 2 == 0 {
                let shade = if (c / 2) % 2 == 0 { rgb([230, 89, 51]) } else { rgb([242, 242, 242]) };
                root = root.child(el().key(format!("sq-{r}-{c}")).absolute(c as f32 * cw, h * 0.7 + r as f32 * ch).size(cw, ch).fill(shade));
            }
        }
    }
    let line = rgb([51, 230, 128]);
    root = root
        .child(el().key("frame-top").absolute(0.0, 0.0).size(w, 4.0).fill(line))
        .child(el().key("frame-bottom").absolute(0.0, h - 4.0).size(w, 4.0).fill(line))
        .child(el().key("frame-left").absolute(0.0, 0.0).size(4.0, h).fill(line))
        .child(el().key("frame-right").absolute(w - 4.0, 0.0).size(4.0, h).fill(line));
    let u = h / 1080.0;
    let body = TextStyle::new((120.0 * u).round(), 700.0);
    let small = TextStyle::new((44.0 * u).round(), 400.0);
    root.child(
        col()
            .key("words")
            .absolute(0.0, h * 0.12)
            .size(w, h * 0.5)
            .center()
            .gap((24.0 * u).round())
            .child(text("Amazing grace").font(body).color(Color::WHITE))
            .child(text("how sweet the sound that saved a wretch like me").font(small).color(Color::WHITE)),
    )
}

fn main() {
    let warp = warp_from_args();
    handle().set(warp);
    if has("--sweep") {
        let ticks = Arc::new(AtomicU64::new(0));
        std::thread::spawn(move || {
            loop {
                let t = ticks.fetch_add(1, Ordering::Relaxed) as f32 * 0.033;
                let s = (t * 1.5).sin() * 0.06;
                let mut w = warp;
                w.corners = [[0.06 + s, 0.03], [0.94 - s, 0.03 + s], [0.97, 0.97 - s], [0.03, 0.96]];
                handle().set(w);
                std::thread::sleep(std::time::Duration::from_millis(33));
            }
        });
    }
    let filter = if flag("--filter").as_deref() == Some("bicubic") { Filter::Bicubic } else { Filter::Bilinear };
    let spec = AppSpec {
        setup: Some(setup),
        poll: Some(poll),
        ..AppSpec::new("pin", (960.0, 540.0), || Pin { seen: 0 }, view)
    };
    App::new(spec).compositor(WarpCompositor::new(handle().clone()).with_filter(filter)).run();
}
