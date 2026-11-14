use std::fmt;

/// Why a file could not be read.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    /// Not a zip, or a damaged one.
    Zip(String),
    /// A part the format requires is not in the zip.
    MissingPart(String),
    Xml(String),
    /// Neither PowerPoint nor LibreOffice is installed, so slides cannot be drawn as made.
    NoRenderer,
    /// A program was found but did not draw every slide (it failed or ran past its time limit).
    Export(String),
    /// The export was stopped by a `Cancel`.
    Cancelled,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "cannot read the file: {e}"),
            Error::Zip(e) => write!(f, "not a readable Office file: {e}"),
            Error::MissingPart(p) => write!(f, "the file has no {p}"),
            Error::Xml(e) => write!(f, "damaged XML: {e}"),
            Error::NoRenderer => write!(f, "neither PowerPoint nor LibreOffice is installed"),
            Error::Export(e) => write!(f, "drawing the slides failed: {e}"),
            Error::Cancelled => write!(f, "drawing the slides was cancelled"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
