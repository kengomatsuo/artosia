use std::fmt;
use std::io;

/// Every failure reads as a short plain sentence; `detail` keeps the technical cause for logs.
#[derive(Debug)]
pub enum UpdateError {
    NotSetUp,
    Offline,
    Slow,
    NotPublished,
    Server(String),
    BadList(String),
    BadSignature,
    WrongPlatform,
    BadFile(String),
    Disk(String),
    NothingStaged,
    StaleStaged,
    Rejected,
    InstallFailed(String),
    RestoreFailed(String),
    BadVersion(String),
}

impl UpdateError {
    /// True for failures an automatic check should swallow (offline, no feed yet, and so on).
    pub fn is_quiet(&self) -> bool {
        matches!(
            self,
            Self::NotSetUp
                | Self::Offline
                | Self::Slow
                | Self::NotPublished
                | Self::NothingStaged
                | Self::Rejected
        )
    }

    /// The technical cause, for the log file and never for the screen.
    pub fn detail(&self) -> Option<&str> {
        match self {
            Self::Server(d)
            | Self::BadList(d)
            | Self::BadFile(d)
            | Self::Disk(d)
            | Self::InstallFailed(d)
            | Self::RestoreFailed(d)
            | Self::BadVersion(d) => Some(d),
            _ => None,
        }
    }
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NotSetUp => "Updates are not set up yet",
            Self::Offline => "No internet connection",
            Self::Slow => "The update server is too slow",
            Self::NotPublished => "No update has been published",
            Self::Server(_) => "The update server gave an error",
            Self::BadList(_) => "The update list is damaged",
            Self::BadSignature => "This update is not from Artosia",
            Self::WrongPlatform => "This update is for another computer",
            Self::BadFile(_) => "A downloaded file is damaged",
            Self::Disk(_) => "Could not save the update to disk",
            Self::NothingStaged => "No update is waiting",
            Self::StaleStaged => "The waiting update is out of date",
            Self::Rejected => "This update failed here before",
            Self::InstallFailed(_) => "Could not install the update",
            Self::RestoreFailed(_) => "Could not restore the old version",
            Self::BadVersion(_) => "The version number is not valid",
        })
    }
}

impl std::error::Error for UpdateError {}

impl From<io::Error> for UpdateError {
    fn from(e: io::Error) -> Self {
        Self::Disk(e.to_string())
    }
}
