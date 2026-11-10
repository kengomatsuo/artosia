//! The whole update path on this PC: a local HTTP server, a fake install folder with dummy
//! programs (two of them running while the update lands), and every refusal and rollback.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use artosia_update::manifest::{self, TrustedKeys, sha256_hex_of_file};
use artosia_update::publish::{ReleaseSpec, build_release};
use artosia_update::{
    Applied, RollbackReason, UpdateError, apply_on_start_with, check_with, confirm_started,
    download_with, roll_back, staging_dir,
};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

const SEED: [u8; 32] = [7; 32];
const OTHER_SEED: [u8; 32] = [9; 32];

// ---- a tiny static file server with Range support and a way to cut connections ----

type RequestLog = Arc<Mutex<Vec<(String, Option<u64>)>>>;

struct Server {
    addr: String,
    log: RequestLog,
    cut: Arc<AtomicUsize>,
    ignore_range: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Server {
    fn start(root: PathBuf) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let log = Arc::new(Mutex::new(Vec::new()));
        let cut = Arc::new(AtomicUsize::new(0));
        let ignore_range = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (l, c, i, s) = (log.clone(), cut.clone(), ignore_range.clone(), stop.clone());
        let handle = std::thread::spawn(move || {
            for conn in listener.incoming() {
                if s.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(conn) = conn {
                    let (root, l, c, i) = (root.clone(), l.clone(), c.clone(), i.clone());
                    std::thread::spawn(move || serve(conn, &root, &l, &c, &i));
                }
            }
        });
        Server {
            addr,
            log,
            cut,
            ignore_range,
            stop,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("http://{}/feed", self.addr)
    }

    fn requests_for(&self, name: &str) -> Vec<Option<u64>> {
        let log = self.log.lock().unwrap();
        log.iter()
            .filter(|(p, _)| p == name)
            .map(|(_, r)| *r)
            .collect()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(&self.addr);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn serve(
    mut conn: TcpStream,
    root: &Path,
    log: &Mutex<Vec<(String, Option<u64>)>>,
    cut: &AtomicUsize,
    ignore_range: &AtomicBool,
) {
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
        if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
            range = v.trim().trim_end_matches('-').parse::<u64>().ok();
        }
    }
    log.lock().unwrap().push((name.clone(), range));
    let Ok(body) = fs::read(root.join(&name)) else {
        let _ = conn
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    };
    let data_file = !name.ends_with(".json") && !name.ends_with(".sig");
    let start = if ignore_range.load(Ordering::SeqCst) {
        None
    } else {
        range
    };
    let (status, slice) = match start {
        Some(n) if (n as usize) < body.len() => (
            format!(
                "206 Partial Content\r\nContent-Range: bytes {n}-{}/{}",
                body.len() - 1,
                body.len()
            ),
            &body[n as usize..],
        ),
        _ => ("200 OK".to_string(), &body[..]),
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        slice.len()
    );
    let _ = conn.write_all(head.as_bytes());
    let cutting = data_file
        && cut
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok();
    if cutting {
        let _ = conn.write_all(&slice[..slice.len() / 2]);
        let _ = conn.flush();
        let _ = conn.shutdown(std::net::Shutdown::Both);
    } else {
        let _ = conn.write_all(slice);
    }
}

// ---- the fake world ----

fn dummy() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let dir = exe.parent().unwrap().parent().unwrap().join("examples");
    dir.join(format!("dummy{}", std::env::consts::EXE_SUFFIX))
}

/// A real, runnable program: the dummy's bytes plus a tag and a patterned tail.
fn make_exe(path: &Path, tag: &str, pad: usize) {
    let mut bytes = fs::read(dummy()).expect("build the dummy example first");
    bytes.extend_from_slice(tag.as_bytes());
    bytes.extend((0..pad).map(|i| (i * 31 % 251) as u8));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

fn blob(path: &Path, tag: u8, len: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        (0..len)
            .map(|i| (i as u8).wrapping_add(tag))
            .collect::<Vec<_>>(),
    )
    .unwrap();
}

