//! Shared by the end-to-end tests: a tiny static server over a feed folder (with Range support
//! and a way to cut connections, copied in shape from `artosia-update/tests/end_to_end.rs`) and
//! a test root whose folders and registry key are removed when it drops.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use artosia_setup::places::Places;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub struct Server {
    pub addr: String,
    pub requests: Arc<Mutex<Vec<String>>>,
    /// How many of the next data connections to cut off half way.
    pub cut: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Server {
    pub fn start(root: PathBuf) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let cut = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (log, cutting, halt) = (requests.clone(), cut.clone(), stop.clone());
        let handle = std::thread::spawn(move || {
            for conn in listener.incoming() {
                if halt.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(conn) = conn {
                    let (root, log, cutting) = (root.clone(), log.clone(), cutting.clone());
                    std::thread::spawn(move || serve(conn, &root, &log, &cutting));
                }
            }
        });
        Server { addr, requests, cut, stop, handle: Some(handle) }
    }

    pub fn url(&self) -> String {
        format!("http://{}/feed", self.addr)
    }

    pub fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(&self.addr);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn serve(mut conn: TcpStream, root: &Path, log: &Mutex<Vec<String>>, cut: &AtomicUsize) {
    let mut reader = BufReader::new(conn.try_clone().unwrap());
    let mut first = String::new();
    if reader.read_line(&mut first).is_err() {
        return;
    }
    let target = first.split_whitespace().nth(1).unwrap_or("/").to_string();
    let name = target.rsplit('/').next().unwrap_or("").to_string();
    let mut range = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
            range = value.trim().trim_end_matches('-').parse::<u64>().ok();
        }
    }
    log.lock().unwrap().push(name.clone());
    let Ok(body) = fs::read(root.join(&name)) else {
        let _ = conn.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    };
    let (status, slice) = match range {
        Some(from) if (from as usize) < body.len() => (
            format!("206 Partial Content\r\nContent-Range: bytes {from}-{}/{}", body.len() - 1, body.len()),
            &body[from as usize..],
        ),
        _ => ("200 OK".to_string(), &body[..]),
    };
    let _ = conn.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", slice.len()).as_bytes());
    let data = !name.ends_with(".json") && !name.ends_with(".sig");
    if data && cut.try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1)).is_ok() {
        let _ = conn.write_all(&slice[..slice.len() / 2]);
        let _ = conn.flush();
        let _ = conn.shutdown(std::net::Shutdown::Both);
    } else {
        let _ = conn.write_all(slice);
    }
}

/// A test root under the temp folder, with a non-ASCII name on purpose (rustup #5035), and the
/// church's data folder with a few files in it. Drops everything, registry key included.
pub struct Root {
    pub dir: PathBuf,
    pub places: Places,
}

impl Root {
    pub fn new(name: &str, padding: usize) -> Root {
        let mut dir = std::env::temp_dir().join(format!("artosia-setup-test-\u{e9}\u{65e5}-{name}-{}", std::process::id()));
        // A folder name holds 255 characters at most, so padding is several folders.
        let mut left = padding;
        while left > 0 {
            let part = left.min(100);
            dir = dir.join("x".repeat(part));
            left -= part;
        }
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let places = Places::under(&dir);
        for (path, bytes) in church_files() {
            let path = places.data[0].join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        Root { dir, places }
    }

    /// Every file under the data folder with its bytes, to compare before and after.
    pub fn data_snapshot(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(folder: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            let Ok(entries) = fs::read_dir(folder) else { return };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else {
                    let bytes = fs::read(&path).unwrap();
                    out.push((path, bytes));
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.places.data[0], &mut out);
        out.sort();
        out
    }
}

/// Stand-ins for a church's files: never real songs.
fn church_files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("library.db", (0..50_000).map(|i| (i * 7 % 253) as u8).collect()),
        ("bibles/test.sqlite", vec![1, 2, 3, 4, 5]),
        ("recordings/2026-11-01.log", b"practice words, not a real service".to_vec()),
        ("settings.json", b"{\"practice\":true}".to_vec()),
    ]
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = artosia_setup::entry::remove(&self.places);
        // Climb past the padding folders to the per-test folder.
        let mut top = self.dir.clone();
        while !top.file_name().is_some_and(|n| n.to_string_lossy().starts_with("artosia-setup-test-")) {
            top = top.parent().unwrap().to_path_buf();
        }
        let _ = fs::remove_dir_all(&top);
    }
}
