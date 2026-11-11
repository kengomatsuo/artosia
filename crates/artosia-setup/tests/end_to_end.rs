//! The whole setup on this PC, inside a test root: a local HTTP server over a signed feed, an
//! install into an empty folder, the files, both shortcuts, the registry values, the first
//! start's confirmation, a self-update that must leave the setup copy alone, then an uninstall
//! that must leave the church's data folder byte for byte.
//!
//! Nothing here touches the real install folder, desktop, Start menu or the `Artosia` key.
#![cfg(windows)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use artosia_setup::entry::{self, Value, uninstall_values};
use artosia_setup::install::{self, Kind, Step};
use artosia_setup::places::{APP_EXE, SETUP_EXE};
use artosia_setup::shortcut;
use artosia_update::manifest::{self, MANIFEST_NAME, TrustedKeys};
use artosia_update::publish::{ReleaseSpec, build_release};
use artosia_update::{Applied, apply_on_start_with, check_with, confirm_started, download_with, staging_dir};
use common::{Root, Server};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::Ordering;

const SEED: [u8; 32] = [7; 32];

fn test_keys() -> TrustedKeys {
    TrustedKeys::from_keys(vec![manifest::public_key_of(&SEED).unwrap()])
}

fn blob(path: &Path, tag: u8, len: usize) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, (0..len).map(|i| (i as u8).wrapping_add(tag)).collect::<Vec<_>>()).unwrap();
}

/// A signed release of stand-in programs, in `<root>/feed-<version>`.
fn make_feed(root: &Root, version: &str, tag: u8) -> PathBuf {
    let built = root.dir.join(format!("built-{version}"));
    let out = root.dir.join(format!("feed-{version}"));
    blob(&built.join(APP_EXE), tag, 300_000);
    blob(&built.join("artosia-output.exe"), tag, 200_000);
    blob(&built.join("lib").join("data.bin"), tag, 100_000);
    // A name long enough that, under a padded root, its full path passes 260 characters.
    blob(&built.join("lib").join(format!("{}.bin", "n".repeat(60))), tag, 50_000);
    build_release(&ReleaseSpec { dir: &built, version, notes: "stand-in", platform: None, date: Some("2026-11-12"), out: &out }, &SEED).unwrap();
    out
}

/// A stand-in for the downloaded setup file; it is copied into the install folder.
fn fake_setup(root: &Root) -> PathBuf {
    let path = root.dir.join("downloaded-setup.exe");
    fs::write(&path, b"setup stand-in").unwrap();
    path
}

fn text(places: &artosia_setup::places::Places, name: &str) -> String {
    match entry::read(places, name) {
        Some(Value::Text(text)) => text,
        other => panic!("{name} is {other:?}"),
    }
}

fn assert_clean(root: &Root) {
    let p = &root.places;
    assert!(!p.install.exists(), "install folder is gone: {:?}", p.install);
    assert!(!p.start_lnk().exists() && !p.desktop_lnk().exists(), "both shortcuts are gone");
    assert!(entry::read(p, "DisplayName").is_none(), "registry key is gone");
}

/// Install, check, first start, uninstall, check. `version` is what the feed publishes.
fn full_cycle(root: &Root, feed: &str, keys: &TrustedKeys, version: &str) {
    let p = &root.places;
    let before = root.data_snapshot();
    assert!(!before.is_empty());
    let setup = fake_setup(root);

    let steps = Mutex::new(Vec::new());
    let done = install::install(p, feed, keys, &setup, &|step| steps.lock().unwrap().push(step)).unwrap();
    assert!(done.downloaded);
    assert_eq!(done.version, version);
    let steps = steps.into_inner().unwrap();
    assert_eq!(steps.first(), Some(&Step::Checking));
    assert!(matches!(steps.last(), Some(Step::SettingUp)));
    assert!(steps.iter().any(|s| matches!(s, Step::Downloading { done, total } if done == total && *total > 0)), "progress reached its total");

    // The files are in place.
    assert!(p.app_exe().is_file());
    assert!(!p.install.join("artosia.exe.artosia-old").exists());
    assert!(p.setup_exe().is_file(), "the setup copy is kept for uninstalling");
    assert_eq!(fs::read(p.setup_exe()).unwrap(), b"setup stand-in");

    // Both shortcuts: target, working folder and icon.
    for lnk in [p.start_lnk(), p.desktop_lnk()] {
        let (target, work, icon) = shortcut::read(&lnk).unwrap();
        assert_eq!(target, p.app_exe(), "{lnk:?}");
        assert_eq!(work, p.install);
        assert_eq!(icon, p.app_exe());
    }

    // Every registry value, and the DWORDs are DWORDs.
    let size_kb = match entry::read(p, "EstimatedSize") {
        Some(Value::Dword(kb)) => kb,
        other => panic!("EstimatedSize must be a DWORD, got {other:?}"),
    };
    assert!(size_kb > 100, "{size_kb}");
    for (name, value) in uninstall_values(p, version, u64::from(size_kb) * 1024) {
        assert_eq!(entry::read(p, name), Some(value), "{name}");
    }
    assert_eq!(text(p, "UninstallString"), format!("\"{}\" --uninstall", p.setup_exe().display()));
    assert_eq!(text(p, "DisplayVersion"), version);

    // The first start of artosia.exe: its `apply_on_start` counts the start, `confirm_started`
    // makes the install permanent and clears the journal.
    let state = staging_dir(&p.install).join("state.json");
    assert!(state.is_file(), "the journal waits for the first start");
    assert_eq!(apply_on_start_with(&p.install, keys).unwrap(), Applied::Nothing);
    confirm_started(&p.install).unwrap();
    assert!(!state.exists(), "the first start confirmed the install");

    // Uninstall.
    install::uninstall(p, &setup).unwrap();
    assert_clean(root);
    assert_eq!(root.data_snapshot(), before, "the church's data is byte for byte as it was");
}

