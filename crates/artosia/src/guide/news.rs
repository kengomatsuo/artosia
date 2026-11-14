//! What's new: each release's new features, shown once after an update to a version that has
//! entries, and from the search whenever asked. Every entry carries its recorded clip and the
//! lesson its Try button starts (JetBrains' looping clip per section and Runway's "Try it now"
//! in the research, `~/Shared/inspo/artosia-guide/`).

use super::lessons::Feature;

/// The features a release brought, newest release first.
pub const RELEASES: &[(&str, &[Feature])] = &[("0.1.32", &[Feature::Calendar, Feature::People, Feature::Giving, Feature::Templates, Feature::WebSongs, Feature::Pptx])];

fn parts(version: &str) -> (u32, u32, u32) {
    let mut numbers = version.trim().split('.').map(|part| part.parse::<u32>().unwrap_or(0));
    (numbers.next().unwrap_or(0), numbers.next().unwrap_or(0), numbers.next().unwrap_or(0))
}

/// The features of every release after `seen` up to and including `running`, newest first. A
/// church that never saw any (`seen` empty) is shown the newest release only, never the history.
pub fn unseen(seen: &str, running: &str) -> Vec<Feature> {
    let running = parts(running);
    let releases = RELEASES.iter().filter(|(version, _)| parts(version) <= running);
    let picked: Vec<&(&str, &[Feature])> =
        if seen.trim().is_empty() { releases.take(1).collect() } else { releases.filter(|(version, _)| parts(version) > parts(seen)).collect() };
    picked.into_iter().flat_map(|(_, features)| features.iter().copied()).collect()
}

/// The newest release with entries at or below `running`, the version the panel is about.
pub fn newest(running: &str) -> Option<&'static str> {
    RELEASES.iter().map(|(version, _)| *version).find(|version| parts(version) <= parts(running))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_update_shows_what_came_since_and_never_twice() {
        let newest_count = RELEASES.last().map_or(0, |(_, features)| features.len());
        assert_eq!(unseen("0.1.31", "0.1.32").len(), newest_count);
        assert!(unseen("0.1.32", "0.1.32").is_empty());
        assert!(unseen("0.1.31", "0.1.31").is_empty(), "a version not yet released shows nothing");
        assert_eq!(unseen("", "0.1.40").len(), newest_count, "a church that never saw any gets the newest release only");
        assert_eq!(newest("0.1.33"), Some("0.1.32"));
        assert_eq!(newest("0.1.20"), None);
    }
}
