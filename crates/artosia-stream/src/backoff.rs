//! The pause between reconnect attempts: it doubles up to a ceiling.

use std::time::Duration;

/// Seconds to wait before the first retry.
const FIRST_SECS: u64 = 1;
/// The longest pause, in seconds.
const CEILING_SECS: u64 = 30;
/// A connection that lasted this long counts as healthy and resets the schedule.
pub const HEALTHY_AFTER_SECS: u64 = 20;

/// Doubling pauses: 1, 2, 4, 8, 16, 30, 30 ...
#[derive(Debug, Default, Clone)]
pub struct Backoff {
    failures: u32,
}

impl Backoff {
    pub fn new() -> Self {
        Self::default()
    }

    /// The pause to take after one more failure.
    pub fn next_pause(&mut self) -> Duration {
        let shift = self.failures.min(6);
        self.failures = self.failures.saturating_add(1);
        let secs = FIRST_SECS.saturating_mul(1u64 << shift).min(CEILING_SECS);
        Duration::from_secs(secs)
    }

    /// Call when a connection stayed up long enough to be called healthy.
    pub fn reset(&mut self) {
        self.failures = 0;
    }

    /// How many failures in a row so far.
    pub fn failures(&self) -> u32 {
        self.failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pauses_grow_then_stop_at_the_ceiling() {
        let mut b = Backoff::new();
        let secs: Vec<u64> = (0..9).map(|_| b.next_pause().as_secs()).collect();
        assert_eq!(secs, vec![1, 2, 4, 8, 16, 30, 30, 30, 30]);
    }

    #[test]
    fn reset_starts_over() {
        let mut b = Backoff::new();
        for _ in 0..5 {
            b.next_pause();
        }
        b.reset();
        assert_eq!(b.next_pause().as_secs(), 1);
        assert_eq!(b.failures(), 1);
    }

    #[test]
    fn many_failures_do_not_overflow() {
        let mut b = Backoff::new();
        for _ in 0..1000 {
            assert!(b.next_pause().as_secs() <= 30);
        }
    }
}
