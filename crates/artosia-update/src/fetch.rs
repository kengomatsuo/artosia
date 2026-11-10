//! Looking for a release and downloading it, with resume and file-level skipping.

use crate::apply::{self, rejected_versions};
use crate::error::UpdateError;
use crate::manifest::{
    self, FileEntry, MANIFEST_NAME, Manifest, SIGNATURE_NAME, TrustedKeys, sha256_hex_of_file,
    this_platform,
};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// A flaky line gets a few tries per file before the download gives up and keeps the part.
const ATTEMPTS: u32 = 3;
const RETRY_PAUSE: Duration = Duration::from_millis(300);
const CHECK_TIMEOUT: Duration = Duration::from_secs(8);
const READ_TIMEOUT: Duration = Duration::from_secs(20);

/// A newer release found on the feed. It carries the signed bytes so `download` re-checks them.
#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub released: String,
    /// Total size of every file in the release; `download` fetches less when files are unchanged.
    pub size: u64,
    pub(crate) base: String,
    pub(crate) manifest_bytes: Vec<u8>,
    pub(crate) signature: String,
}

/// An update that is fully downloaded, verified and waiting for the next start.
#[derive(Clone, Debug)]
pub struct Staged {
    pub version: String,
    pub dir: PathBuf,
}

fn agent(global: Option<Duration>) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_per_read(Some(READ_TIMEOUT))
        .timeout_global(global)
        .build()
        .into()
}

fn map_net(e: ureq::Error) -> UpdateError {
    match e {
        ureq::Error::StatusCode(404) => UpdateError::NotPublished,
        ureq::Error::StatusCode(code) => UpdateError::Server(format!("HTTP {code}")),
        ureq::Error::Timeout(_) => UpdateError::Slow,
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed | ureq::Error::Io(_) => {
            UpdateError::Offline
        }
        other => UpdateError::Server(other.to_string()),
    }
}

