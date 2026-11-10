//! `cargo run -p artosia-video --example loop -- [file] [--seconds N] [--software]`
//! Loops a video in a Rune window with large text over it and prints playback stats.

use artosia_video::{VideoLoop, VideoOptions};
use rune::{AppSpec, Color, Cx, El, NativeSurface, SurfaceFit, TextStyle, Tx, Waker, col, el, text};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct Demo {
    surface: NativeSurface,
    video: Arc<Mutex<Option<VideoLoop>>>,
    seconds: Option<f64>,
    last_print: f64,
}

fn arg_value(flag: &str) -> Option<String> {
    let args = rune::extra_args();
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

fn state() -> Demo {
    Demo {
        surface: NativeSurface::new(),
        video: Arc::new(Mutex::new(None)),
        seconds: arg_value("--seconds").and_then(|s| s.parse().ok()),
        last_print: -1.0,
    }
}

fn setup(s: &mut Demo, waker: Waker) {
    s.surface.set_waker(waker);
    let args = rune::extra_args();
    let file = args
        .iter()
        .enumerate()
        .find(|(i, a)| !a.starts_with("--") && (*i == 0 || args[*i - 1] != "--seconds"))
        .map_or_else(|| PathBuf::from("fixtures/bg30.mp4"), |(_, a)| PathBuf::from(a));
    let options = VideoOptions { prefer_hardware: !args.iter().any(|a| a == "--software"), ..VideoOptions::default() };
    let (surface, slot) = (s.surface.clone(), s.video.clone());
    // start() blocks until the first frame, so it runs off the UI thread.
    std::thread::spawn(move || match VideoLoop::start(&file, surface, options) {
        Ok(v) => {
            if let Ok(mut g) = slot.lock() {
                *g = Some(v);
            }
        }
        Err(e) => eprintln!("video start failed: {e}"),
    });
}

fn poll(s: &mut Demo, t: f64, tx: &mut Tx) -> bool {
    if t - s.last_print >= 1.0 {
        s.last_print = t;
        if let Ok(g) = s.video.lock()
            && let Some(v) = g.as_ref()
        {
            let pos = v.position().map_or(-1.0, |p| p.as_secs_f64());
            println!("pos={pos:.2} loops={} decoder={}", v.loops(), v.decoder().unwrap_or_default());
            if let Some(e) = v.error() {
                eprintln!("error: {e}");
            }
        }
    }
    if s.seconds.is_some_and(|limit| t >= limit) {
        tx.quit();
    }
    tx.wake_after(std::time::Duration::from_millis(250));
    false
}

fn view(s: &Demo, _: &mut Cx<Demo>) -> El {
    let big = TextStyle::new(72.0, 600.0);
    col()
        .w_full()
        .h_full()
        .fill(Color::BLACK)
        .child(el().key("video").w_full().h_full().native_surface(&s.surface, SurfaceFit::Fill))
        .child(
            col()
                .key("lyrics")
                .absolute(0.0, 0.0)
                .w_full()
                .h_full()
                .center()
                .child(text("Amazing grace, how sweet the sound").font(big).color(Color::WHITE)),
        )
}

fn main() {
    rune::run(AppSpec { setup: Some(setup), poll: Some(poll), ..AppSpec::new("artosia-video loop", (1280.0, 720.0), state, view) });
}
