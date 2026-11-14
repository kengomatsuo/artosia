//! Each slide of a .pptx as a PNG, drawn by the installed PowerPoint, else LibreOffice.
//!
//! PowerPoint runs through a PowerShell child that drives COM like `tools/pptx-slides.ps1`
//! (read-only, no window, hidden slides left out). LibreOffice has no PDF-to-PNG converter in this
//! tree, so `soffice` draws one slide per run (`impress_png_Export` with a `PageRange`).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::error::Error;
use crate::pptx::Pptx;

/// Which program drew the slides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    PowerPoint,
    LibreOffice,
}

/// The slides that were drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exported {
    pub tool: Tool,
    /// One PNG per visible slide, in slide order, named `slide-001.png` after the slide's number.
    pub files: Vec<PathBuf>,
}

const START_LIMIT: Duration = Duration::from_secs(60);
const PER_SLIDE_LIMIT: Duration = Duration::from_secs(6);
const LIBREOFFICE_RUN_LIMIT: Duration = Duration::from_secs(90);

/// What a route found.
enum Route {
    Done,
    /// The program is not on this PC.
    Missing,
}

/// Draws each visible slide of `pptx` as a PNG `width` pixels wide into `out`. Tries PowerPoint
/// first (Windows only), then LibreOffice. `Error::NoRenderer` when neither is installed.
pub fn slides_as_pictures(pptx: &Path, out: &Path, width: u32) -> Result<Exported, Error> {
    let deck = Pptx::open(pptx)?;
    let width = width.clamp(16, 8192);
    let (cx, cy) = deck.size;
    let height = if cx > 0 && cy > 0 { ((f64::from(width) * cy as f64 / cx as f64).round() as u32).max(1) } else { width * 9 / 16 };
    let numbers: Vec<usize> = deck.slides.iter().filter(|s| !s.hidden).map(|s| s.number).collect();
    let files: Vec<PathBuf> = numbers.iter().map(|n| out.join(format!("slide-{n:03}.png"))).collect();
    std::fs::create_dir_all(out)?;
    for f in &files {
        let _ = std::fs::remove_file(f);
    }
    let limit = START_LIMIT + PER_SLIDE_LIMIT * numbers.len() as u32;

    let mut first_failure = None;
    match powerpoint(pptx, out, width, limit) {
        Ok(Route::Done) => return finish(Tool::PowerPoint, files),
        Ok(Route::Missing) => {}
        Err(e) => first_failure = Some(e),
    }
    match libreoffice(pptx, &numbers, out, (width, height)) {
        Ok(Route::Done) => finish(Tool::LibreOffice, files),
        Ok(Route::Missing) => Err(first_failure.unwrap_or(Error::NoRenderer)),
        Err(e) => Err(e),
    }
}

/// Every slide must have a PNG, or the route did not really work.
fn finish(tool: Tool, files: Vec<PathBuf>) -> Result<Exported, Error> {
    let drawn = files.iter().filter(|f| f.is_file()).count();
    if drawn == files.len() {
        Ok(Exported { tool, files })
    } else {
        Err(Error::Export(format!("{tool:?} drew {drawn} of {} slides", files.len())))
    }
}

struct Ran {
    status_ok: bool,
    code: Option<i32>,
    stderr: String,
}

/// Runs `cmd` for at most `limit`; a run past it is killed with its children. `None` is a timeout.
fn run_limited(mut cmd: Command, limit: Duration) -> std::io::Result<Option<Ran>> {
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn()?;
    let mut pipe = child.stderr.take();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(p) = pipe.as_mut() {
            let _ = p.read_to_end(&mut bytes);
        }
        String::from_utf8_lossy(&bytes).trim().to_string()
    });
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if start.elapsed() >= limit {
            kill_tree(&mut child);
            break None;
        }
        std::thread::sleep(Duration::from_millis(40));
    };
    let stderr = reader.join().unwrap_or_default();
    Ok(status.map(|s| Ran { status_ok: s.success(), code: s.code(), stderr }))
}

