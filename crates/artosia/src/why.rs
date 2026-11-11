//! Why a download or a file failed, in the window's language. Each reason is a clause that
//! finishes a sentence such as "Could not get the Bible: {why}", so it starts in lower case.
//! The libraries' own error text is English and written for programmers, and none of it
//! reaches the screen: the Indonesian reader met "there is no connection" under an Indonesian
//! heading (2026-11-11).

use crate::strings as t;

/// A call over the network.
pub fn net(error: ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(429 | 503) => t::WHY_BUSY().to_string(),
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
        Failure::Server(429 | 503) => t::WHY_BUSY().to_string(),
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

/// A source that would not open, as a whole sentence for the Listen dialog.
pub fn listen_error(error: &artosia_listen::ListenError) -> String {
    use artosia_listen::ListenError;
    match error {
        ListenError::NoDevice(_) => t::LISTEN_NOT_CONNECTED().to_string(),
        ListenError::Stream(_) | ListenError::Thread(_) => t::LISTEN_WOULD_NOT_OPEN().to_string(),
    }
}

/// What the listener reported while it ran, as a whole sentence, or nothing when the dialog
/// already says it (a missing model has its own row with the button that fetches it) or when it
/// is not worth a volunteer's eye (one phrase that could not be read).
/// A background the projector program could not show, in the window's language. The program
/// sends a kind and what it is about with a tab between them (`artosia-output` `video.rs`).
pub fn media_problem(code: &str) -> String {
    let (kind, about) = code.split_once('\t').unwrap_or((code, ""));
    match kind {
        "camera-none" => t::MEDIA_CAMERA_NONE().to_string(),
        "camera-missing" => t::media_camera_missing(about),
        "camera-busy" => t::MEDIA_CAMERA_BUSY().to_string(),
        "camera-stopped" => t::MEDIA_CAMERA_STOPPED().to_string(),
        "camera-far" => t::MEDIA_CAMERA_FAR().to_string(),
        "video-missing" => t::media_video_missing(about),
        "video-failed" | "video-stopped" => t::media_video_failed(about),
        "video-software" => t::MEDIA_VIDEO_SOFTWARE().to_string(),
        "video-none" => t::MEDIA_VIDEO_NONE().to_string(),
        // An older projector program still sends a sentence of its own.
        _ => code.to_string(),
    }
}

pub fn listen_problem(problem: &artosia_listen::Problem, speech_built: bool) -> Option<String> {
    use artosia_listen::Problem;
    match problem {
        Problem::NoModel if speech_built => None,
        Problem::NoModel | Problem::NoRecogniser => Some(t::LISTEN_BEAT_ONLY().to_string()),
        Problem::Model(_) => Some(t::LISTEN_MODEL_DAMAGED().to_string()),
        Problem::Input(_) => Some(t::LISTEN_INPUT_STOPPED().to_string()),
        // A phrase that could not be read is not worth a volunteer's eye, and a folder that
        // cannot be written is said beside the recordings, where it can be moved.
        Problem::Reading(_) | Problem::Keep(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_background_that_fails_is_worded_here_and_names_what_it_is_about() {
        assert_eq!(media_problem("camera-none"), t::MEDIA_CAMERA_NONE());
        assert_eq!(media_problem("camera-missing\tFront camera"), t::media_camera_missing("Front camera"));
        assert!(media_problem("video-missing\twater.mp4").contains("water.mp4"));
        assert_eq!(media_problem("video-stopped\twater.mp4"), t::media_video_failed("water.mp4"));
        assert_eq!(media_problem("camera-far"), t::MEDIA_CAMERA_FAR());
    }

    #[test]
    fn a_reason_is_never_the_library_s_own_words() {
        t::set_indonesian(false);
        assert_eq!(net(ureq::Error::ConnectionFailed), "there is no internet connection");
        assert_eq!(net(ureq::Error::StatusCode(404)), "the server answered 404");
        assert_eq!(backdrop(&crate::backdrops::Failure::Disk("os error 5".into())), t::WHY_DISK());
        assert!(!lyrics(&artosia_lyrics::LyricsError::Network("dns error".into())).contains("dns"));
    }

    #[test]
    fn a_listening_problem_never_shows_a_path_or_a_device_id() {
        use artosia_listen::{ListenError, Problem};
        t::set_indonesian(false);
        assert_eq!(listen_error(&ListenError::NoDevice("device wasapi:{0.0.1} is not connected".into())), t::LISTEN_NOT_CONNECTED());
        assert_eq!(listen_error(&ListenError::Stream("0x88890008".into())), t::LISTEN_WOULD_NOT_OPEN());
        assert_eq!(listen_problem(&Problem::Input("device lost".into()), true).as_deref(), Some(t::LISTEN_INPUT_STOPPED()));
        // A missing model has its own row with the button that fetches it, so it adds no line.
        assert_eq!(listen_problem(&Problem::NoModel, true), None);
        assert_eq!(listen_problem(&Problem::NoModel, false).as_deref(), Some(t::LISTEN_BEAT_ONLY()));
        assert_eq!(listen_problem(&Problem::Model(r"C:\Users\x\models\file.bin".into()), true).as_deref(), Some(t::LISTEN_MODEL_DAMAGED()));
        assert_eq!(listen_problem(&Problem::Reading("decoder".into()), true), None);
    }
}
