//! Each slide of a .pptx as a PNG, drawn by the installed PowerPoint, else LibreOffice.
//!
//! PowerPoint runs through a PowerShell child that drives COM like `tools/pptx-slides.ps1`
//! (read-only, no window, hidden slides left out). LibreOffice has no PDF-to-PNG converter in this
//! tree, so `soffice` draws one slide per run (`impress_png_Export` with a `PageRange`).

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// A signal to stop an export that is running; clones share it.
#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Cancel {
        Cancel::default()
    }

    /// Asks the export to stop: the program it started is ended, and the call returns `Error::Cancelled`.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// How long PowerPoint is given to close what it opened after a cancel, before it is ended.
const STOP_GRACE: Duration = Duration::from_secs(3);

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
/// `cancel` from another thread ends the program that was started (a PowerPoint the person had
/// open is left running, with nothing of ours open in it) and gives `Error::Cancelled`.
pub fn slides_as_pictures(pptx: &Path, out: &Path, width: u32, cancel: &Cancel) -> Result<Exported, Error> {
    let deck = Pptx::open(pptx)?;
    if cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    // PowerPoint resolves a relative path against its own folder, not this program's.
    let (pptx, out) = (&std::path::absolute(pptx)?, &std::path::absolute(out)?);
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
    match powerpoint(pptx, out, width, limit, cancel) {
        Ok(Route::Done) => return finish(Tool::PowerPoint, files),
        Ok(Route::Missing) => {}
        Err(Error::Cancelled) => return Err(Error::Cancelled),
        Err(e) => first_failure = Some(e),
    }
    match libreoffice(pptx, &numbers, out, (width, height), cancel) {
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

/// How a run ended.
enum Outcome {
    Ran(Ran),
    TimedOut,
    Cancelled,
}

/// Runs `cmd` for at most `limit`; a run past it, or one `cancel` ends, is killed with its
/// children. With `stop` (a file the program watches, and how long to wait) a cancel first
/// writes the file, so the program can close what it opened before it is killed.
fn run_limited(mut cmd: Command, limit: Duration, cancel: &Cancel, stop: Option<(&Path, Duration)>) -> std::io::Result<Outcome> {
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
    let mut cancelled = false;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if cancel.is_cancelled() {
            if let Some((file, grace)) = stop {
                let _ = std::fs::write(file, b"stop");
                let until = Instant::now() + grace;
                while Instant::now() < until && child.try_wait()?.is_none() {
                    std::thread::sleep(Duration::from_millis(40));
                }
            }
            kill_tree(&mut child);
            cancelled = true;
            break None;
        }
        if start.elapsed() >= limit {
            kill_tree(&mut child);
            break None;
        }
        std::thread::sleep(Duration::from_millis(40));
    };
    let stderr = reader.join().unwrap_or_default();
    Ok(match status {
        _ if cancelled => Outcome::Cancelled,
        Some(s) => Outcome::Ran(Ran { status_ok: s.success(), code: s.code(), stderr }),
        None => Outcome::TimedOut,
    })
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
fn powerpoint(pptx: &Path, out: &Path, width: u32, limit: Duration, cancel: &Cancel) -> Result<Route, Error> {
    use std::os::windows::process::CommandExt;

    // The same COM steps as tools/pptx-slides.ps1. Paths travel in the environment, so nothing
    // needs quoting. Exit 3 means the COM class is not registered. The id of a PowerPoint this
    // script started goes to a file, so a timeout or a cancel can end it; a PowerPoint the user
    // already had open is left running. A cancel first creates the stop file: the script then
    // leaves its loop and closes its presentation (and the program, when it started it).
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
        if (Test-Path $env:ARTOSIA_STOP) { break }
        if ($s.SlideShowTransition.Hidden -ne 0) { continue }
        $s.Export((Join-Path $env:ARTOSIA_OUT ('slide-{0:D3}.png' -f $s.SlideIndex)), 'PNG', $w, $h)
    }
} finally {
    if ($deck) { $deck.Close() }
    if ($mine.Count -gt 0 -and $app.Presentations.Count -eq 0) { $app.Quit() }
    [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($app)
}
"#;

    let pid_file = std::env::temp_dir().join(format!("artosia-office-ppt-{}.pid", std::process::id()));
    let stop_file = std::env::temp_dir().join(format!("artosia-office-ppt-{}.stop", std::process::id()));
    let _ = std::fs::remove_file(&pid_file);
    let _ = std::fs::remove_file(&stop_file);
    let before = powerpoint_pids();
    let mut cmd = Command::new("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", SCRIPT])
        .env("ARTOSIA_PPTX", pptx)
        .env("ARTOSIA_OUT", out)
        .env("ARTOSIA_W", width.to_string())
        .env("ARTOSIA_PID", &pid_file)
        .env("ARTOSIA_STOP", &stop_file)
        .creation_flags(CREATE_NO_WINDOW);
    let ran = match run_limited(cmd, limit, cancel, Some((&stop_file, STOP_GRACE))) {
        Ok(ran) => ran,
        // No powershell.exe: no PowerPoint route.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Route::Missing),
        Err(e) => return Err(e.into()),
    };
    let pid = std::fs::read_to_string(&pid_file).ok().and_then(|t| t.trim().parse::<u32>().ok());
    let _ = std::fs::remove_file(&pid_file);
    let _ = std::fs::remove_file(&stop_file);
    match ran {
        Outcome::TimedOut | Outcome::Cancelled => {
            // The script's own record first; when it was ended before it wrote one, any
            // PowerPoint that was not running when the export started.
            let started = match pid {
                Some(pid) => vec![pid],
                None => powerpoint_pids().into_iter().filter(|p| !before.contains(p)).collect(),
            };
            for pid in started {
                // Only a PowerPoint: a process number could have been given to another program since.
                let _ = Command::new("taskkill").args(["/FI", &format!("PID eq {pid}"), "/FI", "IMAGENAME eq POWERPNT.EXE", "/F"]).creation_flags(CREATE_NO_WINDOW).output();
            }
            if matches!(ran, Outcome::Cancelled) { Err(Error::Cancelled) } else { Err(Error::Export(format!("PowerPoint did not finish in {} s", limit.as_secs()))) }
        }
        Outcome::Ran(r) if r.status_ok => Ok(Route::Done),
        Outcome::Ran(r) if r.code == Some(3) => Ok(Route::Missing),
        Outcome::Ran(r) => Err(Error::Export(format!("PowerPoint failed: {}", r.stderr))),
    }
}

