//! A tiny HTTP/1.1 server over `std::net`: one thread per connection, `Connection: close`.

use crate::{RemoteCommand, RemoteState};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

pub const PAGE: &str = include_str!("page.html");

const MAX_HEAD: usize = 16 * 1024;
const MAX_BODY: usize = 4 * 1024;
const REQUEST_DEADLINE: Duration = Duration::from_secs(10);
pub const LONG_POLL: Duration = Duration::from_secs(20);
/// A phone counts as paired while it has talked to us this recently (a poll lasts at most 20 s).
const PAIRED_WINDOW: Duration = Duration::from_secs(45);
const MAX_TOKENS: usize = 64;
const FAIL_LIMIT: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(60);
pub const MAX_CONNECTIONS: usize = 64;

pub struct Shared {
    pub pin: String,
    pub on_command: Box<dyn Fn(RemoteCommand) + Send + Sync>,
    pub inner: Mutex<Inner>,
    pub changed: Condvar,
}

pub struct Inner {
    pub state: RemoteState,
    pub version: u64,
    pub tokens: HashMap<String, Instant>,
    pub failures: u32,
    pub locked_until: Option<Instant>,
    pub shutdown: bool,
    pub conns: HashMap<u64, TcpStream>,
    pub next_conn: u64,
}

