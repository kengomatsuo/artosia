//! The one place that talks to the network: a shared agent, a polite pace and error mapping.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::LyricsError;

/// Names the app, nothing else (no contact address by the owner's choice).
const USER_AGENT: &str = concat!("Artosia/", env!("CARGO_PKG_VERSION"));
const TIMEOUT: Duration = Duration::from_secs(5);
/// LRCLIB asks for 200 to 500 ms between calls; the low end keeps typing responsive.
const MIN_GAP: Duration = Duration::from_millis(250);

#[derive(Default)]
struct Pace {
    last: Option<Instant>,
    /// Set from a `Retry-After` answer; no call leaves before it passes.
    blocked_until: Option<Instant>,
}

fn pace() -> &'static Mutex<Pace> {
    static PACE: OnceLock<Mutex<Pace>> = OnceLock::new();
    PACE.get_or_init(|| Mutex::new(Pace::default()))
}

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent(USER_AGENT)
            .http_status_as_error(false)
            .max_redirects(2)
            .build()
            .into()
    })
}

/// Waits out the minimum gap, or refuses when the service told us to back off.
fn wait_turn() -> Result<(), LyricsError> {
    let mut pace = pace().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let now = Instant::now();
    if let Some(until) = pace.blocked_until {
        if until > now {
            return Err(LyricsError::RateLimited(Some((until - now).as_secs() + 1)));
        }
        pace.blocked_until = None;
    }
    if let Some(last) = pace.last {
        let ready = last + MIN_GAP;
        if ready > now {
            std::thread::sleep(ready - now);
        }
    }
    pace.last = Some(Instant::now());
    Ok(())
}

fn remember_block(wait: Option<u64>) {
    let secs = wait.unwrap_or(30).min(600);
    let mut pace = pace().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    pace.blocked_until = Some(Instant::now() + Duration::from_secs(secs));
}

/// Turns a status into the error a volunteer sees, `None` for success.
pub(crate) fn error_for_status(status: u16, retry_after: Option<&str>) -> Option<LyricsError> {
    match status {
        200..=299 => None,
        404 => Some(LyricsError::NotFound),
        429 | 503 => Some(LyricsError::RateLimited(retry_after.and_then(|v| v.trim().parse().ok()))),
        other => Some(LyricsError::BadAnswer(format!("status {other}"))),
    }
}

/// GETs `url` with `query` pairs and returns the body of a successful answer.
pub(crate) fn get(url: &str, query: &[(&str, &str)]) -> Result<String, LyricsError> {
    wait_turn()?;
    let mut request = agent().get(url);
    for (key, value) in query {
        request = request.query(key, value);
    }
    let mut response = request.call().map_err(network_error)?;
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let status = response.status().as_u16();
    if let Some(error) = error_for_status(status, retry_after.as_deref()) {
        if let LyricsError::RateLimited(wait) = &error {
            remember_block(*wait);
        }
        return Err(error);
    }
    response.body_mut().read_to_string().map_err(network_error)
}

fn network_error(error: ureq::Error) -> LyricsError {
    match error {
        ureq::Error::Timeout(_) => LyricsError::Network("it took too long".to_string()),
        other => LyricsError::Network(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_map_to_readable_errors() {
        assert_eq!(error_for_status(200, None), None);
        assert_eq!(error_for_status(404, None), Some(LyricsError::NotFound));
        assert_eq!(error_for_status(429, Some("12")), Some(LyricsError::RateLimited(Some(12))));
        assert_eq!(error_for_status(503, Some("soon")), Some(LyricsError::RateLimited(None)));
        assert!(matches!(error_for_status(500, None), Some(LyricsError::BadAnswer(_))));
    }
}
