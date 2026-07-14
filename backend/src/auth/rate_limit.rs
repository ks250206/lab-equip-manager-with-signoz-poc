//! In-memory rate limiter for login/register (IP + account).

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitExceeded {
    pub retry_after_secs: u64,
}

pub struct RateLimiter {
    max_attempts: u32,
    window: Duration,
    retry_after: Duration,
    state: Mutex<HashMap<String, Window>>,
    max_entries: usize,
}

#[derive(Debug, Clone)]
struct Window {
    count: u32,
    window_start: Instant,
    blocked_until: Option<Instant>,
}

impl Window {
    fn is_actively_blocked(&self, now: Instant) -> bool {
        self.blocked_until.is_some_and(|until| now < until)
    }
}

impl RateLimiter {
    pub fn new(max_attempts: u32, window: Duration, retry_after: Duration) -> Self {
        Self::with_capacity(max_attempts, window, retry_after, 10_000)
    }

    pub fn with_capacity(
        max_attempts: u32,
        window: Duration,
        retry_after: Duration,
        max_entries: usize,
    ) -> Self {
        assert!(max_entries > 0, "rate limiter capacity must be positive");
        Self {
            max_attempts,
            window,
            retry_after,
            state: Mutex::new(HashMap::new()),
            max_entries,
        }
    }

    pub fn check_and_hit(&self, key: &str) -> Result<(), RateLimitExceeded> {
        let mut guard = self.state.lock().expect("rate limiter poisoned");
        let now = Instant::now();
        guard.retain(|_, entry| match entry.blocked_until {
            Some(until) => now < until,
            None => now.duration_since(entry.window_start) <= self.window,
        });

        if !guard.contains_key(key) && guard.len() >= self.max_entries {
            // Never evict keys that are still blocked (Retry-After active).
            let evict_key = guard
                .iter()
                .filter(|(_, entry)| !entry.is_actively_blocked(now))
                .min_by_key(|(_, entry)| entry.window_start)
                .map(|(k, _)| k.clone());
            match evict_key {
                Some(oldest_key) => {
                    guard.remove(&oldest_key);
                }
                None => {
                    // Map is full of actively blocked entries — fail closed.
                    return Err(RateLimitExceeded {
                        retry_after_secs: self.retry_after.as_secs().max(1),
                    });
                }
            }
        }

        let entry = guard.entry(key.to_string()).or_insert(Window {
            count: 0,
            window_start: now,
            blocked_until: None,
        });

        if let Some(until) = entry.blocked_until {
            if now < until {
                let retry = (until - now).as_secs().max(1);
                return Err(RateLimitExceeded {
                    retry_after_secs: retry,
                });
            }
            entry.blocked_until = None;
            entry.count = 0;
            entry.window_start = now;
        }

        if now.duration_since(entry.window_start) > self.window {
            entry.count = 0;
            entry.window_start = now;
        }

        entry.count += 1;
        if entry.count > self.max_attempts {
            entry.blocked_until = Some(now + self.retry_after);
            return Err(RateLimitExceeded {
                retry_after_secs: self.retry_after.as_secs().max(1),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn allows_under_limit() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60), Duration::from_secs(60));
        assert!(limiter.check_and_hit("ip:1").is_ok());
        assert!(limiter.check_and_hit("ip:1").is_ok());
        assert!(limiter.check_and_hit("ip:1").is_ok());
    }

    #[test]
    fn blocks_over_limit_with_retry_after() {
        let limiter = RateLimiter::new(2, Duration::from_secs(60), Duration::from_secs(60));
        assert!(limiter.check_and_hit("acct:a").is_ok());
        assert!(limiter.check_and_hit("acct:a").is_ok());
        let err = limiter.check_and_hit("acct:a").unwrap_err();
        assert_eq!(err.retry_after_secs, 60);
        let err2 = limiter.check_and_hit("acct:a").unwrap_err();
        assert!(err2.retry_after_secs <= 60);
    }

    #[test]
    fn separate_keys_are_independent() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60), Duration::from_secs(1));
        assert!(limiter.check_and_hit("ip:a").is_ok());
        assert!(limiter.check_and_hit("ip:a").is_err());
        assert!(limiter.check_and_hit("ip:b").is_ok());
    }

    #[test]
    fn recovers_after_retry_window() {
        let limiter = RateLimiter::new(1, Duration::from_millis(50), Duration::from_millis(50));
        assert!(limiter.check_and_hit("k").is_ok());
        assert!(limiter.check_and_hit("k").is_err());
        thread::sleep(Duration::from_millis(60));
        assert!(limiter.check_and_hit("k").is_ok());
    }

    #[test]
    fn removes_expired_entries_and_bounds_tracked_keys() {
        let limiter =
            RateLimiter::with_capacity(3, Duration::from_millis(20), Duration::from_millis(20), 2);
        assert!(limiter.check_and_hit("expired").is_ok());
        thread::sleep(Duration::from_millis(25));
        assert!(limiter.check_and_hit("current").is_ok());
        assert_eq!(limiter.state.lock().unwrap().len(), 1);

        assert!(limiter.check_and_hit("second").is_ok());
        assert!(limiter.check_and_hit("third").is_ok());
        assert_eq!(limiter.state.lock().unwrap().len(), 2);
    }

    #[test]
    fn does_not_evict_actively_blocked_keys_under_capacity_pressure() {
        let limiter =
            RateLimiter::with_capacity(1, Duration::from_secs(60), Duration::from_secs(60), 1);
        assert!(limiter.check_and_hit("blocked").is_ok());
        assert!(limiter.check_and_hit("blocked").is_err());
        assert!(limiter.state.lock().unwrap().contains_key("blocked"));

        // Capacity full with a blocked key — new key must fail closed, not evict the block.
        assert!(limiter.check_and_hit("attacker").is_err());
        assert!(limiter.state.lock().unwrap().contains_key("blocked"));
        assert!(!limiter.state.lock().unwrap().contains_key("attacker"));
    }
}