struct World {
    root: PathBuf,
    install: PathBuf,
    out: PathBuf,
    keys: TrustedKeys,
    server: Server,
}

impl World {
    fn new(name: &str) -> World {
        let root =
            std::env::temp_dir().join(format!("artosia-update-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (install, built, out) = (root.join("install"), root.join("built"), root.join("out"));
        make_exe(&install.join("artosia.exe"), "v1-operator", 200_000);
        make_exe(&install.join("artosia-output.exe"), "v1-output", 200_000);
        blob(&install.join("lib/data.bin"), 1, 100_000);
        blob(&install.join("shared.dll"), 5, 150_000);
        make_exe(&built.join("artosia.exe"), "v2-operator", 300_000);
        make_exe(&built.join("artosia-output.exe"), "v2-output", 300_000);
        blob(&built.join("lib/data.bin"), 2, 120_000);
        blob(&built.join("shared.dll"), 5, 150_000);
        build_release(
            &ReleaseSpec {
                dir: &built,
                version: "2.0.0",
                notes: "New songs page",
                platform: None,
                date: Some("2026-11-10"),
                out: &out,
            },
            &SEED,
        )
        .unwrap();
        let keys = TrustedKeys::from_keys(vec![manifest::public_key_of(&SEED).unwrap()]);
        let server = Server::start(out.clone());
        World {
            root,
            install,
            out,
            keys,
            server,
        }
    }

    fn stage(&self) -> PathBuf {
        staging_dir(&self.install)
    }

    fn read(&self, rel: &str) -> Vec<u8> {
        fs::read(self.install.join(rel)).unwrap()
    }

    fn is_v1(&self) -> bool {
        self.read("artosia.exe")
            .windows(11)
            .any(|w| w == b"v1-operator")
    }

    fn is_v2(&self) -> bool {
        self.read("artosia.exe")
            .windows(11)
            .any(|w| w == b"v2-operator")
            && self
                .read("artosia-output.exe")
                .windows(9)
                .any(|w| w == b"v2-output")
    }

    fn old_files_left(&self) -> Vec<PathBuf> {
        fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
            for e in fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, found);
                } else if p.to_string_lossy().ends_with(".artosia-old") {
                    found.push(p);
                }
            }
        }
        let mut found = Vec::new();
        walk(&self.install, &mut found);
        found
    }

    fn download(&self) -> Result<artosia_update::Staged, UpdateError> {
        let release =
            check_with(&self.server.url(), "1.0.0", &self.keys)?.expect("a newer release");
        download_with(&release, &self.stage(), &self.keys, &|_, _| {})
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Kills only the dummy programs this test started.
struct Running(Vec<Child>);

impl Running {
    fn start(install: &Path) -> Running {
        let spawn = |name: &str| {
            Command::new(install.join(name))
                .arg("60000")
                .stdout(Stdio::null())
                .spawn()
                .unwrap()
        };
        Running(vec![spawn("artosia.exe"), spawn("artosia-output.exe")])
    }

    fn all_alive(&mut self) -> bool {
        self.0.iter_mut().all(|c| c.try_wait().unwrap().is_none())
    }

    fn stop(&mut self) {
        for c in &mut self.0 {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop();
    }
}

// ---- the tests ----

#[test]
fn update_lands_while_both_programs_run_and_old_files_go_after_restart() {
    let w = World::new("happy");
    assert!(w.is_v1());
    assert!(
        check_with(&w.server.url(), "2.0.0", &w.keys)
            .unwrap()
            .is_none(),
        "current build is up to date"
    );
    assert!(
        check_with(&w.server.url(), "3.0.0", &w.keys)
            .unwrap()
            .is_none(),
        "an older signed manifest is not an update"
    );
    let release = check_with(&w.server.url(), "1.0.0", &w.keys)
        .unwrap()
        .unwrap();
    assert_eq!(
        (release.version.as_str(), release.notes.as_str()),
        ("2.0.0", "New songs page")
    );

    let last = Mutex::new((0u64, 0u64));
    let staged = download_with(&release, &w.stage(), &w.keys, &|d, t| {
        *last.lock().unwrap() = (d, t)
    })
    .unwrap();
    assert_eq!(staged.version, "2.0.0");
    let (done, total) = *last.lock().unwrap();
    assert_eq!(done, total);
    assert!(
        total < release.size,
        "the unchanged shared.dll is not downloaded"
    );
    assert!(w.server.requests_for("shared.dll").is_empty());
    assert!(
        w.is_v1(),
        "downloading changes nothing in the install folder"
    );

    let mut running = Running::start(&w.install);
    let applied = apply_on_start_with(&w.install, &w.keys).unwrap();
    assert!(
        matches!(&applied, Applied::Updated { version, relaunch: None } if version == "2.0.0"),
        "{applied:?}"
    );
    assert!(w.is_v2());
    assert_eq!(w.read("lib/data.bin")[0], 2);
    assert!(
        running.all_alive(),
        "the running programs were not disturbed"
    );
    assert!(w.install.join("artosia.exe.artosia-old").exists());

    // The new build starts, then reports that it works; the old programs are still running.
    assert_eq!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Nothing
    );
    confirm_started(&w.install).unwrap();
    running.stop();
    // The next start sweeps whatever Windows kept locked while the old programs ran.
    assert_eq!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Nothing
    );
    assert!(w.old_files_left().is_empty(), "{:?}", w.old_files_left());
    assert!(!w.stage().join("state.json").exists());
    assert_eq!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Nothing
    );
    assert!(w.is_v2());
}

