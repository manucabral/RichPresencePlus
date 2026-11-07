//! Keeping an event that repeats from filling the log.

use std::time::{Duration, Instant};

/// Lets an event through at most once per gap.
#[derive(Debug, Clone)]
pub struct Quiet {
    gap: Duration,
    last: Option<Instant>,
}

impl Quiet {
    /// Creates a gate that lets the first event through.
    pub const fn new(gap: Duration) -> Self {
        Self { gap, last: None }
    }

    /// True for the first event and for one at least a gap after the last;
    /// an event let through starts a new gap.
    pub fn ready(&mut self, now: Instant) -> bool {
        let ready = self
            .last
            .is_none_or(|last| now.duration_since(last) >= self.gap);
        if ready {
            self.last = Some(now);
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_first_event_of_each_gap_goes_through() {
        let start = Instant::now();
        let mut quiet = Quiet::new(Duration::from_secs(60));
        assert!(quiet.ready(start));
        assert!(!quiet.ready(start + Duration::from_secs(59)));
        assert!(quiet.ready(start + Duration::from_secs(60)));
        assert!(!quiet.ready(start + Duration::from_secs(61)));
    }
}
