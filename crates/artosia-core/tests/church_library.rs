//! Imports the church's cleaned library when it is on this machine (`fixtures/church/library`,
//! gitignored: it holds copyrighted lyrics). Skipped everywhere else. Run with `--nocapture`
//! to see the counts.

use std::path::PathBuf;

use artosia_core::openlyrics::read_folder;

fn library() -> Option<PathBuf> {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/church/library");
    folder.join("songs").is_dir().then_some(folder)
}

#[test]
fn every_file_reads_and_empty_placeholders_are_skipped() {
    let Some(folder) = library() else { return };
    let index: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(folder.join("index.json")).unwrap_or_default()).unwrap_or_default();
    let files = std::fs::read_dir(folder.join("songs"))
        .map(|entries| entries.flatten().filter(|entry| entry.path().extension().is_some_and(|e| e == "xml")).count())
        .unwrap_or(0);
    let empty: Vec<&str> = index
        .iter()
        .filter(|row| row.get("kind").and_then(|kind| kind.as_str()) == Some("empty"))
        .filter_map(|row| row.get("id").and_then(|id| id.as_str()))
        .collect();
    let empty_files = empty.iter().filter(|id| folder.join("songs").join(format!("{id}.xml")).is_file()).count();
    let (songs, failed) = read_folder(&folder);
    println!("{files} files, {} index rows, {} empty, {} read, {failed} failed", index.len(), empty.len(), songs.len());
    assert_eq!(failed, 0, "files that could not be read");
    assert_eq!(songs.len(), files - empty_files);
    assert!(songs.iter().all(|song| !song.source.is_empty() && !song.lyrics.trim().is_empty()));
    assert!(!songs.iter().any(|song| empty.contains(&song.source.as_str())));
}

#[test]
fn alternate_titles_come_from_the_index() {
    let Some(folder) = library() else { return };
    let index: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(folder.join("index.json")).unwrap_or_default()).unwrap_or_default();
    let (songs, _) = read_folder(&folder);
    let mut checked = 0;
    for row in &index {
        let (Some(id), Some(titles)) = (row.get("id").and_then(|id| id.as_str()), row.get("alt_titles").and_then(|t| t.as_array())) else { continue };
        let Some(song) = songs.iter().find(|song| song.source == id) else { continue };
        for title in titles.iter().filter_map(|title| title.as_str()) {
            if title != song.title {
                assert!(song.alt_title.split(" / ").any(|own| own == title), "{id} lacks an alternate title from the index");
                checked += 1;
            }
        }
    }
    println!("{checked} alternate titles checked");
}