#[test]
fn a_tampered_file_is_refused_and_nothing_is_installed() {
    let w = World::new("tamper");
    let target = w.out.join("artosia-output.exe");
    let mut bytes = fs::read(&target).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xff;
    fs::write(&target, bytes).unwrap();
    let err = w.download().unwrap_err();
    assert!(matches!(err, UpdateError::BadFile(_)), "{err:?}");
    assert_eq!(err.to_string(), "A downloaded file is damaged");
    assert!(!w.stage().join(manifest::SIGNATURE_NAME).exists());
    assert_eq!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Nothing
    );
    assert!(w.is_v1());
}

#[test]
fn a_tampered_staged_file_is_refused_at_apply() {
    let w = World::new("tamper-staged");
    w.download().unwrap();
    let staged = w.stage().join("files").join("artosia.exe");
    let mut bytes = fs::read(&staged).unwrap();
    bytes[10] ^= 1;
    fs::write(&staged, bytes).unwrap();
    assert!(matches!(
        apply_on_start_with(&w.install, &w.keys),
        Err(UpdateError::BadFile(_))
    ));
    assert!(w.is_v1());
    assert!(w.old_files_left().is_empty());
}

#[test]
fn a_wrong_signature_is_refused() {
    let w = World::new("sig");
    // Signed by someone else.
    let other = TrustedKeys::from_keys(vec![manifest::public_key_of(&OTHER_SEED).unwrap()]);
    let err = check_with(&w.server.url(), "1.0.0", &other).unwrap_err();
    assert!(matches!(err, UpdateError::BadSignature));
    assert_eq!(err.to_string(), "This update is not from Artosia");
    // The manifest edited after signing.
    let path = w.out.join(manifest::MANIFEST_NAME);
    let edited = fs::read_to_string(&path)
        .unwrap()
        .replace("New songs page", "Free gift");
    fs::write(&path, edited).unwrap();
    assert!(matches!(
        check_with(&w.server.url(), "1.0.0", &w.keys),
        Err(UpdateError::BadSignature)
    ));
    // A build with no key compiled in refuses to update at all.
    let err = check_with(
        &w.server.url(),
        "1.0.0",
        &TrustedKeys::from_text(
            "# none\n0000000000000000000000000000000000000000000000000000000000000000",
        ),
    )
    .unwrap_err();
    assert!(matches!(err, UpdateError::NotSetUp));
    assert!(w.is_v1());
}