#[test]
fn install_then_uninstall_with_a_signed_feed_leaves_the_data_alone() {
    let root = Root::new("cycle", 0);
    let feed = make_feed(&root, "2.0.0", 3);
    let server = Server::start(feed);
    full_cycle(&root, &server.url(), &test_keys(), "2.0.0");
}

#[test]
fn install_then_uninstall_on_the_real_release_feed() {
    // The real signed feed of the newest release, with the compiled-in key.
    let Some(feed) = real_feed() else {
        eprintln!("skipped: no dist/<version>/feed on this checkout");
        return;
    };
    let root = Root::new("real", 0);
    let version = newest_version(&feed);
    let manifest_text = fs::read_to_string(feed.join(MANIFEST_NAME)).unwrap();
    assert!(!manifest_text.contains(SETUP_EXE), "no release lists the setup exe, so no update touches the copy");
    let server = Server::start(feed);
    full_cycle(&root, &server.url(), &TrustedKeys::compiled(), &version);
}

fn real_feed() -> Option<PathBuf> {
    let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dist");
    let mut versions: Vec<(semver_like::Version, PathBuf)> = fs::read_dir(dist)
        .ok()?
        .flatten()
        .filter_map(|e| Some((semver_like::Version::parse(&e.file_name().to_string_lossy())?, e.path().join("feed"))))
        .filter(|(_, feed)| feed.join(MANIFEST_NAME).is_file())
        .collect();
    versions.sort();
    versions.pop().map(|(_, feed)| feed)
}

fn newest_version(feed: &Path) -> String {
    let json: serde_json::Value = serde_json::from_slice(&fs::read(feed.join(MANIFEST_NAME)).unwrap()).unwrap();
    json["version"].as_str().unwrap().to_string()
}

/// Just enough of a version to order folder names like 0.1.9 and 0.1.13.
mod semver_like {
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    pub struct Version(u64, u64, u64);

    impl Version {
        pub fn parse(text: &str) -> Option<Version> {
            let mut parts = text.split('.').map(|part| part.parse::<u64>().ok());
            Some(Version(parts.next()??, parts.next()??, parts.next()??))
        }
    }
}

#[test]
fn a_self_update_leaves_the_setup_copy_alone_and_a_long_root_works() {
    // Padding that puts the program under 260 characters and `lib/<long name>` over it.
    let root = Root::new("update", 140);
    let (keys, setup) = (test_keys(), fake_setup(&root));
    let first = Server::start(make_feed(&root, "2.0.0", 3));
    install::install(&root.places, &first.url(), &keys, &setup, &|_| {}).unwrap();
    let long = root.places.install.join("lib").join(format!("{}.bin", "n".repeat(60)));
    assert!(long.as_os_str().len() > 260 && long.is_file(), "a path over 260 characters was written");
    confirm_first_start(&root, &keys);
    let setup_bytes = fs::read(root.places.setup_exe()).unwrap();

    // The app updates itself to 3.0.0 the way `update.rs` does: check, download, apply.
    let second = Server::start(make_feed(&root, "3.0.0", 9));
    let release = check_with(&second.url(), "2.0.0", &keys).unwrap().unwrap();
    download_with(&release, &staging_dir(&root.places.install), &keys, &|_, _| {}).unwrap();
    assert!(matches!(apply_on_start_with(&root.places.install, &keys).unwrap(), Applied::Updated { .. }));
    assert_eq!(fs::read(root.places.app_exe()).unwrap()[0], 9, "the program is the new one");
    confirm_started(&root.places.install).unwrap();
    assert_eq!(fs::read(root.places.setup_exe()).unwrap(), setup_bytes, "the update left the setup copy alone");

    // The installed-apps list still says 2.0.0 until the app asks the kept setup to refresh it.
    // This is the call `artosia.exe` makes after the first confirmed start of 3.0.0.
    assert_eq!(text(&root.places, "DisplayVersion"), "2.0.0");
    let status = refresh_with_the_binary(&root, Some("3.0.0"));
    assert!(status.success());
    assert_eq!(text(&root.places, "DisplayVersion"), "3.0.0", "the entry follows the self-update");
    assert_eq!(text(&root.places, "UninstallString"), format!("\"{}\" --uninstall", root.places.setup_exe().display()));

    // Uninstall still works after the update.
    install::uninstall(&root.places, &setup).unwrap();
    assert_clean(&root);
}

