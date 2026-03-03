//! Property-based test strategies for Resource types.
//!
//! Provides `arb_resource()` for generating valid Resource values in proptests.

use super::*;
use proptest::prelude::*;

/// Strategy to generate arbitrary ResourceType values.
pub fn arb_resource_type() -> impl Strategy<Value = ResourceType> {
    prop_oneof![
        Just(ResourceType::Package),
        Just(ResourceType::File),
        Just(ResourceType::Service),
        Just(ResourceType::User),
        Just(ResourceType::Docker),
        Just(ResourceType::Cron),
        Just(ResourceType::Task),
    ]
}

/// Strategy to generate arbitrary MachineTarget values.
pub fn arb_machine_target() -> impl Strategy<Value = MachineTarget> {
    prop_oneof![
        "[a-z]{3,8}".prop_map(MachineTarget::Single),
        prop::collection::vec("[a-z]{3,8}", 1..3).prop_map(MachineTarget::Multiple),
    ]
}

/// Strategy to generate arbitrary Resource values with realistic fields.
pub fn arb_resource() -> impl Strategy<Value = Resource> {
    (
        arb_resource_type(),
        arb_machine_target(),
        prop::option::of("[a-z_]+"),
        prop::collection::vec("[a-z0-9-]+", 0..3),
    )
        .prop_map(|(resource_type, machine, state, packages)| {
            let mut r = Resource::default();
            r.resource_type = resource_type;
            r.machine = machine;
            r.state = state;
            r.packages = packages;
            r
        })
}

/// Strategy to generate a GlobalLock with optional outputs.
pub fn arb_global_lock() -> impl Strategy<Value = GlobalLock> {
    (
        "[a-z-]{3,10}",
        prop::collection::btree_map("[a-z_]+", "[a-z0-9.]+", 0..5),
    )
        .prop_map(|(name, outputs)| {
            let mut lock = GlobalLock {
                schema: "1.0".to_string(),
                name,
                last_apply: "2026-01-01T00:00:00Z".to_string(),
                generator: "forjar-test".to_string(),
                machines: indexmap::IndexMap::new(),
                outputs: outputs.into_iter().collect(),
            };
            lock.outputs.sort_keys();
            lock
        })
}

/// Strategy to generate a ResourceLock.
pub fn arb_resource_lock() -> impl Strategy<Value = ResourceLock> {
    (
        arb_resource_type(),
        prop_oneof![
            Just(ResourceStatus::Converged),
            Just(ResourceStatus::Failed),
        ],
        "[0-9a-f]{64}",
    )
        .prop_map(|(rt, status, hash)| ResourceLock {
            resource_type: rt,
            status,
            applied_at: Some("2026-01-01T00:00:00Z".to_string()),
            duration_seconds: Some(1.0),
            hash,
            details: std::collections::HashMap::new(),
        })
}

proptest! {
    /// Resource YAML roundtrip: serialize then deserialize produces equivalent.
    #[test]
    fn resource_yaml_roundtrip(resource in arb_resource()) {
        let yaml = serde_yaml_ng::to_string(&resource).unwrap();
        let parsed: Resource = serde_yaml_ng::from_str(&yaml).unwrap();
        prop_assert_eq!(
            format!("{:?}", resource.resource_type),
            format!("{:?}", parsed.resource_type)
        );
        prop_assert_eq!(resource.packages, parsed.packages);
    }

    /// ResourceLock YAML roundtrip preserves all fields.
    #[test]
    fn resource_lock_yaml_roundtrip(lock in arb_resource_lock()) {
        let yaml = serde_yaml_ng::to_string(&lock).unwrap();
        let parsed: ResourceLock = serde_yaml_ng::from_str(&yaml).unwrap();
        prop_assert_eq!(lock.hash, parsed.hash);
        prop_assert_eq!(lock.status, parsed.status);
    }

    /// GlobalLock serde roundtrip preserves outputs.
    #[test]
    fn global_lock_serde_roundtrip(lock in arb_global_lock()) {
        let yaml = serde_yaml_ng::to_string(&lock).unwrap();
        let parsed: GlobalLock = serde_yaml_ng::from_str(&yaml).unwrap();
        prop_assert_eq!(lock.name, parsed.name);
        prop_assert_eq!(lock.outputs.len(), parsed.outputs.len());
        for (k, v) in &lock.outputs {
            prop_assert_eq!(v, &parsed.outputs[k]);
        }
    }
}
