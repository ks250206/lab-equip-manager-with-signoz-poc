//! Reservation time-window conflict detection.

use chrono::{DateTime, Utc};

/// Half-open style overlap: [start, end) overlaps if start < other_end && other_start < end.
pub fn ranges_overlap(
    start_a: DateTime<Utc>,
    end_a: DateTime<Utc>,
    start_b: DateTime<Utc>,
    end_b: DateTime<Utc>,
) -> bool {
    start_a < end_b && start_b < end_a
}

pub fn is_valid_range(starts_at: DateTime<Utc>, ends_at: DateTime<Utc>) -> bool {
    ends_at > starts_at
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(h: u32, m: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 14, h, m, 0).unwrap()
    }

    #[test]
    fn detects_overlap() {
        assert!(ranges_overlap(t(10, 0), t(12, 0), t(11, 0), t(13, 0)));
        assert!(ranges_overlap(t(10, 0), t(12, 0), t(10, 0), t(12, 0)));
        assert!(!ranges_overlap(t(10, 0), t(11, 0), t(11, 0), t(12, 0)));
        assert!(!ranges_overlap(t(10, 0), t(11, 0), t(12, 0), t(13, 0)));
    }

    #[test]
    fn validates_range() {
        assert!(is_valid_range(t(10, 0), t(11, 0)));
        assert!(!is_valid_range(t(11, 0), t(10, 0)));
        assert!(!is_valid_range(t(10, 0), t(10, 0)));
    }
}
