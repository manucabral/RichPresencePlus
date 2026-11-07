//! Growing waits between attempts that keep failing.

use std::time::{Duration, Instant};

/// Doubles its wait after each failure up to a ceiling and resets on success.
#[derive(Debug, Clone)]
pub struct Backoff {
    initial: Duration,
    ceiling: Duration,
    delay: Duration,
    retry_at: Option<Instant>,
}

impl Backoff {
    /// Creates a backoff.
    ///
    /// Args:
    ///     initial: Wait after the first failure.
    ///     ceiling: Longest wait the doubling can reach.
    pub fn new(initial: Duration, ceiling: Duration) -> Self {
        Self {
            initial,
            ceiling,
            delay: initial,
            retry_at: None,
        }
    }

    /// True when no failure is pending or its wait has passed.
    pub fn ready(&self, now: Instant) -> bool {
        self.retry_at.is_none_or(|at| now >= at)
    }

    /// Time left until the next attempt, zero when one may be made.
    pub fn remaining(&self, now: Instant) -> Duration {
        self.retry_at
            .map_or(Duration::ZERO, |at| at.saturating_duration_since(now))
    }

    /// Records a failure at `now` and schedules the next attempt.
    pub fn failure(&mut self, now: Instant) {
        self.retry_at = Some(now + self.delay);
        self.delay = (self.delay * 2).min(self.ceiling);
    }

    /// Records a success and forgets earlier failures.
    pub fn success(&mut self) {
        self.delay = self.initial;
        self.retry_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_double_until_the_ceiling() {
        let mut backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));
        let now = Instant::now();
        let mut waits = Vec::new();
        for _ in 0..8 {
            backoff.failure(now);
            waits.push(backoff.remaining(now).as_secs());
        }
        assert_eq!(waits, [1, 2, 4, 8, 16, 30, 30, 30]);
    }

    #[test]
    fn success_resets_the_wait() {
        let mut backoff = Backoff::new(Duration::from_secs(1), Duration::from_secs(30));
        let now = Instant::now();
        backoff.failure(now);
        backoff.failure(now);
        assert!(!backoff.ready(now));
        backoff.success();
        assert!(backoff.ready(now));
        backoff.failure(now);
        assert_eq!(backoff.remaining(now), Duration::from_secs(1));
    }
}
