//! Backgrounds that draw themselves on the graphics chip while they show (LIVEBG1).
//!
//! The view puts an effect slot where the background goes, carrying the kind in its mark and the
//! colours and pace in its data; the compositor in `artosia-warp` paints every such slot. Where
//! nothing paints it, the gradient under the slot shows.

use serde::{Deserialize, Serialize};

/// What moves. Each is one fill drawn by a shader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LiveKind {
    /// Colours pinned to points that drift and blend.
    Flow,
    /// Curtains of light.
    Aurora,
    /// Colours stirred into a slow swirl.
    Ink,
    /// Layered bands that roll like the sea.
    Sea,
    /// Cells of coloured glass with dark seams.
    Glass,
    /// Glossy blobs that drift and merge.
    Lava,
    /// Twinkling stars over a night sky.
    Stars,
    /// A slow flow folded into mirrored wedges.
    Kaleidoscope,
}

impl LiveKind {
    pub const ALL: [LiveKind; 8] = [LiveKind::Flow, LiveKind::Aurora, LiveKind::Ink, LiveKind::Sea, LiveKind::Glass, LiveKind::Lava, LiveKind::Stars, LiveKind::Kaleidoscope];

    fn index(self) -> u32 {
        LiveKind::ALL.iter().position(|kind| *kind == self).unwrap_or(0) as u32
    }

    /// The colour set a kind starts with.
    pub fn first_colours(self) -> [[u8; 3]; 3] {
        let set = match self {
            LiveKind::Flow => 0,
            LiveKind::Aurora => 5,
            LiveKind::Ink => 1,
            LiveKind::Sea => 1,
            LiveKind::Glass => 6,
            LiveKind::Lava => 3,
            LiveKind::Stars => 5,
            LiveKind::Kaleidoscope => 4,
        };
        COLOUR_SETS[set].1
    }
}

/// How fast it moves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pace {
    Slow,
    #[default]
    Normal,
    Fast,
}

impl Pace {
    pub const ALL: [Pace; 3] = [Pace::Slow, Pace::Normal, Pace::Fast];

    /// A multiplier of each fill's own speed.
    pub fn factor(self) -> f32 {
        match self {
            Pace::Slow => 0.5,
            Pace::Normal => 1.0,
            Pace::Fast => 2.0,
        }
    }
}

/// A live background: what moves, in which colours, how fast.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Live {
    pub kind: LiveKind,
    /// Dark, middle and light, in that order.
    pub colours: [[u8; 3]; 3],
    #[serde(default)]
    pub pace: Pace,
}

impl Live {
    pub fn new(kind: LiveKind) -> Live {
        Live { kind, colours: kind.first_colours(), pace: Pace::Normal }
    }
}

/// Colour sets offered beside the kinds, by id: dark, middle and light. Each kind reads them its
/// own way; every set keeps white words readable on its darkest colour.
pub const COLOUR_SETS: [(&str, [[u8; 3]; 3]); 8] = [
    ("dusk", [[0x2b, 0x1b, 0x5a], [0xc0, 0x4f, 0x7a], [0xff, 0xc4, 0x6b]]),
    ("ocean", [[0x0a, 0x26, 0x49], [0x1b, 0x6c, 0xa8], [0x7f, 0xd3, 0xe8]]),
    ("forest", [[0x0d, 0x24, 0x1a], [0x2d, 0x6a, 0x4f], [0xb7, 0xd9, 0x8c]]),
    ("ember", [[0x1a, 0x0f, 0x1f], [0xd9, 0x48, 0x2b], [0xff, 0xb3, 0x47]]),
    ("royal", [[0x1c, 0x10, 0x3f], [0x6a, 0x3f, 0xb5], [0xe8, 0xc5, 0x6b]]),
    ("night", [[0x05, 0x0a, 0x1f], [0x16, 0x5c, 0x7a], [0x6b, 0xe3, 0xb4]]),
    ("glass", [[0x2e, 0x29, 0x4e], [0xd7, 0x26, 0x3d], [0x1b, 0x99, 0x8b]]),
    ("dawn", [[0x3d, 0x2c, 0x4f], [0xe0, 0x8a, 0x8a], [0xfd, 0xe6, 0xc4]]),
];

/// Effect slot marks for live backgrounds start here; the kind's place is added.
pub const MARK_BASE: u32 = 0x4c42_0000;

/// The slot's mark and data for a live background: the kind in the mark, the three colours as
/// whole numbers (exact in an f32 below 2^24) and the pace, 0 for one still pose.
pub fn slot(live: &Live, moving: bool) -> (u32, [f32; 4]) {
    let pack = |c: [u8; 3]| ((c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32) as f32;
    let pace = if moving { live.pace.factor() } else { 0.0 };
    (MARK_BASE + live.kind.index(), [pack(live.colours[0]), pack(live.colours[1]), pack(live.colours[2]), pace])
}

/// What a compositor reads back from a slot: None for a slot that is not a live background.
pub fn read(mark: u32, data: [f32; 4]) -> Option<(LiveKind, [[u8; 3]; 3], f32)> {
    let index = mark.checked_sub(MARK_BASE)? as usize;
    let kind = *LiveKind::ALL.get(index)?;
    let unpack = |v: f32| {
        let n = v.max(0.0) as u32;
        [(n >> 16 & 0xff) as u8, (n >> 8 & 0xff) as u8, (n & 0xff) as u8]
    };
    Some((kind, [unpack(data[0]), unpack(data[1]), unpack(data[2])], data[3].max(0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_reads_back_as_the_live_background_it_was_made_from() {
        for kind in LiveKind::ALL {
            let live = Live { kind, colours: [[1, 2, 3], [250, 128, 7], [255, 255, 255]], pace: Pace::Fast };
            let (mark, data) = slot(&live, true);
            assert_eq!(read(mark, data), Some((kind, live.colours, 2.0)));
        }
        let (mark, data) = slot(&Live::new(LiveKind::Sea), false);
        assert_eq!(read(mark, data).map(|(_, _, pace)| pace), Some(0.0), "a still pose has no pace");
        assert_eq!(read(7, data), None, "another slot is not a live background");
    }

    #[test]
    fn an_older_look_without_a_pace_reads_as_normal() {
        let live: Live = serde_json::from_str(r#"{"kind":"Sea","colours":[[0,0,0],[1,1,1],[2,2,2]]}"#).unwrap();
        assert_eq!(live.pace, Pace::Normal);
    }
}
