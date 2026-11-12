#![allow(clippy::unwrap_used)]
use artosia_remote::*;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type Seen = Arc<Mutex<Vec<RemoteCommand>>>;

fn start() -> (RemoteServer, Seen) {
    let seen: Seen = Arc::default();
    let s2 = seen.clone();
    let srv = RemoteServer::start(RemoteConfig { port: 0, pin: "4321".into(), local_only: true }, move |c| {
        s2.lock().unwrap().push(c);
    })
    .unwrap();
    (srv, seen)
}

fn http(port: u16, method: &str, path: &str, token: Option<&str>, body: &str) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
    let auth = token.map(|t| format!("Authorization: Bearer {t}\r\n")).unwrap_or_default();
    let req = format!("{method} {path} HTTP/1.1\r\nHost: x\r\n{auth}Content-Length: {}\r\n\r\n{body}", body.len());
    s.write_all(req.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    let status = out[9..12].parse().unwrap();
    let body = out.split_once("\r\n\r\n").map(|x| x.1.to_string()).unwrap_or_default();
    (status, body)
}

fn pair(port: u16) -> String {
    let (st, b) = http(port, "POST", "/api/pair", None, r#"{"pin":"4321"}"#);
    assert_eq!(st, 200);
    serde_json::from_str::<Value>(&b).unwrap()["token"].as_str().unwrap().to_string()
}

fn sample() -> RemoteState {
    RemoteState {
        item_title: "Sample".into(),
        slides: vec![
            RemoteSlide { label: "Verse 1".into(), text: "one".into() },
            RemoteSlide { label: "Chorus".into(), text: "two".into() },
        ],
        live_index: Some(0),
        ..Default::default()
    }
}

#[test]
fn serves_page() {
    let (srv, _) = start();
    let (st, body) = http(srv.port(), "GET", "/", None, "");
    assert_eq!(st, 200);
    assert!(body.contains("Artosia remote"));
    assert_eq!(http(srv.port(), "GET", "/nope", None, "").0, 404);
}

#[test]
fn pairing_right_and_wrong_pin() {
    let (srv, _) = start();
    assert_eq!(srv.paired_count(), 0);
    let (st, _) = http(srv.port(), "POST", "/api/pair", None, r#"{"pin":"0000"}"#);
    assert_eq!(st, 403);
    assert_eq!(http(srv.port(), "POST", "/api/pair", None, "garbage").0, 403);
    let t = pair(srv.port());
    assert_eq!(t.len(), 32);
    assert_eq!(srv.paired_count(), 1);
}

#[test]
fn pin_is_rate_limited() {
    let (srv, _) = start();
    for _ in 0..5 {
        assert_eq!(http(srv.port(), "POST", "/api/pair", None, r#"{"pin":"0000"}"#).0, 403);
    }
    // Locked now: even the right PIN is refused.
    assert_eq!(http(srv.port(), "POST", "/api/pair", None, r#"{"pin":"4321"}"#).0, 429);
}

#[test]
fn auth_required() {
    let (srv, seen) = start();
    let p = srv.port();
    assert_eq!(http(p, "GET", "/api/state?since=0", None, "").0, 401);
    assert_eq!(http(p, "GET", "/api/state?since=0", Some("bad"), "").0, 401);
    assert_eq!(http(p, "POST", "/api/command", None, r#"{"cmd":"next"}"#).0, 401);
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn state_versioning() {
    let (srv, _) = start();
    let p = srv.port();
    let t = pair(p);
    let get = |since: u64| -> Value {
        let (st, b) = http(p, "GET", &format!("/api/state?since={since}"), Some(&t), "");
        assert_eq!(st, 200);
        serde_json::from_str(&b).unwrap()
    };
    let ver0 = get(0)["version"].as_u64().unwrap();
    srv.set_state(sample());
    let v1 = get(0);
    assert_eq!(v1["version"].as_u64().unwrap(), ver0 + 1);
    assert_eq!(v1["title"], "Sample");
    assert_eq!(v1["slides"][1]["label"], "Chorus");
    assert_eq!(v1["live"], 0);
    srv.set_state(sample()); // equal: no bump
    assert_eq!(get(0)["version"], v1["version"]);
    let mut s = sample();
    s.black = true;
    srv.set_state(s);
    let v2 = get(0);
    assert_eq!(v2["version"].as_u64().unwrap(), ver0 + 2);
    assert_eq!(v2["black"], true);
}

#[test]
fn long_poll_wakes_on_set_state() {
    let (srv, _) = start();
    let p = srv.port();
    let t = pair(p);
    let ver = serde_json::from_str::<Value>(&http(p, "GET", "/api/state?since=0", Some(&t), "").1).unwrap()["version"].as_u64().unwrap();
    let (tx, rx) = mpsc::channel();
    let t2 = t.clone();
    let h = std::thread::spawn(move || {
        let r = http(p, "GET", &format!("/api/state?since={ver}"), Some(&t2), "");
        tx.send((Instant::now(), r)).unwrap();
    });
    std::thread::sleep(Duration::from_millis(300));
    assert!(rx.try_recv().is_err(), "must be held while nothing changed");
    let sent = Instant::now();
    srv.set_state(sample());
    let (at, (st, body)) = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(st, 200);
    assert!(at.duration_since(sent) < Duration::from_millis(500));
    assert_eq!(serde_json::from_str::<Value>(&body).unwrap()["title"], "Sample");
    h.join().unwrap();
}

#[test]
fn commands_reach_callback() {
    let (srv, seen) = start();
    let p = srv.port();
    let t = pair(p);
    srv.set_state(sample());
    for (body, ok) in [
        (json!({"cmd":"next"}), true),
        (json!({"cmd":"prev"}), true),
        (json!({"cmd":"goto","index":1}), true),
        (json!({"cmd":"goto","index":2}), false),
        (json!({"cmd":"text"}), true),
        (json!({"cmd":"black"}), true),
        (json!({"cmd":"explode"}), false),
    ] {
        let (st, _) = http(p, "POST", "/api/command", Some(&t), &body.to_string());
        assert_eq!(st, if ok { 204 } else { 400 }, "{body}");
    }
    assert_eq!(
        *seen.lock().unwrap(),
        vec![
            RemoteCommand::Next,
            RemoteCommand::Previous,
            RemoteCommand::GoTo(1),
            RemoteCommand::ToggleText,
            RemoteCommand::ToggleBlack
        ]
    );
}

#[test]
fn several_phones_at_once() {
    let (srv, seen) = start();
    let p = srv.port();
    let tokens: Vec<String> = (0..4).map(|_| pair(p)).collect();
    assert_eq!(srv.paired_count(), 4);
    let hs: Vec<_> = tokens
        .into_iter()
        .map(|t| std::thread::spawn(move || http(p, "GET", "/api/state?since=1", Some(&t), "").0))
        .collect();
    std::thread::sleep(Duration::from_millis(200));
    srv.set_state(sample());
    for h in hs {
        assert_eq!(h.join().unwrap(), 200);
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn drop_stops_server_even_with_a_held_poll() {
    let (srv, _) = start();
    let p = srv.port();
    let t = pair(p);
    let h = std::thread::spawn(move || {
        let mut s = TcpStream::connect(("127.0.0.1", p)).unwrap();
        let req = format!("GET /api/state?since=1 HTTP/1.1\r\nAuthorization: Bearer {t}\r\n\r\n");
        s.write_all(req.as_bytes()).unwrap();
        let mut out = String::new();
        let _ = s.read_to_string(&mut out);
    });
    std::thread::sleep(Duration::from_millis(200));
    let start = Instant::now();
    drop(srv);
    assert!(start.elapsed() < Duration::from_secs(2));
    h.join().unwrap();
    assert!(TcpStream::connect(("127.0.0.1", p)).is_err());
}

#[test]
fn pin_and_qr_shape() {
    for _ in 0..50 {
        let p = new_pin();
        assert_eq!(p.len(), 4);
        assert!(p.chars().all(|c| c.is_ascii_digit()));
    }
    let m = qr_modules("http://192.168.1.20:8080/?pin=1234").unwrap();
    assert!(m.len() >= 21);
    assert!(m.iter().all(|r| r.len() == m.len()));
    assert!(m[0][0] && m[0][6], "finder pattern corner is dark");
    assert!(m.iter().flatten().any(|d| !d));
}

#[test]
fn urls_use_the_port() {
    let (srv, _) = start();
    for u in srv.urls() {
        assert!(u.starts_with("http://") && u.ends_with(&format!(":{}/", srv.port())));
    }
}

/// A test or an example must never make Windows Firewall ask: the server listens on this
/// computer only, so its one address is this computer's and the network is never looked at.
#[test]
fn a_local_server_has_only_this_computers_address() {
    let (srv, _) = start();
    assert_eq!(srv.urls(), vec![format!("http://127.0.0.1:{}/", srv.port())]);
}

fn rich() -> RemoteState {
    RemoteState {
        item_title: "Sample".into(),
        slides: vec![
            RemoteSlide { label: "Verse 1".into(), text: "one".into() },
            RemoteSlide { label: "Chorus".into(), text: "two".into() },
        ],
        live_index: Some(0),
        logo_on: true,
        logo_ready: true,
        plan: vec![
            RemotePlanItem { id: 7, title: "Sample".into(), kind: "song".into(), live: true },
            RemotePlanItem { id: 8, title: "Offering".into(), kind: "heading".into(), live: false },
        ],
        sections: vec![
            RemoteSection { label: "Verse 1".into(), short: "V1".into(), group: 0, slide: 0 },
            RemoteSection { label: "Chorus".into(), short: "C".into(), group: 1, slide: 1 },
        ],
        hits: vec![RemoteHit { id: "song:3".into(), title: "Other".into(), detail: "a line".into(), kind: "song".into() }],
        hits_for: "oth".into(),
        message: "Hello".into(),
        countdown_ends_at: Some(1_900_000_000.5),
        ..Default::default()
    }
}

#[test]
fn state_carries_every_new_field() {
    let (srv, _) = start();
    let p = srv.port();
    let t = pair(p);
    srv.set_state(rich());
    let (st, b) = http(p, "GET", "/api/state?since=0", Some(&t), "");
    assert_eq!(st, 200);
    let v: Value = serde_json::from_str(&b).unwrap();
    assert_eq!(v["logo"], true);
    assert_eq!(v["logo_ready"], true);
    assert_eq!(v["plan"][0], json!({"id": 7, "title": "Sample", "kind": "song", "live": true}));
    assert_eq!(v["plan"][1]["kind"], "heading");
    assert_eq!(v["sections"][1], json!({"label": "Chorus", "short": "C", "group": 1, "slide": 1}));
    assert_eq!(v["hits"][0], json!({"id": "song:3", "title": "Other", "detail": "a line", "kind": "song"}));
    assert_eq!(v["hits_for"], "oth");
    assert_eq!(v["message"], "Hello");
    assert_eq!(v["countdown_ends_at"], 1_900_000_000.5);
    assert!(v["now"].as_f64().unwrap() > 1.7e9);
    // No countdown is null, not absent.
    srv.set_state(sample());
    let v: Value = serde_json::from_str(&http(p, "GET", "/api/state?since=0", Some(&t), "").1).unwrap();
    assert!(v["countdown_ends_at"].is_null());
    assert_eq!(v["plan"], json!([]));
}

fn send_all(srv: &RemoteServer, seen: &Seen, cases: Vec<(Value, Option<RemoteCommand>)>) {
    let t = pair(srv.port());
    for (body, want) in cases {
        let before = seen.lock().unwrap().len();
        let (st, _) = http(srv.port(), "POST", "/api/command", Some(&t), &body.to_string());
        let after = seen.lock().unwrap().clone();
        match want {
            Some(c) => {
                assert_eq!(st, 204, "{body}");
                assert_eq!(after.len(), before + 1, "{body}");
                assert_eq!(after[before], c, "{body}");
            }
            None => {
                assert_eq!(st, 400, "{body}");
                assert_eq!(after.len(), before, "{body}");
            }
        }
    }
}

#[test]
fn new_commands_are_validated() {
    let (srv, seen) = start();
    srv.set_state(rich());
    let long_msg = "x".repeat(201);
    let long_search = "y".repeat(101);
    send_all(
        &srv,
        &seen,
        vec![
            (json!({"cmd":"logo"}), Some(RemoteCommand::ToggleLogo)),
            (json!({"cmd":"plan","id":7}), Some(RemoteCommand::ShowPlanItem(7))),
            (json!({"cmd":"plan","id":8}), None), // a heading cannot be shown
            (json!({"cmd":"plan","id":99}), None),
            (json!({"cmd":"plan","id":"7"}), None),
            (json!({"cmd":"search","text":"  joh 3 "}), Some(RemoteCommand::Search("joh 3".into()))),
            (json!({"cmd":"search","text":""}), Some(RemoteCommand::Search(String::new()))),
            (json!({"cmd":"search","text":long_search}), None),
            (json!({"cmd":"search"}), None),
            (json!({"cmd":"hit","id":"song:3"}), Some(RemoteCommand::ShowHit("song:3".into()))),
            (json!({"cmd":"hit","id":"song:4"}), None),
            (json!({"cmd":"add","id":"song:3"}), Some(RemoteCommand::AddHit("song:3".into()))),
            (json!({"cmd":"add","id":"ref:1:1:1:1:1"}), None),
            (json!({"cmd":"message","text":"Hello\nthere"}), Some(RemoteCommand::Message("Hello there".into()))),
            (json!({"cmd":"message","text":""}), Some(RemoteCommand::Message(String::new()))),
            (json!({"cmd":"message","text":long_msg}), None),
            (json!({"cmd":"countdown","minutes":5}), Some(RemoteCommand::Countdown(5))),
            (json!({"cmd":"countdown","minutes":0}), Some(RemoteCommand::Countdown(0))),
            (json!({"cmd":"countdown","minutes":180}), Some(RemoteCommand::Countdown(180))),
            (json!({"cmd":"countdown","minutes":181}), None),
            (json!({"cmd":"countdown","minutes":-1}), None),
            (json!({"cmd":"countdown","minutes":4_294_967_297u64}), None),
        ],
    );
}

#[test]
fn hit_must_be_in_the_last_answer() {
    let (srv, seen) = start();
    srv.set_state(rich());
    let t = pair(srv.port());
    let body = json!({"cmd":"hit","id":"song:3"}).to_string();
    assert_eq!(http(srv.port(), "POST", "/api/command", Some(&t), &body).0, 204);
    // The operator answers another search: the old hit is gone.
    let mut s = rich();
    s.hits = vec![];
    s.hits_for = "zzz".into();
    srv.set_state(s);
    assert_eq!(http(srv.port(), "POST", "/api/command", Some(&t), &body).0, 400);
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[test]
fn new_commands_need_pairing() {
    let (srv, seen) = start();
    srv.set_state(rich());
    for body in [json!({"cmd":"logo"}), json!({"cmd":"message","text":"hi"}), json!({"cmd":"countdown","minutes":1})] {
        assert_eq!(http(srv.port(), "POST", "/api/command", None, &body.to_string()).0, 401);
    }
    assert!(seen.lock().unwrap().is_empty());
}

#[test]
fn page_is_self_contained_and_in_house_punctuation() {
    let (srv, _) = start();
    let (_, body) = http(srv.port(), "GET", "/", None, "");
    for bad in ["http://", "https://", "//cdn", "<link", "@import", "src=\"http"] {
        assert!(!body.contains(bad), "page mentions {bad}");
    }
    for bad in ['\u{2014}', '\u{2013}', '\u{00b7}'] {
        assert!(!body.contains(bad), "page has {bad:?}");
    }
}
