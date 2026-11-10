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
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "cannot read the file: {e}"),
            Error::Zip(e) => write!(f, "not a readable Office file: {e}"),
            Error::MissingPart(p) => write!(f, "the file has no {p}"),
            Error::Xml(e) => write!(f, "damaged XML: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
