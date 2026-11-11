//! The small picture of the projector's moving background (a looping video or a camera) that
//! the output sends the operator a few times a second, so the two previews in the operator
//! window draw the words over what the projector draws them over.
//!
//! The output shrinks each sample straight from the decoder's NV12 planes (no GStreamer call,
//! so this runs and is tested anywhere), packs the RGB bytes as base64 inside one JSON line
//! of `FromOutput::Preview`, and the operator unpacks them into a Rune `Pixels`.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// How wide the picture sent to the operator is, in pixels.
pub const WIDTH: u32 = 256;

/// How many pictures a second the output sends while a video or camera plays.
pub const PER_SECOND: u32 = 4;

/// One decoded frame as the decoder holds it: a luma plane and an interleaved U,V plane at
/// half the width and height. A stride is the bytes from one row to the next.
#[derive(Clone, Copy, Debug)]
pub struct Nv12<'a> {
    pub width: u32,
    pub height: u32,
    pub luma: &'a [u8],
    pub luma_stride: usize,
    pub chroma: &'a [u8],
    pub chroma_stride: usize,
    /// Luma and chroma use the whole 0 to 255 range instead of 16 to 235 and 16 to 240.
    pub full_range: bool,
    /// BT.601 colours (small, standard-definition frames); false is BT.709.
    pub bt601: bool,
}

/// A shrunk frame: `rgb` is `width * height * 3` bytes, rows top first, sRGB.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shrunk {
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

fn clamp8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// The colour of the source pixel at `(x, y)`.
fn rgb_at(f: &Nv12, x: usize, y: usize) -> [f32; 3] {
    let luma = f32::from(f.luma[y * f.luma_stride + x]);
    let at = (y / 2) * f.chroma_stride + (x / 2) * 2;
    let (u, v) = (f32::from(f.chroma[at]), f32::from(f.chroma[at + 1]));
    let (luma, cb, cr) = if f.full_range { (luma, u - 128.0, v - 128.0) } else { ((luma - 16.0) * (255.0 / 219.0), (u - 128.0) * (255.0 / 224.0), (v - 128.0) * (255.0 / 224.0)) };
    let (kr, kb) = if f.bt601 { (0.299, 0.114) } else { (0.2126, 0.0722) };
    let r = luma + 2.0 * (1.0 - kr) * cr;
    let b = luma + 2.0 * (1.0 - kb) * cb;
    let g = (luma - kr * r - kb * b) / (1.0 - kr - kb);
    [r, g, b]
}

/// Shrinks `frame` to `out_width` pixels wide, keeping its shape: each output pixel is the
/// average of four samples taken inside the block of source pixels it stands for. None when
/// the planes are smaller than the sizes say, or the frame is empty.
pub fn shrink(frame: &Nv12, out_width: u32) -> Option<Shrunk> {
    let (w, h) = (frame.width as usize, frame.height as usize);
    if w == 0 || h == 0 || out_width == 0 {
        return None;
    }
    let ow = (out_width as usize).min(w);
    let oh = ((h * ow + w / 2) / w).max(1);
    let chroma_rows = h.div_ceil(2);
    let chroma_cols = w.div_ceil(2) * 2;
    if frame.luma.len() < (h - 1) * frame.luma_stride + w || frame.chroma.len() < (chroma_rows - 1) * frame.chroma_stride + chroma_cols {
        return None;
    }
    let mut rgb = Vec::with_capacity(ow * oh * 3);
    for oy in 0..oh {
        // The block's rows, then the two sample rows a quarter and three quarters down it.
        let (y0, y1) = (oy * h / oh, ((oy + 1) * h / oh).max(oy * h / oh + 1));
        let ys = [y0 + (y1 - y0) / 4, (y0 + (y1 - y0) * 3 / 4).min(y1 - 1)];
        for ox in 0..ow {
            let (x0, x1) = (ox * w / ow, ((ox + 1) * w / ow).max(ox * w / ow + 1));
            let xs = [x0 + (x1 - x0) / 4, (x0 + (x1 - x0) * 3 / 4).min(x1 - 1)];
            let mut sum = [0.0f32; 3];
            for &y in &ys {
                for &x in &xs {
                    let c = rgb_at(frame, x.min(w - 1), y.min(h - 1));
                    for k in 0..3 {
                        sum[k] += c[k];
                    }
                }
            }
            rgb.extend(sum.map(|v| clamp8(v / 4.0)));
        }
    }
    Some(Shrunk { width: ow as u32, height: oh as u32, rgb })
}

/// The RGB bytes as base64 text, for a JSON line.
pub fn pack(shrunk: &Shrunk) -> String {
    STANDARD.encode(&shrunk.rgb)
}

