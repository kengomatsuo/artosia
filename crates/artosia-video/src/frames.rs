//! Copying a GStreamer video frame into tightly packed planes.

use crate::VideoError;
use gstreamer as gst;
use gstreamer_video as gst_video;
use gst_video::VideoFrameExt;
use rune::native_surface::{ByteFrame, ColorInfo, Format, Matrix, Transfer};

/// Copies `rows` rows of `row_bytes` out of a plane that has `stride` bytes per row.
fn copy_plane(data: &[u8], stride: usize, row_bytes: usize, rows: usize) -> Result<Vec<u8>, VideoError> {
    let short = || VideoError::Frame("plane is smaller than its caps say".into());
    if rows == 0 || row_bytes == 0 {
        return Err(VideoError::Frame("empty plane".into()));
    }
    if stride == row_bytes {
        return data.get(..row_bytes * rows).map(<[u8]>::to_vec).ok_or_else(short);
    }
    let mut out = Vec::with_capacity(row_bytes * rows);
    for row in 0..rows {
        let start = row * stride;
        out.extend_from_slice(data.get(start..start + row_bytes).ok_or_else(short)?);
    }
    Ok(out)
}

fn color_info(info: &gst_video::VideoInfo) -> (Format, ColorInfo) {
    let c = info.colorimetry();
    let format = if c.range() == gst_video::VideoColorRange::Range0_255 { Format::Nv12Full } else { Format::Nv12Video };
    let matrix = match c.matrix() {
        gst_video::VideoColorMatrix::Bt601 => Matrix::Bt601,
        gst_video::VideoColorMatrix::Unknown if info.height() <= 576 => Matrix::Bt601,
        _ => Matrix::Bt709,
    };
    let transfer = if c.transfer() == gst_video::VideoTransferFunction::Srgb { Transfer::Srgb } else { Transfer::Bt709 };
    (format, ColorInfo { matrix, transfer })
}

/// An NV12 system-memory sample as Rune's packed planes.
pub(crate) fn nv12_frame(sample: &gst::Sample) -> Result<ByteFrame, VideoError> {
    let buffer = sample.buffer().ok_or_else(|| VideoError::Frame("sample has no buffer".into()))?;
    let caps = sample.caps().ok_or_else(|| VideoError::Frame("sample has no caps".into()))?;
    let info = gst_video::VideoInfo::from_caps(caps).map_err(|e| VideoError::Frame(e.to_string()))?;
    if info.format() != gst_video::VideoFormat::Nv12 {
        return Err(VideoError::Frame(format!("expected NV12, got {}", info.format())));
    }
    let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
        .map_err(|e| VideoError::Frame(e.to_string()))?;
    let (w, h) = (info.width() as usize, info.height() as usize);
    let strides = frame.plane_stride();
    let stride = |i: usize| strides.get(i).copied().and_then(|s| usize::try_from(s).ok()).unwrap_or(0);
    let y = frame.plane_data(0).map_err(|e| VideoError::Frame(e.to_string()))?;
    let uv = frame.plane_data(1).map_err(|e| VideoError::Frame(e.to_string()))?;
    let (format, color) = color_info(&info);
    Ok(ByteFrame {
        width: info.width(),
        height: info.height(),
        format,
        color,
        planes: [copy_plane(y, stride(0), w, h)?, copy_plane(uv, stride(1), w, h / 2)?],
    })
}

/// Lends an NV12 system-memory sample's planes to `with`, without copying them.
pub(crate) fn nv12_view<R>(sample: &gst::Sample, with: impl FnOnce(crate::tap::Nv12View<'_>) -> R) -> Result<R, VideoError> {
    let buffer = sample.buffer().ok_or_else(|| VideoError::Frame("sample has no buffer".into()))?;
    let caps = sample.caps().ok_or_else(|| VideoError::Frame("sample has no caps".into()))?;
    let info = gst_video::VideoInfo::from_caps(caps).map_err(|e| VideoError::Frame(e.to_string()))?;
    if info.format() != gst_video::VideoFormat::Nv12 {
        return Err(VideoError::Frame(format!("expected NV12, got {}", info.format())));
    }
    let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
        .map_err(|e| VideoError::Frame(e.to_string()))?;
    let strides = frame.plane_stride();
    let stride = |i: usize| strides.get(i).copied().and_then(|s| usize::try_from(s).ok()).unwrap_or(0);
    let luma = frame.plane_data(0).map_err(|e| VideoError::Frame(e.to_string()))?;
    let chroma = frame.plane_data(1).map_err(|e| VideoError::Frame(e.to_string()))?;
    let c = info.colorimetry();
    let bt601 = match c.matrix() {
        gst_video::VideoColorMatrix::Bt601 => true,
        gst_video::VideoColorMatrix::Unknown => info.height() <= 576,
        _ => false,
    };
    Ok(with(crate::tap::Nv12View {
        width: info.width(),
        height: info.height(),
        luma,
        luma_stride: stride(0),
        chroma,
        chroma_stride: stride(1),
        full_range: c.range() == gst_video::VideoColorRange::Range0_255,
        bt601,
    }))
}

/// A tightly packed RGBA copy of a sample already converted to RGBA.
pub(crate) fn rgba_pixels(sample: &gst::Sample) -> Result<(u32, u32, Vec<u8>), VideoError> {
    let buffer = sample.buffer().ok_or_else(|| VideoError::Frame("sample has no buffer".into()))?;
    let caps = sample.caps().ok_or_else(|| VideoError::Frame("sample has no caps".into()))?;
    let info = gst_video::VideoInfo::from_caps(caps).map_err(|e| VideoError::Frame(e.to_string()))?;
    if info.format() != gst_video::VideoFormat::Rgba {
        return Err(VideoError::Frame(format!("expected RGBA, got {}", info.format())));
    }
    let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
        .map_err(|e| VideoError::Frame(e.to_string()))?;
    let data = frame.plane_data(0).map_err(|e| VideoError::Frame(e.to_string()))?;
    let stride = frame.plane_stride().first().copied().and_then(|s| usize::try_from(s).ok()).unwrap_or(0);
    let pixels = copy_plane(data, stride, info.width() as usize * 4, info.height() as usize)?;
    Ok((info.width(), info.height(), pixels))
}
