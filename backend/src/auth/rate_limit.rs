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
}

#[derive(Debug, Clone)]
struct Window {
    count: u32,
    window_start: Instant,
    blocked_until: Option<Instant>,
}

impl RateLimiter {
    pub fn new(max_attempts: u32, window: Duration, retry_after: Duration) -> Self {
        Self {
            max_attempts,
            window,
            retry_after,
            state: Mutex::new(HashMap::new()),
        }
    }

    pub fn check_and_hit(&self, key: &str) -> Result<(), RateLimitExceeded> {
        let mut guard = self.state.lock().expect("rate limiter poisoned");
        let now = Instant::now();
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
}
