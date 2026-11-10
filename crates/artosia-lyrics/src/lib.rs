//! Finding a song's words on the web when the church's library lacks it.
//!
//! One keyless source today, LRCLIB (see `README.md` for why). Both calls block for a few
//! seconds at most and never panic; run them on a worker thread.
//!
//! ```no_run
//! let found = artosia_lyrics::search("how great is our god")?;
//! if let Some(first) = found.first() {
//!     let song = artosia_lyrics::fetch(first)?;
//!     println!("{} stanzas from {}", song.text.split("\n\n").count(), song.source);
//! }
//! # Ok::<(), artosia_lyrics::LyricsError>(())
//! ```

#![cfg_attr(test, allow(clippy::unwrap_used))]

mod http;
mod lrclib;
mod text;

use std::fmt;

/// One suggestion in the list under the title field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// Identifies the song inside its source; only `fetch` reads it.
    pub id: String,
    pub title: String,
    pub artist: String,
    /// Human name of the service, for the credit line ("LRCLIB").
    pub source: String,
    /// A page or endpoint where the song can be checked.
    pub url: String,
}

/// A song's words, ready for `song_from_paste`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub title: String,
    pub artist: String,
    /// Stanzas separated by blank lines, as the source gave them. Timestamps are removed.
    pub text: String,
    pub source: String,
    pub url: String,
}

/// Why a lookup failed, in words a volunteer can read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LyricsError {
    /// No connection, a refused connection or a call that took longer than the limit.
    Network(String),
    /// The service has no such song.
    NotFound,
    /// The service asked us to slow down; the number is the wait it named, in seconds.
    RateLimited(Option<u64>),
    /// The service answered with something unreadable or unexpected.
    BadAnswer(String),
}

impl fmt::Display for LyricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(why) => {
                write!(f, "Could not reach the lyrics service. Check the internet connection ({why}).")
            }
            Self::NotFound => write!(f, "The lyrics service does not have this song."),
            Self::RateLimited(Some(secs)) => {
                write!(f, "The lyrics service is busy. Try again in {secs} seconds.")
            }
            Self::RateLimited(None) => write!(f, "The lyrics service is busy. Try again in a minute."),
            Self::BadAnswer(why) => write!(f, "The lyrics service sent an answer Artosia cannot read ({why})."),
        }
    }
}

impl std::error::Error for LyricsError {}

/// Longest query sent, in characters; a pasted paragraph is cut rather than refused.
const MAX_QUERY_CHARS: usize = 120;

/// Suggests songs for a title or a first line, best first. Empty when nothing matches.
///
/// Blocking, five seconds at most. Callers should wait for a pause in typing before calling:
/// the service allows about 90 calls a minute per address.
pub fn search(query: &str) -> Result<Vec<Found>, LyricsError> {
    let query = text::clean_query(query, MAX_QUERY_CHARS);
    if query.is_empty() {
        return Ok(Vec::new());
    }
    lrclib::search(&query)
}

/// Fetches the words of a song suggested by `search`.
///
/// Blocking, five seconds at most.
pub fn fetch(found: &Found) -> Result<Fetched, LyricsError> {
    lrclib::fetch(found)
}
