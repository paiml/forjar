//! FJ-1388: Property-based fuzz testing for resource handlers.
//!
//! Tests that resource handler code generation, hash computation,
//! and plan generation are robust across all input domains.

use crate::core::planner;
use crate::core::types::*;
use proptest::prelude::*;
use std::collections::HashMap;

/// Strategy for generating valid resource types.
fn arb_resource_type() -> impl Strategy<Value = ResourceType> {
    prop_oneof![
        Just(ResourceType::Package),
        Just(ResourceType::File),
        Just(ResourceType::Service),
        Just(ResourceType::Mount),
        Just(ResourceType::User),
        Just(ResourceType::Docker),
        Just(ResourceType::Cron),
        Just(ResourceType::Network),
    ]
}

/// Strategy for generating valid mode strings.
fn arb_mode() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("0644".to_string()),
        Just("0755".to_string()),
        Just("0600".to_string()),
        Just("0400".to_string()),
        Just("0700".to_string()),
    ]
}

/// Strategy for generating a minimal Resource with random fields.
fn arb_resource() -> impl Strategy<Value = Resource> {
    (
        arb_resource_type(),
        "[a-z]{3,10}",
        proptest::option::of("[a-z]{3,10}"),
        proptest::option::of(arb_mode()),
    )
        .prop_map(|(rtype, name, owner, mode)| {
            let mut r = Resource {
                resource_type: rtype.clone(),
                machine: MachineTarget::Single("localhost".to_string()),
                name: Some(name.clone()),
                owner,
                mode,
                ..Resource::default()
            };
            match rtype {
                ResourceType::Package => {
                    r.packages = vec![name];
                }
                ResourceType::File => {
                    r.path = Some(format!("/etc/{name}"));
                    r.content = Some("test".to_string());
                }
                ResourceType::Service => {}
                ResourceType::Mount => {
                    r.path = Some(format!("/mnt/{name}"));
                    r.fs_type = Some("ext4".to_string());
                }
                _ => {}
            }
            r
        })
}

proptest! {
    /// FALSIFY-HANDLER-001: hash_desired_state is deterministic for any resource.
    #[test]
    fn falsify_handler_001_hash_determinism(resource in arb_resource()) {
        let h1 = planner::hash_desired_state(&resource);
        let h2 = planner::hash_desired_state(&resource);
        prop_assert_eq!(h1, h2, "hash_desired_state must be deterministic");
    }

    /// FALSIFY-HANDLER-002: Different resource types produce different hashes.
    #[test]
    fn falsify_handler_002_type_affects_hash(
        name in "[a-z]{4,8}",
    ) {
        let mut file_r = Resource {
            resource_type: ResourceType::File,
            path: Some(format!("/etc/{name}")),
            content: Some("test".to_string()),
            machine: MachineTarget::Single("localhost".to_string()),
            ..Resource::default()
        };
        let file_hash = planner::hash_desired_state(&file_r);

        // Change type but keep same fields
        file_r.resource_type = ResourceType::Package;
        file_r.packages = vec![name.clone()];
        let pkg_hash = planner::hash_desired_state(&file_r);

        prop_assert_ne!(file_hash, pkg_hash, "different types must produce different hashes");
    }

    /// FALSIFY-HANDLER-003: Converged + same hash = NoOp in planner.
    #[test]
    fn falsify_handler_003_converged_noop(resource in arb_resource()) {
        let resource_id = "test-resource";
        let machine_name = "localhost";
        let hash = planner::hash_desired_state(&resource);

        // Simulate converged lock with matching hash
        let mut locks = std::collections::HashMap::new();
        let mut state_lock = StateLock {
            schema: "1.0".to_string(),
            machine: machine_name.to_string(),
            hostname: "localhost".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar-proptest".to_string(),
            blake3_version: "1.8".to_string(),
            resources: indexmap::IndexMap::new(),
        };
        state_lock.resources.insert(
            resource_id.to_string(),
            ResourceLock {
                resource_type: resource.resource_type.clone(),
                status: ResourceStatus::Converged,
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(1.0),
                hash,
                details: std::collections::HashMap::new(),
            },
        );
        locks.insert(machine_name.to_string(), state_lock);

        // Build a config with this resource
        let mut config = ForjarConfig {
            version: "1.0".to_string(),
            name: "proptest".to_string(),
            description: None,
            machines: indexmap::IndexMap::new(),
            resources: indexmap::IndexMap::new(),
            params: std::collections::HashMap::new(),
            outputs: indexmap::IndexMap::new(),
            policy: Policy::default(),
            policies: vec![],
            moved: vec![],
            secrets: Default::default(),
            includes: vec![],
            include_provenance: HashMap::new(),
            data: indexmap::IndexMap::new(),
            checks: indexmap::IndexMap::new(),
        };
        config.resources.insert(resource_id.to_string(), resource);

        let execution_order = vec![resource_id.to_string()];
        let plan = planner::plan(&config, &execution_order, &locks, None);

        // All changes should be NoOp since hash matches and status is converged
        for change in &plan.changes {
            prop_assert_eq!(
                &change.action,
                &PlanAction::NoOp,
                "converged resource with matching hash must be NoOp"
            );
        }
    }

    /// FALSIFY-HANDLER-004: Codegen never panics on valid resource.
    #[test]
    fn falsify_handler_004_codegen_no_panic(resource in arb_resource()) {
        // apply_script may return Err for unsupported types, but must never panic
        let result = crate::core::codegen::apply_script(&resource);
        // Just verify it completes without panic
        let _ = result;
    }

    /// FALSIFY-HANDLER-005: Proof obligation classification is total (covers all resource types).
    #[test]
    fn falsify_handler_005_proof_obligation_total(
        rtype in arb_resource_type(),
        action in prop_oneof![
            Just(PlanAction::Create),
            Just(PlanAction::Update),
            Just(PlanAction::Destroy),
            Just(PlanAction::NoOp),
        ],
    ) {
        // classify() must return a valid variant for every combination
        let po = planner::proof_obligation::classify(&rtype, &action);
        let _ = planner::proof_obligation::label(&po);
        let _ = planner::proof_obligation::is_safe(&po);
    }

    /// FALSIFY-HANDLER-006: Chain hash is deterministic for same content.
    #[test]
    fn falsify_handler_006_chain_hash_determinism(
        lines in proptest::collection::vec("[a-zA-Z0-9 ]{1,100}", 0..10),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let events = dir.path().join("events.jsonl");
        let content = lines.join("\n");
        std::fs::write(&events, &content).unwrap();

        let h1 = crate::tripwire::chain::compute_chain_hash(&events).unwrap();
        let h2 = crate::tripwire::chain::compute_chain_hash(&events).unwrap();
        prop_assert_eq!(h1, h2, "chain hash must be deterministic");
    }
}
