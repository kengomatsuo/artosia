//! Files handed to Artosia from outside the window (GESTURES1): dropped on the desktop
//! shortcut or the program's file, which Windows passes as the start's arguments, or passed on
//! by a second start to the copy already running. They wait here until the window's next poll
//! imports them, as a drop on the window would.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use rune::Waker;

static WAITING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());
static WAKER: OnceLock<Waker> = OnceLock::new();

/// The files this start was handed: every argument names a file or folder that exists. A start
/// with any flag, or with an argument that is no file, was not a drop, and hands nothing.
pub fn from_args() -> Vec<PathBuf> {
    files_in(std::env::args_os().skip(1).map(PathBuf::from).collect())
}

fn files_in(args: Vec<PathBuf>) -> Vec<PathBuf> {
    let all_files = !args.is_empty() && args.iter().all(|arg| !arg.to_string_lossy().starts_with("--") && arg.exists());
    if all_files { args } else { Vec::new() }
}

/// The files a second start sent after the running copy answered it, one `open<TAB>path` line
/// each.
pub fn heard(reader: impl std::io::BufRead) -> Vec<PathBuf> {
    reader.lines().map_while(Result::ok).filter_map(|line| line.strip_prefix("open\t").map(PathBuf::from)).collect()
}

/// The lines a second start sends for its files.
pub fn told(paths: &[PathBuf]) -> String {
    paths.iter().map(|path| format!("open\t{}\n", path.display())).collect()
}

/// Keeps files for the window and wakes it.
pub fn hand(paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    if let Ok(mut waiting) = WAITING.lock() {
        waiting.extend(paths);
    }
    if let Some(waker) = WAKER.get() {
        waker();
    }
}

/// The files waiting, taken.
pub fn take() -> Vec<PathBuf> {
    WAITING.lock().map(|mut waiting| std::mem::take(&mut *waiting)).unwrap_or_default()
}

/// The window's waker, so a file handed over later wakes it at once.
pub fn set_waker(waker: Waker) {
    let _ = WAKER.set(waker);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_start_hands_over_only_when_every_argument_is_a_file() {
        let here = std::env::current_dir().unwrap_or_default();
        assert_eq!(files_in(vec![here.clone()]), vec![here.clone()], "a folder dropped on the shortcut");
        assert!(files_in(vec![PathBuf::from("--practice")]).is_empty(), "a flag is no drop");
        assert!(files_in(vec![here, PathBuf::from("no-such-file.xyz")]).is_empty(), "nor is a name that is no file");
        assert!(files_in(Vec::new()).is_empty());
    }

    #[test]
    fn the_files_a_second_start_tells_are_the_ones_heard() {
        let files = vec![PathBuf::from(r"C:\Songs\Morning.xml"), PathBuf::from("D:/pictures/sky one.png")];
        let lines = format!("{}show\nnoise\n", told(&files));
        assert_eq!(heard(std::io::Cursor::new(lines)), files);
    }
}