fn confirm_first_start(root: &Root, keys: &TrustedKeys) {
    assert_eq!(apply_on_start_with(&root.places.install, keys).unwrap(), Applied::Nothing);
    confirm_started(&root.places.install).unwrap();
}

#[test]
fn a_failed_install_leaves_no_shortcut_and_no_entry_and_a_second_try_resumes() {
    let root = Root::new("fail", 0);
    let (keys, setup) = (test_keys(), fake_setup(&root));
    let server = Server::start(make_feed(&root, "2.0.0", 3));
    // Every data connection is cut off half way: three tries per file, then the updater gives up.
    server.cut.store(100, Ordering::SeqCst);
    let failed = install::install(&root.places, &server.url(), &keys, &setup, &|_| {}).unwrap_err();
    assert_eq!(failed.kind, Kind::Network, "{}", failed.detail);
    assert!(!root.places.start_lnk().exists() && !root.places.desktop_lnk().exists());
    assert!(entry::read(&root.places, "DisplayName").is_none());
    assert!(!root.places.app_exe().exists(), "nothing was applied");
    let kept: u64 = fs::read_dir(staging_dir(&root.places.install).join("files")).unwrap().flatten().map(|e| e.metadata().unwrap().len()).sum();
    assert!(kept > 0, "the half files are kept");

    server.cut.store(0, Ordering::SeqCst);
    install::install(&root.places, &server.url(), &keys, &setup, &|_| {}).unwrap();
    assert!(root.places.app_exe().is_file());
    install::uninstall(&root.places, &setup).unwrap();
    assert_clean(&root);
}

#[test]
fn a_tampered_file_is_refused_and_nothing_is_written() {
    let root = Root::new("tamper", 0);
    let (keys, setup) = (test_keys(), fake_setup(&root));
    let feed = make_feed(&root, "2.0.0", 3);
    let target = feed.join("artosia-output.exe");
    let mut bytes = fs::read(&target).unwrap();
    bytes[1000] ^= 0xff;
    fs::write(&target, bytes).unwrap();
    let server = Server::start(feed);
    let failed = install::install(&root.places, &server.url(), &keys, &setup, &|_| {}).unwrap_err();
    assert_eq!(failed.kind, Kind::Damaged, "{}", failed.detail);
    assert!(!root.places.app_exe().exists());
    assert!(!root.places.start_lnk().exists() && !root.places.desktop_lnk().exists());
    assert!(entry::read(&root.places, "DisplayName").is_none());
}

#[test]
fn offline_is_a_network_failure_with_nothing_written() {
    let root = Root::new("offline", 0);
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let failed = install::install(&root.places, &format!("http://127.0.0.1:{port}/feed"), &test_keys(), &fake_setup(&root), &|_| {}).unwrap_err();
    assert_eq!(failed.kind, Kind::Network);
    assert!(entry::read(&root.places, "DisplayName").is_none());
}

#[test]
fn running_again_on_an_installed_copy_downloads_nothing_and_rewrites_the_shortcuts() {
    let root = Root::new("again", 0);
    let (keys, setup) = (test_keys(), fake_setup(&root));
    {
        let server = Server::start(make_feed(&root, "2.0.0", 3));
        install::install(&root.places, &server.url(), &keys, &setup, &|_| {}).unwrap();
    }
    fs::remove_file(root.places.desktop_lnk()).unwrap();
    let _ = entry::remove(&root.places);
    // The feed is gone: only the shortcuts and the entry can be written.
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let done = install::install(&root.places, &format!("http://127.0.0.1:{port}/feed"), &keys, &setup, &|_| {}).unwrap();
    assert!(!done.downloaded);
    assert!(root.places.desktop_lnk().is_file() && root.places.start_lnk().is_file());
    assert_eq!(text(&root.places, "DisplayName"), "Artosia");
    install::uninstall(&root.places, &setup).unwrap();
    assert_clean(&root);
}

