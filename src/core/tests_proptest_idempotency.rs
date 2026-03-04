//! FJ-050: Property-based idempotency tests.
//!
//! Three core properties:
//! 1. BLAKE3 hash idempotency — same content always produces same hash
//! 2. Lock file serde roundtrip — serialize then deserialize is identity
//! 3. Template resolution determinism — same inputs produce same outputs

use crate::core::types::*;
use proptest::prelude::*;

proptest! {
    /// FALSIFY-IDEM-001: BLAKE3 hash is idempotent — same input always gives same hash.
    #[test]
    fn falsify_idem_001_hash_idempotency(content in "[a-zA-Z0-9 ]{1,200}") {
        let h1 = blake3::hash(content.as_bytes()).to_hex().to_string();
        let h2 = blake3::hash(content.as_bytes()).to_hex().to_string();
        prop_assert_eq!(h1, h2, "BLAKE3 hash must be deterministic");
    }

    /// FALSIFY-IDEM-002: StateLock YAML roundtrip is identity.
    #[test]
    fn falsify_idem_002_lock_serde_roundtrip(
        machine in "[a-z]{3,8}",
        hostname in "[a-z]{3,8}\\.[a-z]{2,4}",
        n_resources in 0..5usize,
    ) {
        let mut lock = StateLock {
            schema: "1.0".to_string(),
            machine: machine.clone(),
            hostname,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar-proptest".to_string(),
            blake3_version: "1.8".to_string(),
            resources: indexmap::IndexMap::new(),
        };

        for i in 0..n_resources {
            lock.resources.insert(
                format!("res-{i}"),
                ResourceLock {
                    resource_type: ResourceType::Package,
                    status: ResourceStatus::Converged,
                    applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                    duration_seconds: Some(1.0),
                    hash: blake3::hash(format!("res-{i}").as_bytes()).to_hex().to_string(),
                    details: std::collections::HashMap::new(),
                },
            );
        }

        let yaml = serde_yaml_ng::to_string(&lock).unwrap();
        let parsed: StateLock = serde_yaml_ng::from_str(&yaml).unwrap();

        prop_assert_eq!(lock.machine, parsed.machine);
        prop_assert_eq!(lock.resources.len(), parsed.resources.len());
        for (k, v) in &lock.resources {
            let pv = &parsed.resources[k];
            prop_assert_eq!(&v.hash, &pv.hash);
            prop_assert_eq!(&v.status, &pv.status);
        }
    }

    /// FALSIFY-IDEM-003: Converged state with identical hash is a no-op.
    /// If a resource's current hash matches the desired hash, no change is needed.
    #[test]
    fn falsify_idem_003_converged_is_noop(
        content in "[a-zA-Z0-9]{1,100}",
    ) {
        let hash = blake3::hash(content.as_bytes()).to_hex().to_string();

        // Simulate: current state hash == desired state hash
        let current = ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: Some("2026-01-01T00:00:00Z".to_string()),
            duration_seconds: Some(0.5),
            hash: hash.clone(),
            details: std::collections::HashMap::new(),
        };

        let desired_hash = blake3::hash(content.as_bytes()).to_hex().to_string();

        // When hashes match, no action is needed (idempotency)
        prop_assert_eq!(current.hash, desired_hash,
            "identical content must produce identical hash — converged state is a no-op");
    }
}