#[test]
fn a_cut_off_download_resumes_from_the_part_file() {
    let w = World::new("resume");
    w.server.cut.store(3, Ordering::SeqCst);
    let err = w.download().unwrap_err();
    assert!(matches!(err, UpdateError::Offline), "{err:?}");
    assert!(err.is_quiet());
    let part = w.stage().join("files").join("artosia-output.exe.part");
    let kept = fs::metadata(&part).unwrap().len();
    assert!(kept > 0, "the half file is kept");
    w.download().unwrap();
    let ranges = w.server.requests_for("artosia-output.exe");
    assert!(
        ranges.iter().any(|r| r.is_some_and(|n| n >= kept)),
        "resumed from byte {kept}: {ranges:?}"
    );
    assert!(matches!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Updated { .. }
    ));
    assert!(w.is_v2());
}

#[test]
fn a_server_that_ignores_range_restarts_the_file_cleanly() {
    let w = World::new("norange");
    w.server.cut.store(1, Ordering::SeqCst);
    w.server.ignore_range.store(true, Ordering::SeqCst);
    w.download().unwrap();
    assert!(matches!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Updated { .. }
    ));
    assert!(w.is_v2());
}

#[test]
fn asking_for_a_rollback_restores_the_old_files_and_remembers_the_bad_version() {
    let w = World::new("rollback");
    w.download().unwrap();
    let mut running = Running::start(&w.install);
    assert!(matches!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Updated { .. }
    ));
    assert!(w.is_v2());
    let back = roll_back(&w.install).unwrap();
    assert!(
        matches!(
            back,
            Applied::RolledBack {
                reason: RollbackReason::Requested,
                ..
            }
        ),
        "{back:?}"
    );
    assert!(w.is_v1());
    assert_eq!(w.read("lib/data.bin")[0], 1);
    assert!(running.all_alive());
    running.stop();
    assert_eq!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Nothing
    );
    assert!(w.old_files_left().is_empty());
    assert!(!w.install.join("artosia.exe.artosia-new").exists());
    // The same version is not offered again.
    let err = w.download().unwrap_err();
    assert!(matches!(err, UpdateError::Rejected));
    assert!(err.is_quiet());
}

#[test]
fn a_build_that_never_confirms_is_rolled_back_on_its_third_start() {
    let w = World::new("never");
    w.download().unwrap();
    assert!(matches!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Updated { .. }
    ));
    assert_eq!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Nothing
    );
    assert_eq!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Nothing
    );
    let back = apply_on_start_with(&w.install, &w.keys).unwrap();
    assert!(
        matches!(
            back,
            Applied::RolledBack {
                reason: RollbackReason::NeverStarted,
                ..
            }
        ),
        "{back:?}"
    );
    assert!(w.is_v1());
    assert!(matches!(w.download(), Err(UpdateError::Rejected)));
}

#[test]
fn a_swap_cut_short_by_a_power_failure_is_undone_on_the_next_start() {
    let w = World::new("crash");
    w.download().unwrap();
    assert!(matches!(
        apply_on_start_with(&w.install, &w.keys).unwrap(),
        Applied::Updated { .. }
    ));
    // Pretend the journal was never finished: a crash leaves the "applying" phase behind.
    let state = w.stage().join("state.json");
    let text = fs::read_to_string(&state)
        .unwrap()
        .replace("\"Trial\"", "\"Applying\"");
    fs::write(&state, text).unwrap();
    let back = apply_on_start_with(&w.install, &w.keys).unwrap();
    assert!(
        matches!(
            back,
            Applied::RolledBack {
                reason: RollbackReason::Interrupted,
                ..
            }
        ),
        "{back:?}"
    );
    assert!(w.is_v1());
    assert_eq!(w.read("lib/data.bin")[0], 1);
}

