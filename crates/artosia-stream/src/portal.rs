//! A screen's picture under Wayland, where no program may look at another's window: the
//! desktop itself shares a screen or a window through its portal
//! (`org.freedesktop.portal.ScreenCast`) once the person has chosen one in the desktop's own
//! dialog, and the picture arrives as a PipeWire stream. OBS takes the screen the same way.
//!
//! The dialog comes once per stream. The portal hands back a token that stands for the choice;
//! it is kept in the person's state folder and offered the next time, so a desktop that
//! honours it (GNOME, KDE and the wlroots portal do) does not ask again.

use ashpd::desktop::PersistMode;
use ashpd::desktop::screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// The connection to PipeWire and the stream's number in it, for `pipewiresrc`.
pub type Shared = (i32, u32);

static ASKED: AtomicBool = AtomicBool::new(false);
static ANSWER: OnceLock<Result<Shared, String>> = OnceLock::new();

/// Whether this desktop is a Wayland one.
pub fn wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some_and(|name| !name.is_empty())
}

/// What the desktop answered: nothing while its dialog is open, then the shared stream or
/// why there is none. The first call asks; the answer stands until the program ends, because
/// asking again would open the dialog again in front of a running service.
pub fn shared() -> Option<&'static Result<Shared, String>> {
    if !ASKED.swap(true, Ordering::SeqCst) {
        std::thread::spawn(|| {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = ANSWER.set(Err(error.to_string()));
                    return;
                }
            };
            runtime.block_on(async {
                match ask().await {
                    Ok((cast, session, remote, node)) => {
                        let _ = ANSWER.set(Ok((remote.as_raw_fd(), node)));
                        // The share lasts as long as its session and its connection are held.
                        std::future::pending::<()>().await;
                        drop((cast, session, remote));
                    }
                    Err(error) => {
                        let _ = ANSWER.set(Err(error.to_string()));
                    }
                }
            });
        });
    }
    ANSWER.get()
}

async fn ask() -> ashpd::Result<(Screencast, ashpd::desktop::Session<Screencast>, std::os::fd::OwnedFd, u32)> {
    let cast = Screencast::new().await?;
    let session = cast.create_session(Default::default()).await?;
    let kept = token_file().and_then(|file| std::fs::read_to_string(file).ok());
    // Only the kinds this desktop can share may be asked for, or the portal refuses the lot.
    let kinds = cast.available_source_types().await.unwrap_or(SourceType::MONITOR) & (SourceType::MONITOR | SourceType::WINDOW);
    cast.select_sources(
        &session,
        SelectSourcesOptions::default()
            .set_cursor_mode(CursorMode::HIDDEN)
            .set_sources(if kinds.is_empty() { SourceType::MONITOR } else { kinds })
            .set_multiple(false)
            .set_persist_mode(PersistMode::ExplicitlyRevoked)
            .set_restore_token(kept.as_deref().map(str::trim).filter(|token| !token.is_empty())),
    )
    .await?
    .response()?;
    let streams = cast.start(&session, None, Default::default()).await?.response()?;
    if let (Some(file), Some(token)) = (token_file(), streams.restore_token()) {
        if let Some(folder) = file.parent() {
            let _ = std::fs::create_dir_all(folder);
        }
        let _ = std::fs::write(file, token);
    }
    let node = streams.streams().first().map(|stream| stream.pipe_wire_node_id()).ok_or_else(|| ashpd::Error::NoResponse)?;
    let remote = cast.open_pipe_wire_remote(&session, Default::default()).await?;
    Ok((cast, session, remote, node))
}

/// `$XDG_STATE_HOME/artosia/screen-share-token`, as the XDG base directory rules place state.
fn token_file() -> Option<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|folder| folder.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local").join("state")))?;
    Some(state.join("artosia").join("screen-share-token"))
}
