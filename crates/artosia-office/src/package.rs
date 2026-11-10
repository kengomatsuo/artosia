//! The zip container and its relationship files (`_rels/*.rels`), which say which part is
//! which. Slide order and media names come from here, never from file names.

use std::io::{Cursor, Read};
use std::path::Path;

use zip::ZipArchive;

use crate::error::Error;
use crate::xml::{self, Element};

/// A part is read whole; a bomb that unpacks past this is refused.
const MAX_PART: u64 = 256 * 1024 * 1024;

pub(crate) struct Package {
    archive: ZipArchive<Cursor<Vec<u8>>>,
}

#[derive(Clone, Debug)]
pub(crate) struct Rel {
    pub id: String,
    /// The last path segment of the relationship type: `slide`, `image`, `officeDocument`.
    pub kind: String,
    /// The target as a part name in the zip, resolved against the part that owns the relationship.
    pub target: String,
    pub external: bool,
}

impl Package {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let archive = ZipArchive::new(Cursor::new(bytes)).map_err(|e| Error::Zip(e.to_string()))?;
        Ok(Package { archive })
    }

    pub fn open(path: &Path) -> Result<Self, Error> {
        Self::from_bytes(std::fs::read(path)?)
    }

    pub fn read(&mut self, name: &str) -> Option<Vec<u8>> {
        let file = self.archive.by_name(name).ok()?;
        let mut out = Vec::new();
        file.take(MAX_PART).read_to_end(&mut out).ok()?;
        Some(out)
    }

    pub fn read_xml(&mut self, name: &str) -> Result<Element, Error> {
        let bytes = self.read(name).ok_or_else(|| Error::MissingPart(name.to_string()))?;
        xml::parse(&bytes)
    }

    /// The relationships owned by `part` (`""` for the package itself); empty when it has none.
    pub fn rels(&mut self, part: &str) -> Vec<Rel> {
        let (dir, file) = part.rsplit_once('/').unwrap_or(("", part));
        let rels_name = if dir.is_empty() { format!("_rels/{file}.rels") } else { format!("{dir}/_rels/{file}.rels") };
        let Ok(root) = self.read_xml(&rels_name) else { return Vec::new() };
        root.children_named("Relationship")
            .filter_map(|r| {
                let target = r.attr("Target")?;
                let external = r.attr("TargetMode").is_some_and(|m| m.eq_ignore_ascii_case("External"));
                Some(Rel {
                    id: r.attr("Id")?.to_string(),
                    kind: r.attr("Type")?.rsplit('/').next().unwrap_or("").to_string(),
                    target: if external { target.to_string() } else { resolve(part, target) },
                    external,
                })
            })
            .collect()
    }
}

/// `../media/a.png` seen from `ppt/slides/slide1.xml` is `ppt/media/a.png`.
pub(crate) fn resolve(base_part: &str, target: &str) -> String {
    let target = percent_decode(target);
    let mut segments: Vec<&str> = if let Some(absolute) = target.strip_prefix('/') {
        absolute.split('/').collect()
    } else {
        let dir = base_part.rsplit_once('/').map_or("", |(d, _)| d);
        dir.split('/').filter(|s| !s.is_empty()).chain(target.split('/')).collect()
    };
    let mut out: Vec<&str> = Vec::new();
    for s in segments.drain(..) {
        match s {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    out.join("/")
}

fn percent_decode(text: &str) -> String {
    if !text.contains('%') {
        return text.to_string();
    }
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes.get(i + 1..i + 3).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
