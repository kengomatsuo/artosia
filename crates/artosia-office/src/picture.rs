//! Pictures found on a slide, and turning the ones Artosia cannot draw into PNG.

use std::fmt;

/// A box on the slide as fractions of its width and height (0.0 to 1.0, can overshoot when the
/// author dragged a shape off the edge).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Bmp,
    Tiff,
    Emf,
    Wmf,
    Svg,
    Other,
}

impl ImageFormat {
    /// By the first bytes, falling back to the file extension.
    pub fn detect(bytes: &[u8], file_name: &str) -> ImageFormat {
        if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
            return ImageFormat::Png;
        }
        if bytes.starts_with(&[0xFF, 0xD8]) {
            return ImageFormat::Jpeg;
        }
        if bytes.starts_with(b"GIF8") {
            return ImageFormat::Gif;
        }
        if bytes.starts_with(b"BM") {
            return ImageFormat::Bmp;
        }
        if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
            return ImageFormat::Tiff;
        }
        // An EMF header is record type 1 and carries " EMF" at byte 40.
        if bytes.len() > 44 && bytes[..4] == [1, 0, 0, 0] && &bytes[40..44] == b" EMF" {
            return ImageFormat::Emf;
        }
        // A placeable WMF starts with a fixed key; a bare one with type 1 or 2 and header size 9.
        if bytes.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A]) || (bytes.len() > 4 && matches!(bytes[0], 1 | 2) && bytes[1] == 0 && bytes[2..4] == [9, 0]) {
            return ImageFormat::Wmf;
        }
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).to_ascii_lowercase();
        if head.contains("<svg") {
            return ImageFormat::Svg;
        }
        match file_name.rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
            Some("png") => ImageFormat::Png,
            Some("jpg" | "jpeg") => ImageFormat::Jpeg,
            Some("gif") => ImageFormat::Gif,
            Some("bmp") => ImageFormat::Bmp,
            Some("tif" | "tiff") => ImageFormat::Tiff,
            Some("emf") => ImageFormat::Emf,
            Some("wmf") => ImageFormat::Wmf,
            Some("svg") => ImageFormat::Svg,
            _ => ImageFormat::Other,
        }
    }

    /// The formats Artosia draws as they are.
    pub fn is_drawable(self) -> bool {
        matches!(self, ImageFormat::Png | ImageFormat::Jpeg)
    }
}

#[derive(Clone, Debug)]
pub struct Picture {
    /// The file name inside the package, such as `image1.emf`.
    pub name: String,
    pub bytes: Vec<u8>,
    pub format: ImageFormat,
    /// Where it sits on the slide; None when no position could be found.
    pub rect: Option<Rect>,
    /// The author's alternative text, often empty.
    pub alt: String,
}

#[derive(Debug)]
pub enum ConvertError {
    /// Metafiles are turned into PNG with the Windows GDI+ that ships with the OS.
    UnsupportedPlatform(ImageFormat),
    /// The format cannot be converted at all (SVG, unknown bytes).
    UnsupportedFormat(ImageFormat),
    Failed(String),
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvertError::UnsupportedPlatform(k) => write!(f, "converting {k:?} to PNG needs Windows (GDI+); not available on this OS"),
            ConvertError::UnsupportedFormat(k) => write!(f, "no converter for {k:?} pictures"),
            ConvertError::Failed(why) => write!(f, "picture conversion failed: {why}"),
        }
    }
}

impl std::error::Error for ConvertError {}

impl Picture {
    /// The picture in a format Artosia draws: PNG and JPEG come back untouched, everything GDI+
    /// reads (EMF, WMF, BMP, GIF, TIFF) is drawn onto a white `width`-pixel-wide PNG that keeps
    /// the picture's aspect ratio. White, not clear, so a QR code stays scannable.
    pub fn drawable(&self, width: u32) -> Result<(ImageFormat, Vec<u8>), ConvertError> {
        if self.format.is_drawable() {
            return Ok((self.format, self.bytes.clone()));
        }
        self.to_png(width).map(|png| (ImageFormat::Png, png))
    }

    /// Always a PNG, even from a JPEG. Windows only.
    pub fn to_png(&self, width: u32) -> Result<Vec<u8>, ConvertError> {
        match self.format {
            ImageFormat::Svg | ImageFormat::Other => Err(ConvertError::UnsupportedFormat(self.format)),
            format => windows::to_png(&self.bytes, format, width.clamp(16, 8192)),
        }
    }
}

#[cfg(windows)]
mod windows {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::{ConvertError, ImageFormat};

