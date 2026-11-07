//! A value read from the system and reused while it is recent.

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Keeps the last good reading for `ttl`; failures are never kept.
pub(crate) struct Fresh<T> {
    ttl: Duration,
    last: Mutex<Option<(Instant, T)>>,
}

impl<T: Clone> Fresh<T> {
    pub(crate) const fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            last: Mutex::new(None),
        }
    }

    /// The kept reading if it is younger than the limit, else a new one.
    pub(crate) fn get(
        &self,
        now: Instant,
        read: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((at, value)) = last.as_ref()
            && now.saturating_duration_since(*at) < self.ttl
        {
            return Ok(value.clone());
        }
        let value = read()?;
        *last = Some((now, value.clone()));
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn a_recent_reading_is_reused_and_an_old_one_is_replaced() {
        let fresh = Fresh::new(Duration::from_secs(1));
        let reads = Cell::new(0);
        let read = || {
            reads.set(reads.get() + 1);
            Ok(reads.get())
        };
        let start = Instant::now();
        assert_eq!(fresh.get(start, read), Ok(1));
        assert_eq!(fresh.get(start + Duration::from_millis(999), read), Ok(1));
        assert_eq!(fresh.get(start + Duration::from_secs(1), read), Ok(2));
        assert_eq!(reads.get(), 2);
    }

    #[test]
    fn a_failure_is_not_kept() {
        let fresh: Fresh<u8> = Fresh::new(Duration::from_secs(60));
        let now = Instant::now();
        assert!(fresh.get(now, || Err("busy".into())).is_err());
        assert_eq!(fresh.get(now, || Ok(5)), Ok(5));
        assert_eq!(fresh.get(now, || Ok(9)), Ok(5));
    }
}
