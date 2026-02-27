use super::*;
use crate::core::types::{ResourceLock, ResourceStatus, ResourceType};
use std::collections::HashMap;

pub(super) fn make_lock() -> StateLock {
    let mut resources = indexmap::IndexMap::new();
    resources.insert(
        "test-pkg".to_string(),
        ResourceLock {
            resource_type: ResourceType::Package,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-02-16T14:00:00Z".to_string()),
            duration_seconds: Some(1.5),
            hash: "blake3:abc123".to_string(),
            details: HashMap::new(),
        },
    );
    StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test-box".to_string(),
        generated_at: "2026-02-16T14:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources,
    }
}
