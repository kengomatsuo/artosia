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
