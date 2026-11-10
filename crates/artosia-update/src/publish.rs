//! The maker's side: turn a built folder into an upload-ready folder with a signed manifest.

use crate::error::UpdateError;
use crate::manifest::{
    self, FORMAT, FileEntry, MANIFEST_NAME, Manifest, SIGNATURE_NAME, from_hex, sha256_hex_of_file,
    this_platform,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ReleaseSpec<'a> {
    pub dir: &'a Path,
    pub version: &'a str,
    pub notes: &'a str,
    pub platform: Option<&'a str>,
    pub date: Option<&'a str>,
    pub out: &'a Path,
}

/// Reads a 64-digit hex Ed25519 seed from a key file's text.
pub fn parse_seed(text: &str) -> Result<[u8; 32], UpdateError> {
    from_hex(text)
        .and_then(|b| <[u8; 32]>::try_from(b).ok())
        .ok_or_else(|| UpdateError::BadList("the key must be 64 hex digits".into()))
}

/// Copies every file of `spec.dir` flat into `spec.out`, then writes the manifest and its
/// signature next to them.
pub fn build_release(spec: &ReleaseSpec, seed: &[u8; 32]) -> Result<Manifest, UpdateError> {
    semver::Version::parse(spec.version)
        .map_err(|e| UpdateError::BadVersion(format!("{}: {e}", spec.version)))?;
    fs::create_dir_all(spec.out)?;
    let mut found = Vec::new();
    walk(spec.dir, spec.dir, &mut found)?;
    found.sort();
    let mut files = Vec::new();
    for (rel, full) in found {
        let name = rel.replace('/', "__");
        let dest = spec.out.join(&name);
        fs::copy(&full, &dest)?;
        files.push(FileEntry {
            sha256: sha256_hex_of_file(&dest)?,
            size: fs::metadata(&dest)?.len(),
            exec: is_exec(&rel, &full),
            path: rel,
            name,
        });
    }
    let m = Manifest {
        format: FORMAT,
        version: spec.version.to_string(),
        platform: spec.platform.map_or_else(this_platform, str::to_string),
        notes: spec.notes.to_string(),
        released: spec.date.map_or_else(today, str::to_string),
        files,
    };
    let bytes = serde_json::to_vec_pretty(&m).map_err(|e| UpdateError::BadList(e.to_string()))?;
    let sig = manifest::sign(&bytes, seed)?;
    // Prove the result checks out against the matching public key before anyone uploads it.
    let keys = manifest::TrustedKeys::from_keys(vec![manifest::public_key_of(seed)?]);
    manifest::verify(&bytes, &sig, &keys)?;
    fs::write(spec.out.join(MANIFEST_NAME), &bytes)?;
    fs::write(spec.out.join(SIGNATURE_NAME), format!("{sig}\n"))?;
    Ok(m)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<(), UpdateError> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(root, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| UpdateError::Disk(e.to_string()))?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.push((rel, path));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn is_exec(_rel: &str, full: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(full).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_exec(rel: &str, _full: &Path) -> bool {
    rel.ends_with(".exe")
}

/// Today's UTC date as `YYYY-MM-DD` (days-to-civil, no date crate needed).
fn today() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
