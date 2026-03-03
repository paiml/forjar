//! Tests: Apply command.

#![allow(unused_imports)]
use super::apply::*;
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn make_test_resource_lock(rtype: types::ResourceType) -> types::ResourceLock {
        types::ResourceLock {
            resource_type: rtype,
            status: types::ResourceStatus::Converged,
            applied_at: Some("2026-01-15T10:30:00Z".to_string()),
            duration_seconds: Some(0.5),
            hash: "blake3:abcdef123456".to_string(),
            details: HashMap::new(),
        }
    }
}
