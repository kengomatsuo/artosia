//! The signed release list: what it says, how it is signed and how a signature is checked.

use crate::error::UpdateError;
use ring::digest;
use ring::signature::{self, Ed25519KeyPair, KeyPair, UnparsedPublicKey};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

/// The list's name in the staging folder of an installed copy, and on the host for Windows.
pub const MANIFEST_NAME: &str = "manifest.json";
pub const SIGNATURE_NAME: &str = "manifest.json.sig";
pub const FORMAT: u32 = 1;

/// The platform that was published first. Its list and files keep the plain names on the
/// host, because every installed Windows copy and the setup ask for exactly those.
pub const FIRST_PLATFORM: &str = "windows-x86_64";

/// The name of a platform's list on the host: `manifest.json` for Windows,
/// `manifest-linux-x86_64.json` for another, so one release holds a list per platform.
pub fn remote_manifest_name(platform: &str) -> String {
    if platform == FIRST_PLATFORM { MANIFEST_NAME.to_string() } else { format!("manifest-{platform}.json") }
}

/// The name of that list's signature on the host.
pub fn remote_signature_name(platform: &str) -> String {
    format!("{}.sig", remote_manifest_name(platform))
}

/// The name of one file on the host. Names are flat, so any file host works; a platform other
/// than Windows puts its own name in front, so two platforms' files never share a name.
pub fn remote_file_name(platform: &str, path: &str) -> String {
    let flat = path.replace('/', "__");
    if platform == FIRST_PLATFORM { flat } else { format!("{platform}__{flat}") }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FileEntry {
    /// Where the file goes, relative to the install folder, with `/` separators.
    pub path: String,
    /// The file's name next to the manifest on the host (flat, so any file host works).
    pub name: String,
    pub size: u64,
    pub sha256: String,
    #[serde(default)]
    pub exec: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub format: u32,
    pub version: String,
    /// `std::env::consts::OS` and `ARCH` joined by a dash, for example `windows-x86_64`.
    pub platform: String,
    pub notes: String,
    pub released: String,
    pub files: Vec<FileEntry>,
}

pub fn this_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// The public keys that may sign a release. A list so a key can be rotated without a lockout.
#[derive(Clone, Debug, Default)]
pub struct TrustedKeys(Vec<[u8; 32]>);

impl TrustedKeys {
    /// The keys compiled into this build from `update-public-key.txt`.
    pub fn compiled() -> Self {
        Self::from_text(include_str!("../update-public-key.txt"))
    }

    /// One hex key per line; blank lines, `#` comments and all-zero placeholders are skipped.
    pub fn from_text(text: &str) -> Self {
        let keys = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| <[u8; 32]>::try_from(from_hex(l)?).ok())
            .filter(|k| k.iter().any(|b| *b != 0))
            .collect();
        Self(keys)
    }

    pub fn from_keys(keys: Vec<[u8; 32]>) -> Self {
        Self(keys)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    let text = text.trim();
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).ok())
        .collect()
}

pub fn sha256_hex_of(bytes: &[u8]) -> String {
    to_hex(digest::digest(&digest::SHA256, bytes).as_ref())
}

pub fn sha256_hex_of_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut ctx = digest::Context::new(&digest::SHA256);
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        ctx.update(&buf[..n]);
    }
    Ok(to_hex(ctx.finish().as_ref()))
}

/// Signs `message` with a 32-byte Ed25519 seed; returns the signature as hex.
pub fn sign(message: &[u8], seed: &[u8; 32]) -> Result<String, UpdateError> {
    let pair = Ed25519KeyPair::from_seed_unchecked(seed)
        .map_err(|_| UpdateError::BadList("bad signing key".into()))?;
    Ok(to_hex(pair.sign(message).as_ref()))
}

pub fn public_key_of(seed: &[u8; 32]) -> Result<[u8; 32], UpdateError> {
    let pair = Ed25519KeyPair::from_seed_unchecked(seed)
        .map_err(|_| UpdateError::BadList("bad signing key".into()))?;
    <[u8; 32]>::try_from(pair.public_key().as_ref())
        .map_err(|_| UpdateError::BadList("bad public key".into()))
}

/// Checks the signature over the exact manifest bytes, then parses and sanity-checks them.
pub fn verify(
    manifest_bytes: &[u8],
    signature_text: &str,
    keys: &TrustedKeys,
) -> Result<Manifest, UpdateError> {
    if keys.is_empty() {
        return Err(UpdateError::NotSetUp);
    }
    let sig = from_hex(signature_text).ok_or(UpdateError::BadSignature)?;
    let signed = keys.0.iter().any(|key| {
        UnparsedPublicKey::new(&signature::ED25519, key)
            .verify(manifest_bytes, &sig)
            .is_ok()
    });
    if !signed {
        return Err(UpdateError::BadSignature);
    }
    let manifest: Manifest =
        serde_json::from_slice(manifest_bytes).map_err(|e| UpdateError::BadList(e.to_string()))?;
    check_shape(&manifest)?;
    Ok(manifest)
}

fn check_shape(m: &Manifest) -> Result<(), UpdateError> {
    let bad = |why: &str| Err(UpdateError::BadList(why.to_string()));
    if m.format != FORMAT {
        return bad("unknown manifest format");
    }
    if m.files.is_empty() {
        return bad("no files");
    }
    for f in &m.files {
        if !safe_relative(&f.path) {
            return bad("unsafe file path");
        }
        if f.name.is_empty() || f.name.contains(['/', '\\', ':']) || f.name.starts_with('.') {
            return bad("unsafe file name");
        }
        if f.sha256.len() != 64 || from_hex(&f.sha256).is_none() {
            return bad("bad hash");
        }
    }
    Ok(())
}

/// A relative path that stays inside the install folder and avoids our own bookkeeping.
fn safe_relative(path: &str) -> bool {
    if path.is_empty() || path.contains(['\\', ':']) || path.starts_with('/') {
        return false;
    }
    let mut parts = path.split('/');
    let first = parts.clone().next().unwrap_or("");
    if first == "update" || path.ends_with(".artosia-old") || path.ends_with(".artosia-new") {
        return false;
    }
    parts.all(|p| !p.is_empty() && p != "." && p != "..")
}
