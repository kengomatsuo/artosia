//! Run the remote with placeholder content: `cargo run -p artosia-remote --example serve [port]`.
//! It listens on this computer only; add `--lan` to open it from a phone on the same Wi-Fi,
//! which is when Windows Firewall asks whether to allow it.
//!
//! The state covers every control of the page: a plan with headings, sections, search hits,
//! the logo, a message and a countdown. All text is made up for this example.

use artosia_remote::*;
use std::sync::{Mutex, mpsc};
use std::time::{SystemTime, UNIX_EPOCH};

fn slides() -> Vec<(RemoteSlide, usize, &'static str)> {
    let mk = |l: &str, t: &str, g: usize, s: &'static str| (RemoteSlide { label: l.into(), text: t.into() }, g, s);
    vec![
        mk("Verse 1", "Placeholder line one\nPlaceholder line two\nPlaceholder line three", 0, "V1"),
        mk("Chorus", "Sample chorus text goes here\nAnd a second sample line", 1, "C"),
        mk("Verse 2", "Another made-up verse\nWith a second line\nAnd a third", 0, "V2"),
        mk("Chorus", "Sample chorus text goes here\nAnd a second sample line", 1, "C"),
        mk("Bridge", "A short made-up bridge", 2, "B"),
        mk("Verse 3", "Last placeholder verse\nOne more line", 0, "V3"),
        mk("Ending", "A made-up closing line", 4, "E"),
    ]
}

fn plan() -> Vec<RemotePlanItem> {
    let mk = |id: u64, title: &str, kind: &str| RemotePlanItem { id, title: title.into(), kind: kind.into(), live: false };
    vec![
        mk(1, "Welcome", "heading"),
        mk(2, "Sample song", "song"),
        mk(3, "Second sample song", "song"),
        mk(4, "John 3:16", "passage"),
        mk(5, "Offering", "heading"),
        mk(6, "A very long placeholder title that cannot fit on one line of a phone", "song"),
        mk(7, "Announcements", "announcements"),
    ]
}

fn catalogue() -> Vec<RemoteHit> {
    let mk = |id: &str, title: &str, detail: &str, kind: &str| RemoteHit { id: id.into(), title: title.into(), detail: detail.into(), kind: kind.into() };
    vec![
        mk("song:1", "Sample song", "Placeholder line one", "song"),
        mk("song:2", "Second sample song", "Another made-up first line", "song"),
        mk("song:3", "Third sample song", "A line that is long enough to be cut off at the edge of a narrow phone screen", "song"),
        mk("ref:43:3:16:3:16", "John 3:16", "A made-up verse text for the layout check", "passage"),
    ]
}

fn now() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

fn main() -> std::io::Result<()> {
    let port = std::env::args().nth(1).and_then(|p| p.parse().ok()).unwrap_or(4791);
    let pin = std::env::var("PIN").unwrap_or_else(|_| "1234".into());
    // On this computer only unless `--lan` is given: a phone needs the network, a test does
    // not, and listening on the network makes Windows Firewall ask the person at the screen.
    let lan = std::env::args().any(|arg| arg == "--lan");
    let (tx, rx) = mpsc::channel();
    let tx = Mutex::new(tx);
    let server = RemoteServer::start(RemoteConfig { port, pin: pin.clone(), local_only: !lan }, move |c| {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send(c);
        }
    })?;
    println!("port {}  pin {pin}{}", server.port(), if lan { "" } else { "  (this computer only; pass --lan for a phone)" });
    for u in server.urls() {
        println!("{u}?pin={pin}");
    }
    let deck = slides();
    let mut s = RemoteState {
        item_title: "Sample song".into(),
        slides: deck.iter().map(|d| d.0.clone()).collect(),
        live_index: Some(0),
        next_title: "John 3:16".into(),
        logo_ready: true,
        plan: plan(),
        ..Default::default()
    };
    // One section per run of slides, as the operator builds them.
    for (i, (slide, group, short)) in deck.iter().enumerate() {
        if i == 0 || deck[i - 1].2 != *short {
            s.sections.push(RemoteSection { label: slide.label.clone(), short: (*short).into(), group: *group, slide: i });
        }
    }
    s.plan[1].live = true;
    server.set_state(s.clone());
    while let Ok(cmd) = rx.recv() {
        println!("command {cmd:?}");
        let last = s.slides.len().saturating_sub(1);
        // As in the operator, anything sent to the screen lifts Black, Clear text and the logo.
        let reveal = |s: &mut RemoteState| {
            s.black = false;
            s.text_hidden = false;
            s.logo_on = false;
        };
        match cmd {
            RemoteCommand::Next => {
                s.live_index = Some(s.live_index.map_or(0, |i| (i + 1).min(last)));
                reveal(&mut s);
            }
            RemoteCommand::Previous => {
                s.live_index = Some(s.live_index.map_or(0, |i| i.saturating_sub(1)));
                reveal(&mut s);
            }
            RemoteCommand::GoTo(i) => {
                s.live_index = Some(i);
                reveal(&mut s);
            }
            RemoteCommand::ToggleText => s.text_hidden = !s.text_hidden,
            RemoteCommand::ToggleBlack => s.black = !s.black,
            RemoteCommand::ToggleLogo => s.logo_on = !s.logo_on && s.logo_ready,
            RemoteCommand::ShowPlanItem(id) => {
                for p in &mut s.plan {
                    p.live = p.id == id;
                }
                s.live_index = Some(0);
                reveal(&mut s);
            }
            RemoteCommand::Search(q) => {
                let lower = q.to_lowercase();
                s.hits = catalogue().into_iter().filter(|h| lower.is_empty() || h.title.to_lowercase().contains(&lower) || h.detail.to_lowercase().contains(&lower)).collect();
                s.hits_for = q;
            }
            RemoteCommand::ShowHit(_) => {
                s.live_index = Some(0);
                reveal(&mut s);
            }
            RemoteCommand::AddHit(id) => {
                if let Some(h) = catalogue().into_iter().find(|h| h.id == id) {
                    let n = s.plan.iter().map(|p| p.id).max().unwrap_or(0) + 1;
                    s.plan.push(RemotePlanItem { id: n, title: h.title, kind: h.kind, live: false });
                }
            }
            RemoteCommand::Message(t) => s.message = t,
            RemoteCommand::Countdown(0) => s.countdown_ends_at = None,
            RemoteCommand::Countdown(m) => s.countdown_ends_at = Some(now() + f64::from(m) * 60.0),
            RemoteCommand::ToggleLive => s.off = !s.off,
        }
        server.set_state(s.clone());
    }
    Ok(())
}
