//! Tests: Time and duration parsing helpers.

#![allow(unused_imports)]
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::helpers_time::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj284_parse_duration_secs() {
        assert_eq!(parse_duration_secs("30m").unwrap(), 1800);
        assert_eq!(parse_duration_secs("24h").unwrap(), 86400);
        assert_eq!(parse_duration_secs("7d").unwrap(), 604800);
        assert_eq!(parse_duration_secs("60s").unwrap(), 60);
    }

    #[test]
    fn test_fj387_parse_duration_string() {
        assert_eq!(parse_duration_string("30s").unwrap(), 30);
        assert_eq!(parse_duration_string("5m").unwrap(), 300);
        assert_eq!(parse_duration_string("2h").unwrap(), 7200);
        assert_eq!(parse_duration_string("7d").unwrap(), 604800);
        assert!(parse_duration_string("abc").is_err());
    }

    // ── #154 (#28): multi-byte trailing char and overflow must Err, not panic ──

    #[test]
    fn test_gh154_parse_duration_string_multibyte_unit_errs() {
        // "5↑" / "10€" end in a multi-byte char; byte split_at used to panic
        // ("not a char boundary"). Now they return the existing Err string.
        assert!(parse_duration_string("5↑").is_err());
        assert!(parse_duration_string("10€").is_err());
        assert!(parse_duration_secs("5µ").is_err());
    }

    #[test]
    fn test_gh154_parse_duration_string_overflow_errs() {
        // num * 86400 overflows u64; must Err (was a debug panic / release wrap).
        assert!(parse_duration_string("999999999999999d").is_err());
        assert!(parse_duration_secs("999999999999999d").is_err());
    }

    #[test]
    fn test_gh154_parse_duration_string_bad_unit_errs() {
        assert!(parse_duration_string("10x").is_err());
        assert!(parse_duration_secs("10x").is_err());
        assert!(parse_duration_string("").is_err());
    }

    proptest::proptest! {
        /// #154: arbitrary input must never panic — only Ok/Err.
        #[test]
        fn prop_gh154_parse_duration_no_panic(s in ".*") {
            let _ = parse_duration_string(&s);
            let _ = parse_duration_secs(&s);
        }
    }

    // ── Phase 25: Operational Maturity (FJ-390→FJ-397) ──
}
