//! The JSON lines on stdout.

use serde_json::{Value, json};

/// The state of the whole program, or of one destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Starting,
    Live,
    Reconnecting,
    Stopped,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Starting => "starting",
            State::Live => "live",
            State::Reconnecting => "reconnecting",
            State::Stopped => "stopped",
        }
    }
}

/// The state of the program given the states of its destinations.
///
/// Live when any destination is live. Reconnecting when one was live before and none is now.
/// Starting before the first one connects.
pub fn overall(dests: &[State], was_live: bool) -> State {
    if dests.contains(&State::Live) {
        State::Live
    } else if !dests.is_empty() && dests.iter().all(|s| *s == State::Stopped) {
        State::Stopped
    } else if was_live || dests.contains(&State::Reconnecting) {
        State::Reconnecting
    } else {
        State::Starting
    }
}

/// One destination in a status line.
#[derive(Debug, Clone)]
pub struct DestStatus {
    pub name: String,
    pub state: State,
}

/// What one status line says.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub state: State,
    pub seconds_live: u64,
    pub frames_encoded: u64,
    pub frames_dropped: u64,
    pub frames_repeated: u64,
    pub encoder: String,
    pub bitrate_kbps: u64,
    pub picture: &'static str,
    pub destinations: Vec<DestStatus>,
}

impl Snapshot {
    pub fn to_json(&self) -> Value {
        let dests: Vec<Value> = self
            .destinations
            .iter()
            .map(|d| json!({ "name": d.name, "state": d.state.as_str() }))
            .collect();
        json!({
            "type": "status",
            "state": self.state.as_str(),
            "seconds_live": self.seconds_live,
            "frames_encoded": self.frames_encoded,
            "frames_dropped": self.frames_dropped,
            "frames_repeated": self.frames_repeated,
            "encoder": self.encoder,
            "bitrate_kbps": self.bitrate_kbps,
            "picture": self.picture,
            "destinations": dests,
        })
    }
}

/// A problem in one line, with a sentence a volunteer can read. `detail` is for the log.
pub fn problem(message: &str, detail: Option<&str>) -> Value {
    match detail {
        Some(d) => json!({ "type": "problem", "message": message, "detail": d }),
        None => json!({ "type": "problem", "message": message }),
    }
}

/// A line that says what happened, such as the encoder chosen.
pub fn info(message: &str) -> Value {
    json!({ "type": "info", "message": message })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overall_follows_the_destinations() {
        use State::*;
        assert_eq!(overall(&[Starting, Starting], false), Starting);
        assert_eq!(overall(&[Live, Reconnecting], true), Live);
        assert_eq!(overall(&[Reconnecting], true), Reconnecting);
        assert_eq!(overall(&[Starting], true), Reconnecting);
        assert_eq!(overall(&[Stopped, Stopped], true), Stopped);
        assert_eq!(overall(&[], false), Starting);
    }

    #[test]
    fn status_line_has_every_field() {
        let s = Snapshot {
            state: State::Live,
            seconds_live: 12,
            frames_encoded: 360,
            frames_dropped: 1,
            frames_repeated: 2,
            encoder: "qsvh264enc".into(),
            bitrate_kbps: 2400,
            picture: "ok",
            destinations: vec![DestStatus {
                name: "rtmp://h/app/***".into(),
                state: State::Live,
            }],
        };
        let v = s.to_json();
        assert_eq!(v["type"], "status");
        assert_eq!(v["state"], "live");
        assert_eq!(v["seconds_live"], 12);
        assert_eq!(v["frames_encoded"], 360);
        assert_eq!(v["frames_dropped"], 1);
        assert_eq!(v["encoder"], "qsvh264enc");
        assert_eq!(v["bitrate_kbps"], 2400);
        assert_eq!(v["destinations"][0]["state"], "live");
        assert!(!v.to_string().contains('\n'));
    }

    #[test]
    fn problem_lines_are_one_line() {
        let v = problem("The window is gone. Waiting for it.", Some("x\ny"));
        assert!(!v.to_string().contains('\n'));
        assert_eq!(v["type"], "problem");
    }
}
