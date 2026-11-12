//! Phone remote for Artosia: a small HTTP server on the local network.
//!
//! A phone opens the served page, pairs with a PIN and then follows the live slide over a
//! long-poll (`GET /api/state?since=<version>`). Taps arrive as `POST /api/command`.

mod languages;
mod net;
mod server;

use server::{MAX_CONNECTIONS, Shared};
use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RemoteSlide {
    pub label: String,
    pub text: String,
    /// The same lines in each further language the slide shows, in the order they are drawn
    /// under `text`; empty for a slide of one language.
    pub more: Vec<String>,
}

/// One line of today's plan.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RemotePlanItem {
    pub id: u64,
    pub title: String,
    /// "song", "passage", "announcements" or "heading".
    pub kind: String,
    pub live: bool,
}

/// A section of the live item to jump to, such as the chorus.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RemoteSection {
    /// "Verse 1", "Chorus".
    pub label: String,
    /// "V1", "C".
    pub short: String,
    /// Colour family: 0 verse, 1 chorus, 2 bridge, 3 pre-chorus, 4 intro or ending, 5 other.
    pub group: usize,
    /// Index into `slides` of the section's first slide.
    pub slide: usize,
}

/// One search result the phone can show or add to the plan.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RemoteHit {
    /// Opaque; sent back in `ShowHit` or `AddHit`.
    pub id: String,
    pub title: String,
    pub detail: String,
    /// "song" or "passage".
    pub kind: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RemoteState {
    pub item_title: String,
    pub slides: Vec<RemoteSlide>,
    pub live_index: Option<usize>,
    pub next_title: String,
    pub text_hidden: bool,
    pub black: bool,
    /// The Live switch is off: the Live list is filled and the room sees none of it.
    pub off: bool,
    /// The logo is showing in place of the slide.
    pub logo_on: bool,
    /// A logo picture is set, so the Logo button can do something.
    pub logo_ready: bool,
    pub plan: Vec<RemotePlanItem>,
    pub sections: Vec<RemoteSection>,
    /// The answer to the phone's last `Search`, and the query it answers.
    pub hits: Vec<RemoteHit>,
    pub hits_for: String,
    /// The message showing to the room; empty when none is.
    pub message: String,
    /// When the countdown ends, in seconds since the Unix epoch; None when none runs.
    pub countdown_ends_at: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RemoteCommand {
    Next,
    Previous,
    GoTo(usize),
    ToggleText,
    ToggleBlack,
    /// The Live switch: the projector on or off.
    ToggleLive,
    ToggleLogo,
    /// Show the first slide of a plan item.
    ShowPlanItem(u64),
    /// Ask for songs and passages matching this text; the answer arrives in `RemoteState::hits`.
    Search(String),
    /// Show a search result now, by its `RemoteHit::id`.
    ShowHit(String),
    /// Add a search result to the plan.
    AddHit(String),
    /// Show this message to the room; an empty text hides it.
    Message(String),
    /// Start a countdown of this many minutes; 0 stops it.
    Countdown(u32),
}

pub struct RemoteConfig {
    /// 0 picks a free port.
    pub port: u16,
    pub pin: String,
    /// Listen on this computer only (127.0.0.1), for tests and the example. A program that
    /// listens on the network makes Windows Firewall ask whether to allow it: the example did
    /// so twice on 2026-11-11 and its question sat on the owner's screen for hours.
    pub local_only: bool,
}

/// Dropping the server stops it and joins its threads.
pub struct RemoteServer {
    shared: Arc<Shared>,
    port: u16,
    local_only: bool,
    accept: Option<JoinHandle<()>>,
}

impl RemoteServer {
    /// `on_command` runs on a connection thread, not the caller's: several phones can call it
    /// at the same moment, so it must be quick and thread safe (send into a channel, say).
    pub fn start(config: RemoteConfig, on_command: impl Fn(RemoteCommand) + Send + Sync + 'static) -> io::Result<RemoteServer> {
        let local_only = config.local_only;
        let listener = TcpListener::bind((if local_only { Ipv4Addr::LOCALHOST } else { Ipv4Addr::UNSPECIFIED }, config.port))?;
        let port = listener.local_addr()?.port();
        let shared = Arc::new(Shared {
            pin: config.pin,
            on_command: Box::new(on_command),
            inner: Mutex::new(server::Inner {
                state: RemoteState::default(),
                version: 1,
                tokens: HashMap::new(),
                failures: 0,
                locked_until: None,
                shutdown: false,
                conns: HashMap::new(),
                next_conn: 0,
            }),
            changed: Condvar::new(),
        });
        let s = shared.clone();
        let accept = std::thread::Builder::new().name("remote-accept".into()).spawn(move || accept_loop(listener, s))?;
        Ok(RemoteServer { shared, port, local_only, accept: Some(accept) })
    }

    /// Cheap; does nothing when `state` equals the current one.
    pub fn set_state(&self, state: RemoteState) {
        let mut g = self.shared.lock();
        if g.state == state {
            return;
        }
        g.state = state;
        g.version += 1;
        drop(g);
        self.shared.changed.notify_all();
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// One URL per usable IPv4 interface, best guess first. May be empty with no network.
    /// A server that listens on this computer only has one address, this computer's, and
    /// never looks at the network: finding the network's address opens a socket on every
    /// network, and Windows Firewall asks about a program that does.
    pub fn urls(&self) -> Vec<String> {
        let ips = if self.local_only { vec![Ipv4Addr::LOCALHOST] } else { net::lan_ips() };
        ips.into_iter().map(|ip| format!("http://{ip}:{}/", self.port)).collect()
    }

    /// Phones that talked to the server in the last 45 seconds.
    pub fn paired_count(&self) -> usize {
        server::paired_count(&self.shared)
    }
}

impl Drop for RemoteServer {
    fn drop(&mut self) {
        {
            let mut g = self.shared.lock();
            g.shutdown = true;
            for s in g.conns.values() {
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
        }
        self.shared.changed.notify_all();
        // Wake the blocking accept.
        let _ = TcpStream::connect_timeout(&SocketAddr::from((Ipv4Addr::LOCALHOST, self.port)), Duration::from_secs(1));
        if let Some(h) = self.accept.take() {
            let _ = h.join();
        }
    }
}

fn accept_loop(listener: TcpListener, shared: Arc<Shared>) {
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    for incoming in listener.incoming() {
        let Ok(stream) = incoming else { continue };
        handles.retain(|h| !h.is_finished());
        let id = {
            let mut g = shared.lock();
            if g.shutdown {
                break;
            }
            if g.conns.len() >= MAX_CONNECTIONS {
                continue; // dropped: the phone retries
            }
            let Ok(clone) = stream.try_clone() else { continue };
            g.next_conn += 1;
            let id = g.next_conn;
            g.conns.insert(id, clone);
            id
        };
        let s = shared.clone();
        let spawned = std::thread::Builder::new().name("remote-conn".into()).spawn(move || {
            server::handle(&s, stream);
            s.lock().conns.remove(&id);
        });
        match spawned {
            Ok(h) => handles.push(h),
            Err(_) => {
                shared.lock().conns.remove(&id);
            }
        }
    }
    for h in handles {
        let _ = h.join();
    }
}

/// Four random digits.
pub fn new_pin() -> String {
    let mut b = [0u8; 4];
    if getrandom::fill(&mut b).is_err() {
        // Clock fallback: still four digits, only used if the OS RNG is unavailable.
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos());
        return format!("{:04}", t % 10_000);
    }
    let n = u32::from_le_bytes(b) % 10_000;
    format!("{n:04}")
}

/// The QR code for `text` as a square matrix, `true` = dark. No quiet zone is included.
pub fn qr_modules(text: &str) -> Option<Vec<Vec<bool>>> {
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    let w = code.width();
    let flat = code.to_colors();
    Some(flat.chunks(w).map(|row| row.iter().map(|c| *c == qrcode::Color::Dark).collect()).collect())
}
