//! The decisions about rundown items that need no GStreamer: what an end does, how loud music is
//! under a video, and how fast it moves there. `items.rs` plays; this file only decides.

use artosia_slides::frame::{MediaEnd, MediaItem, MediaKind};

/// What the output does when an item reaches its end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AtEnd {
    /// Play again from the start.
    Again,
    /// Tell the operator, and show black: the item layer never falls back to the background.
    Black,
    /// Tell the operator and keep the last frame, so a next video follows without a flash.
    Hold,
}

pub fn at_end(end: MediaEnd) -> AtEnd {
    match end {
        MediaEnd::Loop => AtEnd::Again,
        MediaEnd::Stop => AtEnd::Black,
        MediaEnd::Next => AtEnd::Hold,
    }
}

/// An item's own level, 0.0 to 1.0.
pub fn gain(volume: u8) -> f64 {
    f64::from(volume.min(100)) / 100.0
}

/// Music under a video with sound plays at a third of its level (FreeShow lowers other audio
/// under an audible video).
pub fn music_gain(music: &MediaItem, ducked: bool) -> f64 {
    if ducked { gain(music.volume) / 3.0 } else { gain(music.volume) }
}

/// Whether music is ducked: a video item is playing with sound.
pub fn ducks(front: Option<&MediaItem>, playing: bool) -> bool {
    playing && front.is_some_and(|item| item.kind == MediaKind::Video && item.volume > 0)
}

/// How long the music takes to reach its new level.
pub const RAMP_MS: u64 = 500;

/// One step from `now` toward `target`, a fraction `step_ms / RAMP_MS` of the whole 0..1 range.
pub fn ramp(now: f64, target: f64, step_ms: u64) -> f64 {
    let step = step_ms as f64 / RAMP_MS as f64;
    if (target - now).abs() <= step { target } else { now + step.copysign(target - now) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: MediaKind, volume: u8) -> MediaItem {
        MediaItem { id: 1, path: "x".into(), kind, end: MediaEnd::Stop, volume }
    }

    #[test]
    fn each_end_does_its_own_thing() {
        assert_eq!(at_end(MediaEnd::Loop), AtEnd::Again);
        assert_eq!(at_end(MediaEnd::Stop), AtEnd::Black);
        assert_eq!(at_end(MediaEnd::Next), AtEnd::Hold);
    }

    #[test]
    fn a_level_above_100_plays_as_100() {
        assert_eq!(gain(0), 0.0);
        assert_eq!(gain(50), 0.5);
        assert_eq!(gain(250), 1.0);
    }

    #[test]
    fn music_drops_to_a_third_under_a_video_with_sound() {
        let music = item(MediaKind::Audio, 90);
        assert!((music_gain(&music, false) - 0.9).abs() < 1e-9);
        assert!((music_gain(&music, true) - 0.3).abs() < 1e-9);
    }

    #[test]
    fn only_a_playing_video_with_sound_ducks() {
        let video = item(MediaKind::Video, 80);
        assert!(ducks(Some(&video), true));
        assert!(!ducks(Some(&video), false), "an ended video lets the music back");
        assert!(!ducks(Some(&item(MediaKind::Video, 0)), true), "a muted video leaves the music alone");
        assert!(!ducks(Some(&item(MediaKind::Audio, 80)), true), "a second sound is not a video");
        assert!(!ducks(None, true));
    }

    #[test]
    fn the_music_reaches_its_level_in_half_a_second() {
        let (mut level, mut steps) = (0.9, 0);
        while level != 0.3 {
            level = ramp(level, 0.3, 100);
            steps += 1;
            assert!(steps <= 5, "stuck at {level}");
        }
        assert_eq!(ramp(0.3, 0.9, 100), 0.5);
        assert_eq!(ramp(0.5, 0.5, 100), 0.5);
    }
}
