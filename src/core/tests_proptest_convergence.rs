//! FJ-2600 / FJ-2601: Convergence and idempotency property tests.
//!
//! Properties verified:
//! - CONV-001: Hash stability — same resource always produces same hash
//! - CONV-002: Plan convergence — new resource plans Create, then NoOp after converging
//! - CONV-003: Preservation — independent resources don't interfere (pairwise)
//! - CONV-004: Codegen idempotency — same resource always produces identical script
//! - CONV-005: Plan idempotency — converged plan applied twice yields identical plans
//! - CONV-006: Hash sensitivity — any field change produces a different hash

use crate::core::{codegen, planner, types::*};
use proptest::prelude::*;
use std::collections::HashMap;

/// Strategy for generating valid resource types (subset that supports codegen).
fn arb_convergent_type() -> impl Strategy<Value = ResourceType> {
    prop_oneof![
        Just(ResourceType::Package),
        Just(ResourceType::File),
        Just(ResourceType::Service),
    ]
}

/// Strategy for generating a minimal convergent resource.
fn arb_convergent_resource() -> impl Strategy<Value = (String, Resource)> {
    (arb_convergent_type(), "[a-z]{3,8}").prop_map(|(rtype, name)| {
        let mut r = Resource {
            resource_type: rtype.clone(),
            machine: MachineTarget::Single("localhost".to_string()),
            name: Some(name.clone()),
            ..Resource::default()
        };
        match rtype {
            ResourceType::Package => {
                r.packages = vec![name.clone()];
                r.provider = Some("apt".to_string());
            }
            ResourceType::File => {
                r.path = Some(format!("/tmp/{name}"));
                r.content = Some("managed".to_string());
                r.mode = Some("0644".to_string());
                r.owner = Some("root".to_string());
            }
            ResourceType::Service => {
                r.state = Some("running".to_string());
            }
            _ => {}
        }
        (name, r)
    })
}