fn kill_tree(child: &mut std::process::Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill").args(["/PID", &child.id().to_string(), "/T", "/F"]).creation_flags(CREATE_NO_WINDOW).output();
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn powerpoint(pptx: &Path, out: &Path, width: u32, limit: Duration) -> Result<Route, Error> {
    use std::os::windows::process::CommandExt;

    // The same COM steps as tools/pptx-slides.ps1. Paths travel in the environment, so nothing
    // needs quoting. Exit 3 means the COM class is not registered. The id of a PowerPoint this
    // script started goes to a file, so a timeout can end it; a PowerPoint the user already had
    // open is left running.
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$before = @(Get-Process POWERPNT -ErrorAction SilentlyContinue | ForEach-Object Id)
try { $app = New-Object -ComObject PowerPoint.Application } catch { [Console]::Error.WriteLine('PowerPoint is not installed'); exit 3 }
$mine = @(Get-Process POWERPNT -ErrorAction SilentlyContinue | Where-Object { $before -notcontains $_.Id } | ForEach-Object Id)
if ($mine.Count -gt 0) { Set-Content -Path $env:ARTOSIA_PID -Value $mine[0] }
$deck = $null
try {
    $deck = $app.Presentations.Open($env:ARTOSIA_PPTX, $true, $true, $false)
    $w = [int]$env:ARTOSIA_W
    $h = [int][Math]::Round($w * $deck.PageSetup.SlideHeight / $deck.PageSetup.SlideWidth)
    foreach ($s in $deck.Slides) {
        if ($s.SlideShowTransition.Hidden -ne 0) { continue }
        $s.Export((Join-Path $env:ARTOSIA_OUT ('slide-{0:D3}.png' -f $s.SlideIndex)), 'PNG', $w, $h)
    }
} finally {
    if ($deck) { $deck.Close() }
    if ($app.Presentations.Count -eq 0) { $app.Quit() }
    [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($app)
}
"#;

    let pid_file = std::env::temp_dir().join(format!("artosia-office-ppt-{}.pid", std::process::id()));
    let _ = std::fs::remove_file(&pid_file);
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", SCRIPT])
        .env("ARTOSIA_PPTX", pptx)
        .env("ARTOSIA_OUT", out)
        .env("ARTOSIA_W", width.to_string())
        .env("ARTOSIA_PID", &pid_file)
        .creation_flags(CREATE_NO_WINDOW);
    let ran = match run_limited(cmd, limit) {
        Ok(ran) => ran,
        // No powershell.exe: no PowerPoint route.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Route::Missing),
        Err(e) => return Err(e.into()),
    };
    let pid = std::fs::read_to_string(&pid_file).ok().and_then(|t| t.trim().parse::<u32>().ok());
    let _ = std::fs::remove_file(&pid_file);
    match ran {
        None => {
            if let Some(pid) = pid {
                let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]).creation_flags(CREATE_NO_WINDOW).output();
            }
            Err(Error::Export(format!("PowerPoint did not finish in {} s", limit.as_secs())))
        }
        Some(r) if r.status_ok => Ok(Route::Done),
        Some(r) if r.code == Some(3) => Ok(Route::Missing),
        Some(r) => Err(Error::Export(format!("PowerPoint failed: {}", r.stderr))),
    }
}

#[cfg(not(windows))]
fn powerpoint(_pptx: &Path, _out: &Path, _width: u32, _limit: Duration) -> Result<Route, Error> {
    Ok(Route::Missing)
}

/// `soffice` on the PATH or where its installer puts it.
fn find_soffice() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            candidates.push(dir.join(if cfg!(windows) { "soffice.exe" } else { "soffice" }));
        }
    }
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(base) = std::env::var_os(var) {
            candidates.push(Path::new(&base).join("LibreOffice").join("program").join("soffice.exe"));
        }
    }
    candidates.push(PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice"));
    candidates.into_iter().find(|p| p.is_file())
}