    /// GDI+ through the Windows PowerShell every Windows install has: no unsafe code and no
    /// dependency in the build. It costs a process start (a few hundred ms), paid once per
    /// picture at import. Paths and size travel in the environment so nothing needs quoting.
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$img = [System.Drawing.Image]::FromFile($env:ARTOSIA_IN)
try {
    $w = [int]$env:ARTOSIA_W
    $h = [Math]::Max(1, [int][Math]::Round($img.Height * $w / [double]$img.Width))
    $bmp = New-Object System.Drawing.Bitmap $w, $h, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.InterpolationMode = 'HighQualityBicubic'
    $g.PixelOffsetMode = 'HighQuality'
    $g.TextRenderingHint = 'AntiAlias'
    $g.Clear([System.Drawing.Color]::White)
    $g.DrawImage($img, 0, 0, $w, $h)
    $g.Dispose()
    $bmp.Save($env:ARTOSIA_OUT, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
} finally { $img.Dispose() }
"#;

    static COUNTER: AtomicU32 = AtomicU32::new(0);
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    pub fn to_png(bytes: &[u8], format: ImageFormat, width: u32) -> Result<Vec<u8>, ConvertError> {
        let fail = |e: &dyn std::fmt::Display| ConvertError::Failed(e.to_string());
        let stem = format!("artosia-office-{}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed));
        let input = std::env::temp_dir().join(format!("{stem}.{}", format!("{format:?}").to_ascii_lowercase()));
        let output = std::env::temp_dir().join(format!("{stem}.png"));
        std::fs::write(&input, bytes).map_err(|e| fail(&e))?;
        let result = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", SCRIPT])
            .env("ARTOSIA_IN", &input)
            .env("ARTOSIA_OUT", &output)
            .env("ARTOSIA_W", width.to_string())
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        let png = match result {
            Ok(out) if out.status.success() => std::fs::read(&output).map_err(|e| fail(&e)),
            Ok(out) => Err(ConvertError::Failed(String::from_utf8_lossy(&out.stderr).trim().to_string())),
            Err(e) => Err(fail(&e)),
        };
        let _ = std::fs::remove_file(&input);
        let _ = std::fs::remove_file(&output);
        png
    }
}

#[cfg(not(windows))]
mod windows {
    use super::{ConvertError, ImageFormat};

    pub fn to_png(_bytes: &[u8], format: ImageFormat, _width: u32) -> Result<Vec<u8>, ConvertError> {
        Err(ConvertError::UnsupportedPlatform(format))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(bytes: Vec<u8>, name: &str) -> Picture {
        let format = ImageFormat::detect(&bytes, name);
        Picture { name: name.into(), bytes, format, rect: None, alt: String::new() }
    }

    #[test]
    fn formats_are_told_by_their_first_bytes() {
        let mut emf = vec![0u8; 88];
        emf[0] = 1;
        emf[40..44].copy_from_slice(b" EMF");
        assert_eq!(ImageFormat::detect(&emf, "x.bin"), ImageFormat::Emf);
        assert_eq!(ImageFormat::detect(&[0xD7, 0xCD, 0xC6, 0x9A, 0, 0], "x"), ImageFormat::Wmf);
        assert_eq!(ImageFormat::detect(&[0xFF, 0xD8, 0xFF], "x"), ImageFormat::Jpeg);
        assert_eq!(ImageFormat::detect(b"<svg xmlns=''/>", "x"), ImageFormat::Svg);
        assert_eq!(ImageFormat::detect(b"??", "photo.JPG"), ImageFormat::Jpeg);
    }

    #[test]
    fn png_and_jpeg_pass_through_untouched() {
        let p = picture(crate::test_files::PNG_1X1.to_vec(), "a.png");
        let (format, bytes) = p.drawable(100).unwrap();
        assert_eq!((format, bytes), (ImageFormat::Png, crate::test_files::PNG_1X1.to_vec()));
    }

    #[test]
    fn svg_has_no_converter() {
        let p = picture(b"<svg/>".to_vec(), "a.svg");
        assert!(matches!(p.to_png(100), Err(ConvertError::UnsupportedFormat(ImageFormat::Svg))));
    }

    /// A 2x1 24-bit BMP stands in for a metafile: GDI+ reads both through the same path.
    #[cfg(windows)]
    #[test]
    fn gdi_plus_turns_a_bmp_into_a_png_of_the_asked_width() {
        let mut bmp = b"BM".to_vec();
        bmp.extend_from_slice(&(54u32 + 8).to_le_bytes());
        bmp.extend_from_slice(&[0; 4]);
        bmp.extend_from_slice(&54u32.to_le_bytes());
        bmp.extend_from_slice(&40u32.to_le_bytes());
        bmp.extend_from_slice(&2i32.to_le_bytes());
        bmp.extend_from_slice(&1i32.to_le_bytes());
        bmp.extend_from_slice(&1u16.to_le_bytes());
        bmp.extend_from_slice(&24u16.to_le_bytes());
        bmp.extend_from_slice(&[0; 24]);
        bmp.extend_from_slice(&[255, 0, 0, 0, 255, 0, 0, 0]);
        let png = picture(bmp, "a.bmp").to_png(64).unwrap();
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));
        // IHDR width and height follow the 8-byte signature and the 8-byte chunk header.
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 64);
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 32);
    }

    #[cfg(not(windows))]
    #[test]
    fn off_windows_a_metafile_says_so() {
        let p = picture(vec![0xD7, 0xCD, 0xC6, 0x9A, 0, 0], "a.wmf");
        assert!(matches!(p.to_png(100), Err(ConvertError::UnsupportedPlatform(ImageFormat::Wmf))));
    }
}