#[test]
fn being_offline_is_a_quiet_error() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let keys = TrustedKeys::from_keys(vec![manifest::public_key_of(&SEED).unwrap()]);
    let err = check_with(&format!("http://127.0.0.1:{port}/feed"), "1.0.0", &keys).unwrap_err();
    assert!(matches!(err, UpdateError::Offline), "{err:?}");
    assert!(err.is_quiet());
    assert!(err.to_string().split_whitespace().count() < 10);
}

#[test]
fn every_message_is_short_and_plain() {
    use UpdateError::*;
    let all = [
        NotSetUp,
        Offline,
        Slow,
        NotPublished,
        Server("x".into()),
        BadList("x".into()),
        BadSignature,
        WrongPlatform,
        BadFile("x".into()),
        Disk("x".into()),
        NothingStaged,
        StaleStaged,
        Rejected,
        InstallFailed("x".into()),
        RestoreFailed("x".into()),
        BadVersion("x".into()),
    ];
    for e in all {
        let text = e.to_string();
        assert!(text.split_whitespace().count() < 10, "{text}");
        assert!(!text.contains(['\u{2014}', '\u{b7}']), "{text}");
    }
}

#[test]
fn the_release_tool_makes_a_folder_the_app_accepts() {
    let root =
        std::env::temp_dir().join(format!("artosia-update-test-tool-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let tool = env!("CARGO_BIN_EXE_artosia-release");
    let key_file = root.join("private.key");
    let made = Command::new(tool)
        .arg("keygen")
        .arg(&key_file)
        .output()
        .unwrap();
    assert!(made.status.success());
    let public = String::from_utf8(made.stdout)
        .unwrap()
        .lines()
        .last()
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(public.len(), 64);
    // Refuses to overwrite an existing key.
    assert!(
        !Command::new(tool)
            .arg("keygen")
            .arg(&key_file)
            .output()
            .unwrap()
            .status
            .success()
    );

    let built = root.join("built");
    blob(&built.join("app.exe"), 3, 5_000);
    blob(&built.join("sub/x.dll"), 4, 6_000);
    let out = root.join("out");
    let run = |extra: &[&str]| {
        Command::new(tool)
            .args(["build", "--dir"])
            .arg(&built)
            .args(["--version", "1.2.3", "--notes", "Hello", "--out"])
            .arg(&out)
            .args(extra)
            .env_remove("ARTOSIA_UPDATE_KEY")
            .output()
            .unwrap()
    };
    assert!(!run(&[]).status.success(), "no key, no release");
    let ok = run(&["--key-file", key_file.to_str().unwrap()]);
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    assert!(out.join("sub__x.dll").is_file());

    let bytes = fs::read(out.join(manifest::MANIFEST_NAME)).unwrap();
    let sig = fs::read_to_string(out.join(manifest::SIGNATURE_NAME)).unwrap();
    let m = manifest::verify(&bytes, sig.trim(), &TrustedKeys::from_text(&public)).unwrap();
    assert_eq!(m.files.len(), 2);
    for f in &m.files {
        assert_eq!(sha256_hex_of_file(&out.join(&f.name)).unwrap(), f.sha256);
    }
    // The key can also come from the environment.
    let env_key = fs::read_to_string(&key_file).unwrap();
    let viaenv = Command::new(tool)
        .args(["build", "--dir"])
        .arg(&built)
        .args(["--version", "1.2.4", "--out"])
        .arg(root.join("out2"))
        .env("ARTOSIA_UPDATE_KEY", env_key.trim())
        .output()
        .unwrap();
    assert!(viaenv.status.success());
    let _ = fs::remove_dir_all(&root);
}