impl Shared {
    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

struct Request {
    method: String,
    path: String,
    query: String,
    auth: Option<String>,
    body: Vec<u8>,
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let start = Instant::now();
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 2048];
    let head_end = loop {
        if let Some(p) = find(&buf, b"\r\n\r\n") {
            break p;
        }
        if buf.len() > MAX_HEAD || start.elapsed() > REQUEST_DEADLINE {
            return None;
        }
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = std::str::from_utf8(&buf[..head_end]).ok()?;
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_string();
    let target = first.next()?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut auth = None;
    let mut len = 0usize;
    for l in lines {
        let Some((k, v)) = l.split_once(':') else { continue };
        let v = v.trim();
        match k.trim().to_ascii_lowercase().as_str() {
            "authorization" => auth = v.strip_prefix("Bearer ").map(|t| t.trim().to_string()),
            "content-length" => len = v.parse().ok()?,
            _ => {}
        }
    }
    if len > MAX_BODY {
        return None;
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < len {
        if start.elapsed() > REQUEST_DEADLINE {
            return None;
        }
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(len);
    Some(Request { method, path: path.to_string(), query: query.to_string(), auth, body })
}

fn respond(stream: &mut TcpStream, status: u16, ctype: &str, body: &[u8]) {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        429 => "Too Many Requests",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn json_reply(stream: &mut TcpStream, status: u16, v: &Value) {
    respond(stream, status, "application/json", v.to_string().as_bytes());
}

fn now_secs() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

fn state_json(s: &RemoteState, version: u64) -> Value {
    json!({
        "version": version,
        "now": now_secs(),
        "title": s.item_title,
        "slides": s.slides.iter().map(|x| json!({"label": x.label, "text": x.text})).collect::<Vec<_>>(),
        "live": s.live_index,
        "next_title": s.next_title,
        "hidden": s.text_hidden,
        "black": s.black,
        "logo": s.logo_on,
        "logo_ready": s.logo_ready,
        "plan": s.plan.iter().map(|x| json!({"id": x.id, "title": x.title, "kind": x.kind, "live": x.live})).collect::<Vec<_>>(),
        "sections": s.sections.iter().map(|x| json!({"label": x.label, "short": x.short, "group": x.group, "slide": x.slide})).collect::<Vec<_>>(),
        "hits": s.hits.iter().map(|x| json!({"id": x.id, "title": x.title, "detail": x.detail, "kind": x.kind})).collect::<Vec<_>>(),
        "hits_for": s.hits_for,
        "message": s.message,
        "countdown_ends_at": s.countdown_ends_at,
    })
}

pub fn random_token() -> String {
    let mut b = [0u8; 16];
    // A failure here would hand out a guessable token, so fall back to refusing to pair.
    if getrandom::fill(&mut b).is_err() {
        return String::new();
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Returns true if the bearer token is known, and marks it seen.
fn authorize(shared: &Shared, req: &Request) -> bool {
    let Some(t) = &req.auth else { return false };
    let mut g = shared.lock();
    match g.tokens.get_mut(t) {
        Some(seen) => {
            *seen = Instant::now();
            true
        }
        None => false,
    }
}

fn pair(shared: &Shared, stream: &mut TcpStream, req: &Request) {
    let pin = serde_json::from_slice::<Value>(&req.body)
        .ok()
        .and_then(|v| v.get("pin").and_then(Value::as_str).map(str::to_string))
        .unwrap_or_default();
    let mut g = shared.lock();
    let now = Instant::now();
    if let Some(until) = g.locked_until {
        if now < until {
            drop(g);
            return json_reply(stream, 429, &json!({"error": "locked"}));
        }
        g.locked_until = None;
        g.failures = 0;
    }
    if pin != shared.pin {
        g.failures += 1;
        if g.failures >= FAIL_LIMIT {
            g.locked_until = Some(now + LOCKOUT);
        }
        drop(g);
        return json_reply(stream, 403, &json!({"error": "wrong pin"}));
    }
    let token = random_token();
    if token.is_empty() {
        drop(g);
        return json_reply(stream, 503, &json!({"error": "no randomness"}));
    }
    g.failures = 0;
    if g.tokens.len() >= MAX_TOKENS
        && let Some(oldest) = g.tokens.iter().min_by_key(|(_, t)| **t).map(|(k, _)| k.clone())
    {
        g.tokens.remove(&oldest);
    }
    g.tokens.insert(token.clone(), now);
    drop(g);
    json_reply(stream, 200, &json!({"token": token}));
}

fn long_poll(shared: &Shared, stream: &mut TcpStream, req: &Request) {
    let since = req
        .query
        .split('&')
        .find_map(|kv| kv.strip_prefix("since="))
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let deadline = Instant::now() + LONG_POLL;
    let mut g = shared.lock();
    while g.version == since && !g.shutdown {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        g = shared.changed.wait_timeout(g, left).unwrap_or_else(PoisonError::into_inner).0;
    }
    if let Some(seen) = req.auth.as_ref().and_then(|t| g.tokens.get_mut(t)) {
        *seen = Instant::now();
    }
    let body = state_json(&g.state, g.version);
    drop(g);
    json_reply(stream, 200, &body);
}

/// Longest search text accepted, in characters.
pub const MAX_SEARCH_CHARS: usize = 100;
/// Longest message to the room, in characters.
pub const MAX_MESSAGE_CHARS: usize = 200;
/// Longest countdown, in minutes.
pub const MAX_COUNTDOWN_MINUTES: u32 = 180;

/// Turns a request body into a command, checking it against the state the phone last saw.
fn parse_command(body: &[u8], state: &RemoteState) -> Option<RemoteCommand> {
    let v: Value = serde_json::from_slice(body).ok()?;
    let text = |max: usize| -> Option<String> {
        let t = v.get("text")?.as_str()?;
        if t.chars().count() > max {
            return None;
        }
        // One line, no control characters: it ends up on a projector.
        let clean: String = t.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
        Some(clean.trim().to_string())
    };
    let hit_id = || -> Option<String> {
        let id = v.get("id")?.as_str()?;
        state.hits.iter().any(|h| h.id == id).then(|| id.to_string())
    };
    match v.get("cmd")?.as_str()? {
        "next" => Some(RemoteCommand::Next),
        "prev" => Some(RemoteCommand::Previous),
        "text" => Some(RemoteCommand::ToggleText),
        "black" => Some(RemoteCommand::ToggleBlack),
        "logo" => Some(RemoteCommand::ToggleLogo),
        "goto" => {
            let i = usize::try_from(v.get("index")?.as_u64()?).ok()?;
            (i < state.slides.len()).then_some(RemoteCommand::GoTo(i))
        }
        "plan" => {
            let id = v.get("id")?.as_u64()?;
            state.plan.iter().any(|p| p.id == id && p.kind != "heading").then_some(RemoteCommand::ShowPlanItem(id))
        }
        "search" => text(MAX_SEARCH_CHARS).map(RemoteCommand::Search),
        "hit" => hit_id().map(RemoteCommand::ShowHit),
        "add" => hit_id().map(RemoteCommand::AddHit),
        "message" => text(MAX_MESSAGE_CHARS).map(RemoteCommand::Message),
        "countdown" => {
            let m = u32::try_from(v.get("minutes")?.as_u64()?).ok()?;
            (m <= MAX_COUNTDOWN_MINUTES).then_some(RemoteCommand::Countdown(m))
        }
        _ => None,
    }
}

fn command(shared: &Shared, stream: &mut TcpStream, req: &Request) {
    let parsed = {
        let g = shared.lock();
        parse_command(&req.body, &g.state)
    };
    match parsed {
        Some(c) => {
            (shared.on_command)(c);
            respond(stream, 204, "text/plain", b"");
        }
        None => json_reply(stream, 400, &json!({"error": "bad command"})),
    }
}

pub fn paired_count(shared: &Shared) -> usize {
    let now = Instant::now();
    shared.lock().tokens.values().filter(|t| now.duration_since(**t) < PAIRED_WINDOW).count()
}

pub fn handle(shared: &Shared, mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_nodelay(true);
    let Some(req) = read_request(&mut stream) else {
        return respond(&mut stream, 400, "text/plain", b"bad request");
    };
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") => respond(&mut stream, 200, "text/html; charset=utf-8", crate::languages::page().as_bytes()),
        ("POST", "/api/pair") => pair(shared, &mut stream, &req),
        ("GET", "/api/state") | ("POST", "/api/command") if !authorize(shared, &req) => {
            json_reply(&mut stream, 401, &json!({"error": "not paired"}))
        }
        ("GET", "/api/state") => long_poll(shared, &mut stream, &req),
        ("POST", "/api/command") => command(shared, &mut stream, &req),
        (_, "/" | "/api/pair" | "/api/state" | "/api/command") => respond(&mut stream, 405, "text/plain", b"method not allowed"),
        _ => respond(&mut stream, 404, "text/plain", b"not found"),
    }
    let _ = stream.shutdown(Shutdown::Both);
}