/// The text from `pack` as `width * height` straight-alpha RGBA pixels, which is what Rune's
/// `Pixels::set` takes. None when the text is not base64 or does not fit the size.
pub fn unpack(width: u32, height: u32, text: &str) -> Option<Vec<u8>> {
    let pixels = (width as usize).checked_mul(height as usize)?;
    if pixels == 0 || pixels > 1 << 20 {
        return None;
    }
    let rgb = STANDARD.decode(text).ok()?;
    if rgb.len() != pixels * 3 {
        return None;
    }
    let mut rgba = Vec::with_capacity(pixels * 4);
    for p in rgb.chunks_exact(3) {
        rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
    }
    Some(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up frame: left half red, right half blue, top rows darker, as BT.709 limited
    /// range NV12 (the planes are filled in by hand from the colour equations' inverse).
    fn frame(w: usize, h: usize, stride_extra: usize) -> (Vec<u8>, Vec<u8>, usize, usize) {
        let (ls, cs) = (w + stride_extra, w + stride_extra);
        let mut luma = vec![0u8; ls * h];
        let mut chroma = vec![0u8; cs * h / 2];
        // Pure red is Y 63, U 102, V 240 and pure blue is Y 32, U 240, V 118 in BT.709 limited.
        for y in 0..h {
            for x in 0..w {
                luma[y * ls + x] = if x < w / 2 { 63 } else { 32 };
            }
        }
        for y in 0..h / 2 {
            for x in 0..w / 2 {
                let (u, v) = if x < w / 4 { (102, 240) } else { (240, 118) };
                chroma[y * cs + x * 2] = u;
                chroma[y * cs + x * 2 + 1] = v;
            }
        }
        (luma, chroma, ls, cs)
    }

    fn nv12<'a>(w: usize, h: usize, luma: &'a [u8], chroma: &'a [u8], ls: usize, cs: usize) -> Nv12<'a> {
        Nv12 { width: w as u32, height: h as u32, luma, luma_stride: ls, chroma, chroma_stride: cs, full_range: false, bt601: false }
    }

    #[test]
    fn shrinking_keeps_the_shape_and_the_colours() {
        let (w, h) = (640, 360);
        let (luma, chroma, ls, cs) = frame(w, h, 64);
        let small = shrink(&nv12(w, h, &luma, &chroma, ls, cs), 64).expect("a frame");
        assert_eq!((small.width, small.height), (64, 36), "16:9 stays 16:9");
        assert_eq!(small.rgb.len(), 64 * 36 * 3);
        let at = |x: usize, y: usize| <[u8; 3]>::try_from(&small.rgb[(y * 64 + x) * 3..(y * 64 + x) * 3 + 3]).expect("three bytes");
        let (red, blue) = (at(8, 10), at(56, 10));
        let near = |got: [u8; 3], want: [u8; 3]| got.iter().zip(want).all(|(a, b)| a.abs_diff(b) <= 6);
        assert!(near(red, [255, 0, 0]), "left is red, got {red:?}");
        assert!(near(blue, [0, 0, 255]), "right is blue, got {blue:?}");
    }

    #[test]
    fn a_frame_smaller_than_the_target_is_not_enlarged_and_odd_sizes_work() {
        let (luma, chroma, ls, cs) = frame(32, 18, 0);
        let small = shrink(&nv12(32, 18, &luma, &chroma, ls, cs), 256).expect("a frame");
        assert_eq!((small.width, small.height), (32, 18));
        let (luma, chroma, ls, cs) = frame(34, 20, 0);
        assert!(shrink(&nv12(33, 19, &luma, &chroma, ls, cs), 8).is_some());
    }

    #[test]
    fn planes_shorter_than_the_size_say_give_nothing() {
        let (luma, chroma, ls, cs) = frame(64, 36, 0);
        assert!(shrink(&nv12(64, 36, &luma[..100], &chroma, ls, cs), 16).is_none());
        assert!(shrink(&nv12(64, 36, &luma, &chroma[..10], ls, cs), 16).is_none());
        assert!(shrink(&nv12(0, 0, &luma, &chroma, ls, cs), 16).is_none());
    }

    #[test]
    fn packing_round_trips_and_refuses_a_wrong_size() {
        let shrunk = Shrunk { width: 3, height: 2, rgb: (0..18).collect() };
        let text = pack(&shrunk);
        let rgba = unpack(3, 2, &text).expect("fits");
        assert_eq!(rgba.len(), 24);
        assert_eq!(&rgba[..8], &[0, 1, 2, 255, 3, 4, 5, 255]);
        assert!(unpack(3, 3, &text).is_none(), "wrong size");
        assert!(unpack(3, 2, "not base64!").is_none());
        assert!(unpack(0, 0, "").is_none());
    }

    #[test]
    fn the_message_survives_a_json_line() {
        use crate::frame::{FromOutput, decode, encode};
        let shrunk = Shrunk { width: 2, height: 1, rgb: vec![9, 8, 7, 6, 5, 4] };
        let line = encode(&FromOutput::Preview { width: 2, height: 1, rgb: pack(&shrunk) });
        match decode::<FromOutput>(&line) {
            Some(FromOutput::Preview { width, height, rgb }) => assert_eq!(unpack(width, height, &rgb), Some(vec![9, 8, 7, 255, 6, 5, 4, 255])),
            other => panic!("came back as {other:?}"),
        }
        assert_eq!(decode::<FromOutput>(&encode(&FromOutput::PreviewEnd)), Some(FromOutput::PreviewEnd));
    }
}
