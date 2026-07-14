//! Integration-style tests for reservation conflict logic against SQL expressions.
//! Domain unit tests live next to modules; this file covers overlap predicate parity.

use chrono::{TimeZone, Utc};
use equipment_reservation::domain::ranges_overlap;

#[test]
fn overlap_predicate_matches_sql_semantics() {
    let a0 = Utc.with_ymd_and_hms(2026, 7, 14, 10, 0, 0).unwrap();
    let a1 = Utc.with_ymd_and_hms(2026, 7, 14, 12, 0, 0).unwrap();
    let b0 = Utc.with_ymd_and_hms(2026, 7, 14, 11, 0, 0).unwrap();
    let b1 = Utc.with_ymd_and_hms(2026, 7, 14, 13, 0, 0).unwrap();
    // SQL: starts_at < $ends AND ends_at > $starts
    assert!(ranges_overlap(a0, a1, b0, b1));
    assert!(a0 < b1 && b0 < a1);

    let c0 = Utc.with_ymd_and_hms(2026, 7, 14, 12, 0, 0).unwrap();
    let c1 = Utc.with_ymd_and_hms(2026, 7, 14, 14, 0, 0).unwrap();
    assert!(!ranges_overlap(a0, a1, c0, c1));
}
