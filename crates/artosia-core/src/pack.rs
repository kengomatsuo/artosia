//! Packs a folder into one zip file, for handing over what a person chose to share.

use std::io::{self, Write};
use std::path::Path;

/// Writes every file under `folder` into a zip at `out`, with paths as they are under the
/// folder. Folders whose name starts with a dot are left out. Returns how many files went in.
pub fn zip_folder(folder: &Path, out: &Path) -> io::Result<usize> {
    let mut files = Vec::new();
    gather(folder, folder, &mut files)?;
    files.sort();
    let mut writer = zip::ZipWriter::new(io::BufWriter::new(std::fs::File::create(out)?));
    let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for name in &files {
        writer.start_file(name.as_str(), options).map_err(io::Error::other)?;
        io::copy(&mut std::fs::File::open(folder.join(name))?, &mut writer)?;
    }
    writer.finish().map_err(io::Error::other)?.flush()?;
    Ok(files.len())
}

fn gather(root: &Path, folder: &Path, files: &mut Vec<String>) -> io::Result<()> {
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if entry.file_type()?.is_dir() {
            gather(root, &path, files)?;
        } else if let Ok(inside) = path.strip_prefix(root) {
            files.push(inside.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn a_folder_goes_into_one_file_and_comes_back_out() {
        let scratch = std::env::temp_dir().join(format!("artosia-pack-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        let folder = scratch.join("kept");
        std::fs::create_dir_all(folder.join("first")).and_then(|()| std::fs::create_dir_all(folder.join(".deleted"))).unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(folder.join("first").join("log.txt"), "a line\n").unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(folder.join(".deleted").join("gone.txt"), "x").unwrap_or_else(|e| panic!("{e}"));
        let out = scratch.join("kept.zip");
        assert_eq!(zip_folder(&folder, &out).unwrap_or_else(|e| panic!("{e}")), 1);
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&out).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(archive.len(), 1);
        let mut text = String::new();
        archive.by_name("first/log.txt").unwrap_or_else(|e| panic!("{e}")).read_to_string(&mut text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(text, "a line\n");
        let _ = std::fs::remove_dir_all(&scratch);
    }
}
