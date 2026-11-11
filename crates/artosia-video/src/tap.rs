//! A look at the frames now and then, for the small pictures in the operator window.
//!
//! The program that owns the loop registers one tap with [`set_tap`]; while [`want`] is on,
//! the appsink callback of a file or a camera hands it a borrowed view of the newest NV12
//! frame at most once per interval. Between those moments the cost is one atomic load.

use crate::frames::nv12_view;
use gstreamer as gst;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// An NV12 frame as the decoder holds it, borrowed for the length of the call.
#[derive(Clone, Copy, Debug)]
pub struct Nv12View<'a> {
    pub width: u32,
    pub height: u32,
    pub luma: &'a [u8],
    pub luma_stride: usize,
    /// The interleaved U and V plane, half as wide and tall.
    pub chroma: &'a [u8],
    pub chroma_stride: usize,
    pub full_range: bool,
    pub bt601: bool,
}

type Tap = Box<dyn Fn(Nv12View<'_>) + Send + Sync>;

struct Registered {
    tap: Tap,
    every: Duration,
}

static REGISTERED: OnceLock<Registered> = OnceLock::new();
static WANTED: AtomicBool = AtomicBool::new(false);
/// Milliseconds after `epoch()` before which no frame is offered.
static NEXT_AT: AtomicU64 = AtomicU64::new(0);

fn epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

/// Registers the tap, once for the process; `every` is the least time between two offers.
pub fn set_tap(every: Duration, tap: impl Fn(Nv12View<'_>) + Send + Sync + 'static) {
    let _ = epoch();
    let _ = REGISTERED.set(Registered { tap: Box::new(tap), every });
}

/// Turns the offers on or off. Off, a frame costs one atomic load.
pub fn want(on: bool) {
    WANTED.store(on, Ordering::Release);
}

/// Called by the appsink callback with every sample; offers it when one is due.
pub(crate) fn offer(sample: &gst::Sample) {
    if !WANTED.load(Ordering::Acquire) {
        return;
    }
    let Some(registered) = REGISTERED.get() else { return };
    let now = epoch().elapsed().as_millis() as u64;
    if now < NEXT_AT.load(Ordering::Relaxed) {
        return;
    }
    NEXT_AT.store(now + registered.every.as_millis() as u64, Ordering::Relaxed);
    let _ = nv12_view(sample, |view| (registered.tap)(view));
}
