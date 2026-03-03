//! Tests: Core status command.

#![allow(unused_imports)]
use super::commands::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::status_alerts::*;
use super::status_compliance::*;
use super::status_convergence::*;
use super::status_core::*;
use super::status_drift::*;
use super::status_failures::*;
use super::status_fleet::*;
use super::status_observability::*;
use super::status_resource_detail::*;
use super::status_resources::*;
use super::status_trends::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj602_status_security_posture() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_security_posture(dir.path(), None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj602_status_security_posture_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_security_posture(dir.path(), None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj612_status_resource_cost() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_resource_cost(dir.path(), None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj617_status_drift_forecast() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_status_drift_forecast(dir.path(), None, false);
        assert!(result.is_ok());
    }
}
