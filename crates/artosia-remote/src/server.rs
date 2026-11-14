//! A tiny HTTP/1.1 server over `std::net`: one thread per connection, `Connection: close`.

use crate::edit::{Level, Refusal, Taken};
use crate::{PhoneInfo, RemoteCommand, RemoteState};
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

/// A paired phone.
pub struct Phone {
    pub seen: Instant,
    pub level: Level,
    pub id: u32,
    pub agent: String,
}

/// Wrong PINs counted, and the minute pairing waits after five.
#[derive(Default)]
pub struct Guard {
    pub failures: u32,
    pub locked_until: Option<Instant>,
}

impl Guard {
    /// True while the lock holds; an expired lock is cleared.
    fn locked(&mut self, now: Instant) -> bool {
        match self.locked_until {
            Some(until) if now < until => true,
            Some(_) => {
                *self = Guard::default();
                false
            }
            None => false,
        }
    }

    fn failed(&mut self, now: Instant) {
        self.failures += 1;
        if self.failures >= FAIL_LIMIT {
            self.locked_until = Some(now + LOCKOUT);
        }
    }
}

pub struct Inner {
    pub state: RemoteState,
    pub version: u64,
    pub tokens: HashMap<String, Phone>,
    /// The first PIN's tries and the second PIN's own, so a guessed second PIN never stops pairing.
    pub guard: Guard,
    pub edit_guard: Guard,
    pub edit_pin: Option<String>,
    pub next_phone: u32,
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
    agent: String,
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
    let mut agent = String::new();
    let mut len = 0usize;
    for l in lines {
        let Some((k, v)) = l.split_once(':') else { continue };
        let v = v.trim();
        match k.trim().to_ascii_lowercase().as_str() {
            "authorization" => auth = v.strip_prefix("Bearer ").map(|t| t.trim().to_string()),
            "content-length" => len = v.parse().ok()?,
            "user-agent" => agent = v.to_string(),
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
    Some(Request { method, path: path.to_string(), query: query.to_string(), auth, agent, body })
}

fn respond(stream: &mut TcpStream, status: u16, ctype: &str, body: &[u8]) {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        409 => "Conflict",
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

fn state_json(s: &RemoteState, version: u64, level: Level) -> Value {
    let mut out = base_json(s, version);
    out["level"] = json!(level.code());
    if level == Level::Edit {
        out["cards"] = json!(s.cards.iter().map(|c| json!({"id": c.id, "title": c.title, "body": c.body, "body_editable": c.body_editable, "kind": c.kind, "rev": c.rev()})).collect::<Vec<_>>());
        out["people"] = json!(s.people.iter().map(|p| json!({"id": p.id, "name": p.name, "day": p.day, "month": p.month, "next": p.next, "rev": p.rev()})).collect::<Vec<_>>());
        out["events"] = json!(s.events.iter().map(|e| json!({"id": e.id, "original": e.original, "name": e.name, "date": e.date, "start": e.start, "end": e.end, "repeats": e.repeats, "rev": e.rev()})).collect::<Vec<_>>());
        out["live_card"] = json!(s.live_card);
        out["locked"] = json!(s.locked);
    }
    out
}

fn base_json(s: &RemoteState, version: u64) -> Value {
    json!({
        "version": version,
        "now": now_secs(),
        "title": s.item_title,
        "slides": s.slides.iter().map(|x| json!({"label": x.label, "text": x.text, "more": x.more})).collect::<Vec<_>>(),
        "live": s.live_index,
        "next_title": s.next_title,
        "hidden": s.text_hidden,
        "black": s.black,
        "off": s.off,
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

/// The level of the bearer token's phone when it is known, and marks it seen.
fn authorize(shared: &Shared, req: &Request) -> Option<Level> {
    let t = req.auth.as_ref()?;
    let mut g = shared.lock();
    let phone = g.tokens.get_mut(t)?;
    phone.seen = Instant::now();
    Some(phone.level)
}

/// "iPhone", "iPad", "Android" or nothing, from what the phone's browser calls itself.
fn agent_name(agent: &str) -> String {
    ["iPhone", "iPad", "Android"].iter().find(|name| agent.contains(**name)).map(|name| (*name).to_string()).unwrap_or_default()
}

fn pair(shared: &Shared, stream: &mut TcpStream, req: &Request) {
    let body = serde_json::from_slice::<Value>(&req.body).ok();
    let pin = body.as_ref().and_then(|v| v.get("pin").and_then(Value::as_str)).unwrap_or_default().to_string();
    // The second PIN is typed on purpose (`edit: true`), so its tries are counted apart.
    let second = body.as_ref().and_then(|v| v.get("edit").and_then(Value::as_bool)).unwrap_or(false);
    let mut g = shared.lock();
    let now = Instant::now();
    let locked = if second { g.edit_guard.locked(now) } else { g.guard.locked(now) };
    if locked {
        drop(g);
        return json_reply(stream, 429, &json!({"error": "locked"}));
    }
    let right = if second { g.edit_pin.as_deref() == Some(pin.as_str()) } else { pin == shared.pin };
    if !right {
        if second {
            g.edit_guard.failed(now);
        } else {
            g.guard.failed(now);
        }
        drop(g);
        return json_reply(stream, 403, &json!({"error": "wrong pin"}));
    }
    let token = random_token();
    if token.is_empty() {
        drop(g);
        return json_reply(stream, 503, &json!({"error": "no randomness"}));
    }
    if second {
        g.edit_guard = Guard::default();
    } else {
        g.guard = Guard::default();
    }
    if g.tokens.len() >= MAX_TOKENS
        && let Some(oldest) = g.tokens.iter().min_by_key(|(_, p)| p.seen).map(|(k, _)| k.clone())
    {
        g.tokens.remove(&oldest);
    }
    g.next_phone += 1;
    let level = if second { Level::Edit } else { Level::Control };
    let id = g.next_phone;
    g.tokens.insert(token.clone(), Phone { seen: now, level, id, agent: agent_name(&req.agent) });
    drop(g);
    json_reply(stream, 200, &json!({"token": token, "level": level.code()}));
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
    let mut level = Level::Control;
    if let Some(phone) = req.auth.as_ref().and_then(|t| g.tokens.get_mut(t)) {
        phone.seen = Instant::now();
        level = phone.level;
    }
    let body = state_json(&g.state, g.version, level);
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
        "live" => Some(RemoteCommand::ToggleLive),
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

fn command(shared: &Shared, stream: &mut TcpStream, req: &Request, level: Level) {
    if serde_json::from_slice::<Value>(&req.body).ok().is_some_and(|v| v.get("cmd").and_then(Value::as_str) == Some("edit")) {
        return edit(shared, stream, req, level);
    }
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

/// An edit: the level, the limits, the revision and the console's lock are checked against the
/// server's copy, the copy takes the change at once and the computer applies it.
fn edit(shared: &Shared, stream: &mut TcpStream, req: &Request, level: Level) {
    let Ok(body) = serde_json::from_slice::<Value>(&req.body) else { return json_reply(stream, 400, &json!({"error": "bad command"})) };
    let taken = {
        let mut g = shared.lock();
        let taken = g.state.take_edit(level, &body);
        if let Ok(Taken::Relay(edit)) = &taken {
            g.state.apply_edit(edit);
            g.version += 1;
            shared.changed.notify_all();
        }
        taken
    };
    match taken {
        Ok(Taken::Relay(edit)) => {
            (shared.on_command)(RemoteCommand::Edit(edit));
            respond(stream, 204, "text/plain", b"");
        }
        Ok(Taken::Already) => respond(stream, 204, "text/plain", b""),
        Err(Refusal::Bad) => json_reply(stream, 400, &json!({"error": "bad command"})),
        Err(Refusal::Forbidden) => json_reply(stream, 403, &json!({"error": "no edit"})),
        Err(Refusal::Stale(current)) => json_reply(stream, 409, &json!({"error": "stale", "current": current})),
        Err(Refusal::Console) => json_reply(stream, 409, &json!({"error": "console"})),
    }
}

pub fn paired_count(shared: &Shared) -> usize {
    let now = Instant::now();
    shared.lock().tokens.values().filter(|p| now.duration_since(p.seen) < PAIRED_WINDOW).count()
}

pub fn phones(shared: &Shared) -> Vec<PhoneInfo> {
    let now = Instant::now();
    let mut list: Vec<PhoneInfo> = shared
        .lock()
        .tokens
        .values()
        .filter(|p| now.duration_since(p.seen) < PAIRED_WINDOW)
        .map(|p| PhoneInfo { id: p.id, agent: p.agent.clone(), level: p.level, active: true })
        .collect();
    list.sort_by_key(|p| p.id);
    list
}

pub fn handle(shared: &Shared, mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let _ = stream.set_nodelay(true);
    let Some(req) = read_request(&mut stream) else {
        return respond(&mut stream, 400, "text/plain", b"bad request");
    };
    let level = if matches!(req.path.as_str(), "/api/state" | "/api/command") { authorize(shared, &req) } else { None };
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") => respond(&mut stream, 200, "text/html; charset=utf-8", crate::languages::page().as_bytes()),
        ("POST", "/api/pair") => pair(shared, &mut stream, &req),
        ("GET", "/api/state") | ("POST", "/api/command") if level.is_none() => json_reply(&mut stream, 401, &json!({"error": "not paired"})),
        ("GET", "/api/state") => long_poll(shared, &mut stream, &req),
        ("POST", "/api/command") => command(shared, &mut stream, &req, level.unwrap_or_default()),
        (_, "/" | "/api/pair" | "/api/state" | "/api/command") => respond(&mut stream, 405, "text/plain", b"method not allowed"),
        _ => respond(&mut stream, 404, "text/plain", b"not found"),
    }
    let _ = stream.shutdown(Shutdown::Both);
}