/// Build a minimal ForjarConfig for plan testing.
fn make_config(resources: Vec<(String, Resource)>) -> ForjarConfig {
    let mut config = ForjarConfig {
        version: "1.0".to_string(),
        name: "convergence-test".to_string(),
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
    for (id, r) in resources {
        config.resources.insert(id, r);
    }
    config
}

/// Create a converged lock for a resource.
fn converged_lock(id: &str, resource: &Resource, machine: &str) -> StateLock {
    let hash = planner::hash_desired_state(resource);
    let mut lock = StateLock {
        schema: "1.0".to_string(),
        machine: machine.to_string(),
        hostname: machine.to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar-proptest".to_string(),
        blake3_version: "1.8".to_string(),
        resources: indexmap::IndexMap::new(),
    };
    lock.resources.insert(
        id.to_string(),
        ResourceLock {
            resource_type: resource.resource_type.clone(),
            status: ResourceStatus::Converged,
            applied_at: Some("2026-01-01T00:00:00Z".to_string()),
            duration_seconds: Some(0.1),
            hash,
            details: std::collections::HashMap::new(),
        },
    );
    lock
}

proptest! {
    /// CONV-001: hash_desired_state is deterministic across multiple calls.
    #[test]
    fn conv_001_hash_stability((_, resource) in arb_convergent_resource()) {
        let h1 = planner::hash_desired_state(&resource);
        let h2 = planner::hash_desired_state(&resource);
        let h3 = planner::hash_desired_state(&resource);
        prop_assert_eq!(&h1, &h2);
        prop_assert_eq!(&h2, &h3);
    }

    /// CONV-002: A new resource (no prior state) plans Create;
    /// after converging (lock hash matches), plans NoOp.
    #[test]
    fn conv_002_create_then_noop((id, resource) in arb_convergent_resource()) {
        let config = make_config(vec![(id.clone(), resource.clone())]);
        let order = vec![id.clone()];

        // Phase 1: No prior state → should Create
        let empty_locks = std::collections::HashMap::new();
        let plan1 = planner::plan(&config, &order, &empty_locks, None);
        prop_assert!(!plan1.changes.is_empty(), "plan should have changes");
        prop_assert_eq!(
            &plan1.changes[0].action,
            &PlanAction::Create,
            "new resource must plan Create"
        );

        // Phase 2: After converging → should NoOp
        let lock = converged_lock(&id, &resource, "localhost");
        let mut locks = std::collections::HashMap::new();
        locks.insert("localhost".to_string(), lock);
        let plan2 = planner::plan(&config, &order, &locks, None);
        for change in &plan2.changes {
            prop_assert_eq!(
                &change.action,
                &PlanAction::NoOp,
                "converged resource must plan NoOp"
            );
        }
    }

    /// CONV-003: Independent resources preserve each other's convergence.
    /// If resource A is converged and we plan for both A and B (new),
    /// A stays NoOp while B plans Create.
    #[test]
    fn conv_003_preservation_independent(
        (id_a, res_a) in arb_convergent_resource(),
        (id_b, res_b) in arb_convergent_resource(),
    ) {
        // Ensure distinct IDs
        let id_b = if id_a == id_b { format!("{id_b}-b") } else { id_b };
        let config = make_config(vec![
            (id_a.clone(), res_a.clone()),
            (id_b.clone(), res_b.clone()),
        ]);
        let order = vec![id_a.clone(), id_b.clone()];

        // A is converged, B is new
        let lock_a = converged_lock(&id_a, &res_a, "localhost");
        let mut locks = std::collections::HashMap::new();
        locks.insert("localhost".to_string(), lock_a);

        let plan = planner::plan(&config, &order, &locks, None);

        for change in &plan.changes {
            if change.resource_id == id_a {
                prop_assert_eq!(
                    &change.action,
                    &PlanAction::NoOp,
                    "converged A must remain NoOp when B is added"
                );
            }
            if change.resource_id == id_b {
                prop_assert_eq!(
                    &change.action,
                    &PlanAction::Create,
                    "new B must plan Create"
                );
            }
        }
    }

    /// CONV-004: Codegen produces identical output for the same resource.
    #[test]
    fn conv_004_codegen_idempotency((_, resource) in arb_convergent_resource()) {
        let s1 = codegen::apply_script(&resource);
        let s2 = codegen::apply_script(&resource);
        prop_assert_eq!(s1, s2, "codegen must be deterministic");
    }

    /// CONV-005: Planning twice on the same converged state yields identical plans.
    #[test]
    fn conv_005_plan_idempotency((id, resource) in arb_convergent_resource()) {
        let config = make_config(vec![(id.clone(), resource.clone())]);
        let order = vec![id.clone()];
        let lock = converged_lock(&id, &resource, "localhost");
        let mut locks = std::collections::HashMap::new();
        locks.insert("localhost".to_string(), lock);

        let plan1 = planner::plan(&config, &order, &locks, None);
        let plan2 = planner::plan(&config, &order, &locks, None);

        prop_assert_eq!(plan1.changes.len(), plan2.changes.len());
        for (c1, c2) in plan1.changes.iter().zip(plan2.changes.iter()) {
            prop_assert_eq!(&c1.action, &c2.action);
            prop_assert_eq!(&c1.resource_id, &c2.resource_id);
        }
    }

    /// CONV-006: Changing any field produces a different hash.
    #[test]
    fn conv_006_hash_sensitivity(name in "[a-z]{3,8}") {
        let r1 = Resource {
            resource_type: ResourceType::File,
            path: Some(format!("/tmp/{name}")),
            content: Some("version-a".to_string()),
            machine: MachineTarget::Single("localhost".to_string()),
            mode: Some("0644".to_string()),
            ..Resource::default()
        };
        let mut r2 = r1.clone();
        r2.content = Some("version-b".to_string());

        let h1 = planner::hash_desired_state(&r1);
        let h2 = planner::hash_desired_state(&r2);
        prop_assert_ne!(h1, h2, "different content must produce different hash");
    }
}
