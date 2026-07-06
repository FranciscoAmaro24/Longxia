//! Security primitives for the server: shared-token auth and an AI rate/cost
//! limiter. Kept separate from the HTTP wiring so the logic is unit-tested.

use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Constant-time byte comparison. Avoids leaking, via timing, how many leading
/// bytes of a candidate token matched. The length is allowed to leak (a
/// mismatched length returns immediately), which is standard for token checks.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Shared-token authentication. `token: None` disables auth (local dev only);
/// the caller is responsible for refusing that mode on a non-local bind.
pub struct Auth {
    token: Option<String>,
}

impl Auth {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }

    /// Whether auth is turned off (no token configured).
    pub fn disabled(&self) -> bool {
        self.token.is_none()
    }

    /// Validate an `Authorization` header value (e.g. `Bearer <token>`).
    /// Returns true when auth is disabled, or the bearer token matches.
    pub fn check(&self, header: Option<&str>) -> bool {
        let expected = match &self.token {
            None => return true,
            Some(t) => t,
        };
        let bearer = match header.and_then(|h| h.strip_prefix("Bearer ")) {
            Some(b) => b.trim(),
            None => return false,
        };
        ct_eq(bearer.as_bytes(), expected.as_bytes())
    }
}

/// Why an AI request was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum LimitError {
    /// Too many requests within the last minute.
    RatePerMinute,
    /// The daily cost cap has been reached.
    CostPerDay,
}

/// In-memory limiter for the AI endpoint: a per-minute rate limit (abuse
/// control) and a per-day cap (cost control), as fixed windows. Adequate for a
/// single-host personal server; a multi-node deploy would move this to a shared
/// store. A limit of 0 disables that dimension.
pub struct AiLimiter {
    per_min: u32,
    per_day: u32,
    inner: Mutex<Windows>,
}

struct Windows {
    minute_start: Instant,
    minute_count: u32,
    day_start: Instant,
    day_count: u32,
}

impl AiLimiter {
    pub fn new(per_min: u32, per_day: u32) -> Self {
        let now = Instant::now();
        Self {
            per_min,
            per_day,
            inner: Mutex::new(Windows {
                minute_start: now,
                minute_count: 0,
                day_start: now,
                day_count: 0,
            }),
        }
    }

    /// Try to reserve one AI call now. Counts the attempt against both windows
    /// on success. Conservative for the cost cap: an attempt that later fails
    /// upstream still counts, so the budget can only be under-spent, never over.
    pub fn try_acquire(&self) -> Result<(), LimitError> {
        self.try_acquire_at(Instant::now())
    }

    fn try_acquire_at(&self, now: Instant) -> Result<(), LimitError> {
        let mut w = self.inner.lock().unwrap_or_else(|e| e.into_inner());

        if now.duration_since(w.minute_start) >= Duration::from_secs(60) {
            w.minute_start = now;
            w.minute_count = 0;
        }
        if now.duration_since(w.day_start) >= Duration::from_secs(86_400) {
            w.day_start = now;
            w.day_count = 0;
        }

        // Check the day cap first so a spent budget reports the cost limit, not
        // the per-minute one.
        if self.per_day > 0 && w.day_count >= self.per_day {
            return Err(LimitError::CostPerDay);
        }
        if self.per_min > 0 && w.minute_count >= self.per_min {
            return Err(LimitError::RatePerMinute);
        }

        w.minute_count += 1;
        w.day_count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ct_eq_matches_only_identical() {
        assert!(ct_eq(b"secret-token", b"secret-token"));
        assert!(!ct_eq(b"secret-token", b"secret-toke"));
        assert!(!ct_eq(b"secret-token", b"Secret-token"));
        assert!(!ct_eq(b"", b"x"));
        assert!(ct_eq(b"", b""));
    }

    #[test]
    fn auth_disabled_allows_everything() {
        let auth = Auth::new(None);
        assert!(auth.disabled());
        assert!(auth.check(None));
        assert!(auth.check(Some("anything")));
    }

    #[test]
    fn auth_requires_exact_bearer_token() {
        let auth = Auth::new(Some("s3cret".into()));
        assert!(!auth.disabled());
        assert!(auth.check(Some("Bearer s3cret")));
        assert!(!auth.check(Some("Bearer wrong")));
        assert!(!auth.check(Some("s3cret"))); // missing scheme
        assert!(!auth.check(Some("Basic s3cret"))); // wrong scheme
        assert!(!auth.check(None));
    }

    #[test]
    fn ai_limiter_enforces_per_minute() {
        let lim = AiLimiter::new(2, 0);
        let t0 = Instant::now();
        assert!(lim.try_acquire_at(t0).is_ok());
        assert!(lim.try_acquire_at(t0).is_ok());
        assert_eq!(lim.try_acquire_at(t0), Err(LimitError::RatePerMinute));
        // window resets after a minute
        assert!(lim.try_acquire_at(t0 + Duration::from_secs(61)).is_ok());
    }

    #[test]
    fn ai_limiter_enforces_daily_cap() {
        let lim = AiLimiter::new(0, 2);
        let t0 = Instant::now();
        assert!(lim.try_acquire_at(t0).is_ok());
        assert!(lim.try_acquire_at(t0).is_ok());
        assert_eq!(lim.try_acquire_at(t0), Err(LimitError::CostPerDay));
        // still capped later in the same day
        assert_eq!(
            lim.try_acquire_at(t0 + Duration::from_secs(3600)),
            Err(LimitError::CostPerDay)
        );
        // resets after a day
        assert!(lim.try_acquire_at(t0 + Duration::from_secs(86_401)).is_ok());
    }
}