/// A `file://` URL for LibreOffice's `-env:UserInstallation`.
fn file_url(path: &Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/").replace(' ', "%20");
    if p.starts_with('/') { format!("file://{p}") } else { format!("file:///{p}") }
}

/// One `soffice` run per slide, each in a profile of its own so a LibreOffice the user has open
/// is not disturbed. Not run on the PC this was written on (no LibreOffice installed there).
fn libreoffice(pptx: &Path, numbers: &[usize], out: &Path, (width, height): (u32, u32)) -> Result<Route, Error> {
    let Some(soffice) = find_soffice() else { return Ok(Route::Missing) };
    let work = std::env::temp_dir().join(format!("artosia-office-lo-{}", std::process::id()));
    std::fs::create_dir_all(&work)?;
    let stem = pptx.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let result = numbers.iter().try_for_each(|&n| {
        let options = format!(
            r#"png:impress_png_Export:{{"PageRange":{{"type":"string","value":"{n}"}},"PixelWidth":{{"type":"long","value":"{width}"}},"PixelHeight":{{"type":"long","value":"{height}"}}}}"#
        );
        let mut cmd = Command::new(&soffice);
        cmd.arg(format!("-env:UserInstallation={}", file_url(&work.join("profile"))))
            .args(["--headless", "--convert-to", &options, "--outdir"])
            .arg(&work)
            .arg(pptx);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        match run_limited(cmd, LIBREOFFICE_RUN_LIMIT)? {
            None => return Err(Error::Export(format!("LibreOffice did not finish slide {n} in {} s", LIBREOFFICE_RUN_LIMIT.as_secs()))),
            Some(r) if !r.status_ok => return Err(Error::Export(format!("LibreOffice failed: {}", r.stderr))),
            Some(_) => {}
        }
        let target = out.join(format!("slide-{n:03}.png"));
        let drawn = work.join(format!("{stem}.png"));
        std::fs::copy(&drawn, target)?;
        std::fs::remove_file(&drawn)?;
        Ok(())
    });
    let _ = std::fs::remove_dir_all(&work);
    result.map(|()| Route::Done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_files::pptx_bytes;

    #[test]
    fn file_urls() {
        assert_eq!(file_url(Path::new("C:\\Temp\\a b")), "file:///C:/Temp/a%20b");
        assert_eq!(file_url(Path::new("/tmp/a")), "file:///tmp/a");
    }

    #[test]
    fn a_run_past_its_limit_is_killed() {
        let mut cmd;
        if cfg!(windows) {
            cmd = Command::new("powershell.exe");
            cmd.args(["-NoProfile", "-Command", "Start-Sleep 20"]);
        } else {
            cmd = Command::new("sleep");
            cmd.arg("20");
        }
        let start = Instant::now();
        assert!(run_limited(cmd, Duration::from_millis(400)).unwrap().is_none());
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn a_file_that_is_not_a_deck_is_an_error_before_any_program_starts() {
        let dir = std::env::temp_dir().join(format!("artosia-export-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bad = dir.join("bad.pptx");
        std::fs::write(&bad, b"not a zip").unwrap();
        assert!(matches!(slides_as_pictures(&bad, &dir.join("out"), 800), Err(Error::Zip(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Runs the real PowerPoint or LibreOffice when there is one; a PC with neither must say so.
    #[test]
    #[ignore = "starts PowerPoint or LibreOffice"]
    fn a_made_up_deck_is_drawn_when_a_program_exists() {
        let dir = std::env::temp_dir().join(format!("artosia-export-real-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let deck = dir.join("deck.pptx");
        std::fs::write(&deck, pptx_bytes()).unwrap();
        match slides_as_pictures(&deck, &dir.join("out"), 640) {
            Ok(done) => assert_eq!(done.files.len(), 3),
            Err(e) => assert!(matches!(e, Error::NoRenderer | Error::Export(_)), "{e}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
