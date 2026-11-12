//! Needs GStreamer on PATH and `fixtures/bg30.mp4` (tools/make-fixture.sh).
#![allow(clippy::expect_used)]
use std::path::Path;

fn fixture() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/bg30.mp4")
}

#[test]
fn poster_scales_and_keeps_aspect() {
    if !fixture().is_file() {
        return;
    }
    let p = artosia_video::poster(&fixture(), 320).expect("poster");
    assert_eq!((p.width, p.height), (320, 180));
    assert_eq!(p.rgba.len(), 320 * 180 * 4);
    assert!(p.rgba.chunks(4).all(|px| px[3] == 255));
    assert!(p.rgba.iter().any(|&b| b != 0));
}

#[test]
fn missing_file_is_a_clean_error() {
    let err = artosia_video::poster(Path::new("does-not-exist.mp4"), 64).err().expect("error");
    assert!(matches!(err, artosia_video::VideoError::NotFound(_)));
    let err = artosia_video::VideoLoop::start(
        Path::new("does-not-exist.mp4"),
        rune::NativeSurface::new(),
        artosia_video::VideoOptions::default(),
    )
    .err()
    .expect("error");
    assert!(err.to_string().contains("not found"));
}

/// The projector's background as the output process runs it, with no window at all: the video
/// is decoded, comes round to its start twice, and names the decoder that did the work.
/// `cargo test -p artosia-video a_video_loops -- --nocapture` prints that name, which is how
/// a system's hardware decoding is checked where no window may be opened.
#[test]
fn a_video_loops_twice_with_no_window_and_names_its_decoder() {
    use std::time::{Duration, Instant};
    if !fixture().is_file() {
        return;
    }
    let options = artosia_video::VideoOptions { discard_frames: true, ..Default::default() };
    let video = artosia_video::VideoLoop::start(&fixture(), rune::NativeSurface::new(), options).expect("the video starts");
    let began = Instant::now();
    while video.loops() < 2 {
        assert!(video.error().is_none(), "{:?}", video.error());
        assert!(began.elapsed() < Duration::from_secs(120), "after two minutes: {} loops, {} frames", video.loops(), video.frames());
        std::thread::sleep(Duration::from_millis(250));
    }
    let decoder = video.decoder().unwrap_or_default();
    println!("decoder {decoder}, fallback {:?}, {} frames and {} loops in {:.1} s", video.fallback(), video.frames(), video.loops(), began.elapsed().as_secs_f64());
    assert!(!decoder.is_empty(), "no decoder was named");
    assert!(video.frames() > 100, "{} frames", video.frames());
}

/// A camera on the network: a made-up moving picture sent over SRT from this computer to
/// itself, and opened by its address. Skipped where GStreamer has no parts to send one.
#[test]
fn a_camera_on_the_network_gives_pictures_by_its_address() {
    use gst::prelude::*;
    use gstreamer as gst;
    gst::init().expect("gstreamer starts");
    if ["srtsink", "x264enc", "mpegtsmux"].iter().any(|name| gst::ElementFactory::find(name).is_none()) {
        eprintln!("skipped: this GStreamer cannot send a test stream");
        return;
    }
    let sender = gst::parse::launch(
        "videotestsrc is-live=true ! video/x-raw,width=320,height=180,framerate=25/1 ! x264enc tune=zerolatency speed-preset=ultrafast key-int-max=25 ! mpegtsmux ! srtsink uri=srt://127.0.0.1:47963?mode=listener wait-for-connection=false",
    )
    .expect("the sender is built");
    sender.set_state(gst::State::Playing).expect("the sender plays");
    let camera = artosia_video::CameraLoop::open_address("srt://127.0.0.1:47963", &rune::NativeSurface::new()).expect("the camera at the address");
    assert!(camera.frames() > 0);
    assert!(artosia_video::is_address(camera.name()));
    drop(camera);
    let _ = sender.set_state(gst::State::Null);
}

#[test]
fn an_address_nothing_answers_is_an_error_after_a_wait_with_an_end() {
    let began = std::time::Instant::now();
    let error = artosia_video::CameraLoop::open_address("srt://127.0.0.1:47964", &rune::NativeSurface::new()).err().expect("an error");
    assert!(began.elapsed() < std::time::Duration::from_secs(20), "it waited {:?}", began.elapsed());
    assert!(!error.to_string().is_empty());
}
