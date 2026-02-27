//! Tests: Time and duration parsing helpers.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::helpers_time::*;

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

    // ── Phase 25: Operational Maturity (FJ-390→FJ-397) ──

}
