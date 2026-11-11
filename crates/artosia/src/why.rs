//! Why a download or a file failed, in the window's language. Each reason is a clause that
//! finishes a sentence such as "Could not get the Bible: {why}", so it starts in lower case.
//! The libraries' own error text is English and written for programmers, and none of it
//! reaches the screen: the Indonesian reader met "there is no connection" under an Indonesian
//! heading (2026-11-11).

use crate::strings as t;

/// A call over the network.
pub fn net(error: ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(code) => t::why_status(code),
        ureq::Error::Timeout(_) => t::WHY_SLOW().to_string(),
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed | ureq::Error::Io(_) => t::WHY_OFFLINE().to_string(),
        _ => t::WHY_BROKEN().to_string(),
    }
}

/// A Bible file that could not be brought in.
pub fn bible_file(error: &artosia_core::bible_file::ImportError) -> String {
    use artosia_core::bible_file::ImportError;
    match error {
        ImportError::Write(_) => t::WHY_DISK().to_string(),
        ImportError::Unsupported => t::WHY_NOT_BIBLE().to_string(),
        ImportError::Read(_) | ImportError::Empty => t::WHY_UNREADABLE().to_string(),
    }
}

/// A backdrop that did not arrive. Nothing for one the person cancelled.
pub fn backdrop(failure: &crate::backdrops::Failure) -> String {
    use crate::backdrops::Failure;
    match failure {
        Failure::Offline => t::WHY_OFFLINE().to_string(),
        Failure::Server(code) => t::why_status(*code),
        Failure::Disk(_) => t::WHY_DISK().to_string(),
        Failure::Empty => t::WHY_DAMAGED().to_string(),
        Failure::Cancelled => String::new(),
    }
}

/// A lyrics lookup, as a whole sentence for a toast.
pub fn lyrics(error: &artosia_lyrics::LyricsError) -> String {
    use artosia_lyrics::LyricsError;
    match error {
        LyricsError::Network(_) => t::LYRICS_OFFLINE().to_string(),
        LyricsError::NotFound => t::LYRICS_NOT_FOUND().to_string(),
        LyricsError::RateLimited(Some(seconds)) => t::lyrics_busy_for(*seconds),
        LyricsError::RateLimited(None) => t::LYRICS_BUSY().to_string(),
        LyricsError::BadAnswer(_) => t::LYRICS_UNREADABLE().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reason_is_never_the_library_s_own_words() {
        t::set_indonesian(false);
        assert_eq!(net(ureq::Error::ConnectionFailed), "there is no internet connection");
        assert_eq!(net(ureq::Error::StatusCode(404)), "the server answered 404");
        assert_eq!(backdrop(&crate::backdrops::Failure::Disk("os error 5".into())), t::WHY_DISK());
        assert!(!lyrics(&artosia_lyrics::LyricsError::Network("dns error".into())).contains("dns"));
    }
}