/// The process numbers of the PowerPoints running now.
#[cfg(windows)]
fn powerpoint_pids() -> Vec<u32> {
    use std::os::windows::process::CommandExt;
    let Ok(listed) = Command::new("tasklist").args(["/FI", "IMAGENAME eq POWERPNT.EXE", "/FO", "CSV", "/NH"]).creation_flags(CREATE_NO_WINDOW).output() else { return Vec::new() };
    // "POWERPNT.EXE","1234",... per line; with none running tasklist prints a sentence instead.
    String::from_utf8_lossy(&listed.stdout).lines().filter(|line| line.starts_with('"')).filter_map(|line| line.split("\",\"").nth(1)).filter_map(|pid| pid.trim_matches('"').parse().ok()).collect()
}

#[cfg(not(windows))]
fn powerpoint(_pptx: &Path, _out: &Path, _width: u32, _limit: Duration, _cancel: &Cancel) -> Result<Route, Error> {
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
fn libreoffice(pptx: &Path, numbers: &[usize], out: &Path, (width, height): (u32, u32), cancel: &Cancel) -> Result<Route, Error> {
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
        match run_limited(cmd, LIBREOFFICE_RUN_LIMIT, cancel, None)? {
            Outcome::Cancelled => return Err(Error::Cancelled),
            Outcome::TimedOut => return Err(Error::Export(format!("LibreOffice did not finish slide {n} in {} s", LIBREOFFICE_RUN_LIMIT.as_secs()))),
            Outcome::Ran(r) if !r.status_ok => return Err(Error::Export(format!("LibreOffice failed: {}", r.stderr))),
            Outcome::Ran(_) => {}
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

    fn sleeper() -> Command {
        let mut cmd;
        if cfg!(windows) {
            cmd = Command::new("powershell.exe");
            cmd.args(["-NoProfile", "-Command", "Start-Sleep 20"]);
        } else {
            cmd = Command::new("sleep");
            cmd.arg("20");
        }
        cmd
    }

    #[test]
    fn a_run_past_its_limit_is_killed() {
        let start = Instant::now();
        assert!(matches!(run_limited(sleeper(), Duration::from_millis(400), &Cancel::new(), None).unwrap(), Outcome::TimedOut));
        assert!(start.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn a_cancel_ends_the_run_at_once() {
        let cancel = Cancel::new();
        let signal = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            signal.cancel();
        });
        let start = Instant::now();
        assert!(matches!(run_limited(sleeper(), Duration::from_secs(60), &cancel, None).unwrap(), Outcome::Cancelled));
        assert!(start.elapsed() < Duration::from_secs(5), "{:?}", start.elapsed());
    }

    #[test]
    fn a_cancel_writes_the_stop_file_and_ends_a_program_that_ignores_it_after_the_grace() {
        let stop = std::env::temp_dir().join(format!("artosia-office-stop-{}", std::process::id()));
        let _ = std::fs::remove_file(&stop);
        let cancel = Cancel::new();
        cancel.cancel();
        let start = Instant::now();
        let ran = run_limited(sleeper(), Duration::from_secs(60), &cancel, Some((&stop, Duration::from_millis(300)))).unwrap();
        assert!(matches!(ran, Outcome::Cancelled));
        assert!(stop.is_file(), "the program is told to stop first");
        assert!(start.elapsed() < Duration::from_secs(5), "{:?}", start.elapsed());
        let _ = std::fs::remove_file(stop);
    }

    #[test]
    fn a_cancel_before_the_start_draws_nothing() {
        let dir = std::env::temp_dir().join(format!("artosia-export-cancel-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let deck = dir.join("deck.pptx");
        std::fs::write(&deck, pptx_bytes()).unwrap();
        let cancel = Cancel::new();
        cancel.cancel();
        assert!(matches!(slides_as_pictures(&deck, &dir.join("out"), 640, &cancel), Err(Error::Cancelled)));
        assert!(!dir.join("out").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Starts the real PowerPoint, cancels while it works and finds none of ours left running.
    #[test]
    #[ignore = "starts PowerPoint"]
    #[cfg(windows)]
    fn a_cancel_ends_the_powerpoint_it_started() {
        let dir = std::env::temp_dir().join(format!("artosia-export-cancel-real-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let deck = dir.join("deck.pptx");
        std::fs::write(&deck, pptx_bytes()).unwrap();
        let before = powerpoint_pids();
        let cancel = Cancel::new();
        let signal = cancel.clone();
        // Before the made-up deck could be opened (PowerPoint rejects it: it is no real deck).
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            signal.cancel();
        });
        let start = Instant::now();
        let result = slides_as_pictures(&deck, &dir.join("out"), 640, &cancel);
        println!("answered in {:?}: {result:?}", start.elapsed());
        assert!(matches!(result, Err(Error::Cancelled)));
        std::thread::sleep(Duration::from_millis(1000));
        let left: Vec<u32> = powerpoint_pids().into_iter().filter(|p| !before.contains(p)).collect();
        assert!(left.is_empty(), "PowerPoint left running: {left:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_that_is_not_a_deck_is_an_error_before_any_program_starts() {
        let dir = std::env::temp_dir().join(format!("artosia-export-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bad = dir.join("bad.pptx");
        std::fs::write(&bad, b"not a zip").unwrap();
        assert!(matches!(slides_as_pictures(&bad, &dir.join("out"), 800, &Cancel::new()), Err(Error::Zip(_))));
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
        match slides_as_pictures(&deck, &dir.join("out"), 640, &Cancel::new()) {
            Ok(done) => assert_eq!(done.files.len(), 3),
            Err(e) => assert!(matches!(e, Error::NoRenderer | Error::Export(_)), "{e}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
