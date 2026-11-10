//! Corner pin (keystone) and colour correction for the whole projector picture, text included.
//!
//! Rune has no perspective transform on elements, so this is a Rune `Compositor`: it has Rune
//! draw the finished UI into an offscreen texture, then draws that texture to the window
//! through a homography and a brightness, contrast and saturation matrix. With the identity
//! setting it does nothing but Rune's own draw.
//!
//! ```ignore
//! let handle = WarpHandle::default();
//! let app = App::new(spec).compositor(WarpCompositor::new(handle.clone()));
//! handle.set(warp); // from any thread
//! ```

mod compositor;
mod handle;
pub mod homography;

pub use artosia_slides::frame::Warp;
pub use compositor::{Filter, WarpCompositor};
pub use handle::WarpHandle;
pub use homography::{Degenerate, homography, try_homography};