#[test]
fn a_file_that_will_not_delete_stops_the_uninstall_and_keeps_the_entry_for_a_second_run() {
    use std::os::windows::fs::OpenOptionsExt;
    let root = Root::new("locked", 0);
    let (keys, setup) = (test_keys(), fake_setup(&root));
    let server = Server::start(make_feed(&root, "2.0.0", 3));
    install::install(&root.places, &server.url(), &keys, &setup, &|_| {}).unwrap();
    let before = root.data_snapshot();
    // Held open with no sharing, as an antivirus scan or a running program would.
    let held = fs::OpenOptions::new().read(true).share_mode(0).open(root.places.install.join("lib").join("data.bin")).unwrap();
    let failed = install::uninstall(&root.places, &setup).unwrap_err();
    assert_eq!(failed.kind, Kind::Disk);
    assert!(entry::read(&root.places, "DisplayName").is_some(), "the entry stays until every file is gone");
    assert_eq!(root.data_snapshot(), before);
    drop(held);
    install::uninstall(&root.places, &setup).unwrap();
    assert_clean(&root);
}

#[test]
fn uninstall_never_enters_a_data_folder_that_sits_inside_the_install_folder() {
    let mut root = Root::new("inside", 0);
    let (keys, setup) = (test_keys(), fake_setup(&root));
    let server = Server::start(make_feed(&root, "2.0.0", 3));
    install::install(&root.places, &server.url(), &keys, &setup, &|_| {}).unwrap();
    // ARTOSIA_DATA pointed inside the install folder.
    let inside = root.places.install.join("mine").join("library");
    fs::create_dir_all(&inside).unwrap();
    fs::write(inside.join("library.db"), b"the church").unwrap();
    root.places.data.push(inside.clone());
    install::uninstall(&root.places, &setup).unwrap();
    assert_eq!(fs::read(inside.join("library.db")).unwrap(), b"the church");
    assert!(!root.places.app_exe().exists() && !root.places.start_lnk().exists());
    assert!(entry::read(&root.places, "DisplayName").is_none());
}

#[test]
fn a_running_program_is_noticed_and_never_killed() {
    let root = Root::new("running", 0);
    let install_dir = &root.places.install;
    fs::create_dir_all(install_dir).unwrap();
    let system = std::env::var_os("SystemRoot").map(PathBuf::from).unwrap().join("System32").join("ping.exe");
    let copy = install_dir.join("artosia-output.exe");
    fs::copy(system, &copy).unwrap();
    let mut child = std::process::Command::new(&copy).args(["-n", "30", "127.0.0.1"]).stdout(std::process::Stdio::null()).spawn().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let me = fake_setup(&root);
    assert_eq!(install::running_programs(&root.places, &me), vec!["artosia-output.exe".to_string()]);
    assert!(child.try_wait().unwrap().is_none(), "the setup did not touch the running program");
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(install::running_programs(&root.places, &me).is_empty());
}

#[test]
fn a_folder_too_long_for_a_shortcut_fails_plainly_and_leaves_no_entry() {
    let root = Root::new("toolong", 300);
    let server = Server::start(make_feed(&root, "2.0.0", 3));
    let failed = install::install(&root.places, &server.url(), &test_keys(), &fake_setup(&root), &|_| {}).unwrap_err();
    assert_eq!(failed.kind, Kind::Shortcuts, "{}", failed.detail);
    assert!(entry::read(&root.places, "DisplayName").is_none());
    assert!(!root.places.start_lnk().exists() && !root.places.desktop_lnk().exists());
}

/// Runs the real `Artosia-setup.exe --refresh [version]` inside the test root.
fn refresh_with_the_binary(root: &Root, version: Option<&str>) -> std::process::ExitStatus {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_Artosia-setup"));
    command.arg("--refresh").env("ARTOSIA_SETUP_ROOT", &root.dir);
    command.args(version);
    command.status().unwrap()
}

#[test]
fn refresh_does_nothing_when_there_is_no_entry_and_nothing_is_created() {
    let root = Root::new("refresh-none", 0);
    // A copy unzipped by hand: the program is there, no setup copy, no entry.
    fs::create_dir_all(&root.places.install).unwrap();
    fs::write(root.places.app_exe(), b"program").unwrap();
    assert!(refresh_with_the_binary(&root, Some("9.9.9")).success());
    assert!(entry::read(&root.places, "DisplayName").is_none(), "no entry was made");
    assert!(!root.places.setup_exe().exists() && !root.places.start_lnk().exists() && !root.places.desktop_lnk().exists());
    assert!(!install::refresh_entry(&root.places, Some("9.9.9")).unwrap());
}