fn get_small(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, UpdateError> {
    let mut resp = agent.get(url).call().map_err(map_net)?;
    resp.body_mut()
        .with_config()
        .limit(2 * 1024 * 1024)
        .read_to_vec()
        .map_err(map_net)
}

/// Asks the feed for the latest release. `Ok(None)` means this build is current.
pub fn check(feed: &str, current: &str) -> Result<Option<Release>, UpdateError> {
    check_with(feed, current, &TrustedKeys::compiled())
}

pub fn check_with(
    feed: &str,
    current: &str,
    keys: &TrustedKeys,
) -> Result<Option<Release>, UpdateError> {
    if keys.is_empty() {
        return Err(UpdateError::NotSetUp);
    }
    let current_v = semver::Version::parse(current)
        .map_err(|e| UpdateError::BadVersion(format!("{current}: {e}")))?;
    let base = feed.trim_end_matches('/').to_string();
    let agent = agent(Some(CHECK_TIMEOUT));
    let manifest_bytes = get_small(&agent, &format!("{base}/{MANIFEST_NAME}"))?;
    let signature =
        String::from_utf8_lossy(&get_small(&agent, &format!("{base}/{SIGNATURE_NAME}"))?)
            .trim()
            .to_string();
    let m = manifest::verify(&manifest_bytes, &signature, keys)?;
    if m.platform != this_platform() {
        return Err(UpdateError::WrongPlatform);
    }
    let latest = semver::Version::parse(&m.version)
        .map_err(|e| UpdateError::BadVersion(format!("{}: {e}", m.version)))?;
    // A replayed older manifest is signed too, so only a strictly newer version counts.
    if latest <= current_v {
        return Ok(None);
    }
    Ok(Some(Release {
        size: m.files.iter().map(|f| f.size).sum(),
        version: m.version,
        notes: m.notes,
        released: m.released,
        base,
        manifest_bytes,
        signature,
    }))
}

/// Downloads into `into`, which must be `staging_dir(install_dir)`. Safe to call again after
/// a cut-off: finished files are kept and half files resume.
pub fn download(
    release: &Release,
    into: &Path,
    progress: &dyn Fn(u64, u64),
) -> Result<Staged, UpdateError> {
    download_with(release, into, &TrustedKeys::compiled(), progress)
}

pub fn download_with(
    release: &Release,
    into: &Path,
    keys: &TrustedKeys,
    progress: &dyn Fn(u64, u64),
) -> Result<Staged, UpdateError> {
    let m = manifest::verify(&release.manifest_bytes, &release.signature, keys)?;
    if rejected_versions(into).contains(&m.version) {
        return Err(UpdateError::Rejected);
    }
    let files_dir = into.join("files");
    fs::create_dir_all(&files_dir)?;
    // A fresh manifest is only written once everything is in place, so drop an older marker.
    let _ = fs::remove_file(into.join(SIGNATURE_NAME));
    let _ = fs::remove_file(into.join(MANIFEST_NAME));
    drop_unlisted(&files_dir, &m);

    let install_dir = into.parent().unwrap_or(into);
    let todo: Vec<&FileEntry> = m
        .files
        .iter()
        .filter(|f| !installed_matches(install_dir, f))
        .collect();
    let total: u64 = todo.iter().map(|f| f.size).sum();
    let mut before = 0u64;
    progress(0, total);
    let agent = agent(None);
    for entry in todo {
        let target = files_dir.join(&entry.name);
        let have = sha256_hex_of_file(&target).ok();
        if have.as_deref() != Some(&entry.sha256) {
            let _ = fs::remove_file(&target);
            let url = format!("{}/{}", release.base, entry.name);
            fetch_file(&agent, &url, entry, &target, &mut |len| {
                progress((before + len).min(total), total)
            })?;
        }
        before += entry.size;
        progress(before, total);
    }
    // Manifest first, signature last: the signature file is the "complete" marker.
    write_atomic(&into.join(MANIFEST_NAME), &release.manifest_bytes)?;
    write_atomic(&into.join(SIGNATURE_NAME), release.signature.as_bytes())?;
    Ok(Staged {
        version: m.version,
        dir: into.to_path_buf(),
    })
}

pub(crate) fn installed_matches(install_dir: &Path, f: &FileEntry) -> bool {
    let path = install_dir.join(&f.path);
    fs::metadata(&path).is_ok_and(|md| md.len() == f.size)
        && sha256_hex_of_file(&path).ok().as_deref() == Some(&f.sha256)
}

fn drop_unlisted(files_dir: &Path, m: &Manifest) {
    let Ok(entries) = fs::read_dir(files_dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let stem = name.strip_suffix(".part").unwrap_or(&name);
        if !m.files.iter().any(|f| f.name == stem) {
            let _ = fs::remove_file(e.path());
        }
    }
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), UpdateError> {
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    let mut f = File::create(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    apply::replace_file(&tmp, path)?;
    Ok(())
}

/// Downloads one file to `target` through `target.part`, resuming and retrying, then verifies it.
fn fetch_file(
    agent: &ureq::Agent,
    url: &str,
    entry: &FileEntry,
    target: &Path,
    on_len: &mut dyn FnMut(u64),
) -> Result<(), UpdateError> {
    let part = PathBuf::from(format!("{}.part", target.display()));
    let mut last = UpdateError::Offline;
    for attempt in 0..ATTEMPTS {
        if attempt > 0 {
            std::thread::sleep(RETRY_PAUSE);
        }
        match try_once(agent, url, entry, &part, on_len) {
            Ok(()) => {}
            Err(e @ (UpdateError::Offline | UpdateError::Slow)) => {
                last = e;
                continue;
            }
            Err(e) => return Err(e),
        }
        if sha256_hex_of_file(&part).ok().as_deref() == Some(&entry.sha256) {
            apply::replace_file(&part, target)?;
            return Ok(());
        }
        // Wrong bytes cannot be resumed; start that file again.
        let _ = fs::remove_file(&part);
        last = UpdateError::BadFile(format!("hash mismatch for {}", entry.name));
    }
    Err(last)
}

/// One request. Reports the part file's length as it grows; keeps the part on a cut-off.
fn try_once(
    agent: &ureq::Agent,
    url: &str,
    entry: &FileEntry,
    part: &Path,
    on_len: &mut dyn FnMut(u64),
) -> Result<(), UpdateError> {
    let mut have = fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    if have > entry.size {
        let _ = fs::remove_file(part);
        have = 0;
    }
    if have == entry.size {
        return Ok(());
    }
    let mut req = agent.get(url);
    if have > 0 {
        req = req.header("Range", format!("bytes={have}-"));
    }
    let resp = match req.call() {
        Ok(r) => r,
        Err(ureq::Error::StatusCode(416)) => {
            // The server disagrees about the part; start over on the next try.
            let _ = fs::remove_file(part);
            return Err(UpdateError::Offline);
        }
        Err(e) => return Err(map_net(e)),
    };
    let resumed = have > 0 && resp.status().as_u16() == 206;
    let mut len = if resumed { have } else { 0 };
    let mut file = if resumed {
        OpenOptions::new().append(true).open(part)?
    } else {
        // A plain 200 sends the whole file again, so the old part is useless.
        File::create(part)?
    };
    on_len(len);
    let mut reader = resp.into_body().into_reader();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => {
                file.sync_all().ok();
                return Err(UpdateError::Offline);
            }
        };
        file.write_all(&buf[..n])?;
        len += n as u64;
        on_len(len);
    }
    file.sync_all()?;
    if len < entry.size {
        return Err(UpdateError::Offline);
    }
    Ok(())
}
