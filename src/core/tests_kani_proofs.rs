//! Runtime tests for Kani proof stubs (FJ-041, FJ-2201, FJ-2202).
//!
//! These run with regular `cargo test`, not Kani. They verify the same
//! invariants as the bounded model-checking proofs.

use super::kani_proofs::*;

#[test]
fn test_blake3_idempotency_runtime() {
    let data = b"hello world";
    let h1 = blake3::hash(data);
    let h2 = blake3::hash(data);
    assert_eq!(h1, h2);
}

#[test]
fn test_converged_state_noop_runtime() {
    let content = b"test content";
    let h1 = blake3::hash(content).to_hex().to_string();
    let h2 = blake3::hash(content).to_hex().to_string();
    assert_eq!(h1, h2);
}

#[test]
fn test_topo_sort_stability_runtime() {
    let o1 = compute_order(true, false, true);
    let o2 = compute_order(true, false, true);
    assert_eq!(o1, o2);
}

#[test]
fn test_topo_sort_no_edges() {
    let order = compute_order(false, false, false);
    assert_eq!(order, [0, 1, 2]);
}

#[test]
fn test_topo_sort_chain() {
    // 0 → 1 → 2
    let order = compute_order(true, false, true);
    assert_eq!(order, [0, 1, 2]);
}

#[test]
fn test_topo_sort_fan_out() {
    // 0 → 1, 0 → 2
    let order = compute_order(true, true, false);
    assert_eq!(order, [0, 1, 2]);
}

#[test]
fn test_handler_invariant_file_runtime() {
    use crate::core::planner::hash_desired_state;
    use crate::core::types::{Resource, ResourceType};

    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.path = Some("/etc/test.conf".into());
    r.content = Some("key=value".into());
    let h_base = hash_desired_state(&r);

    r.tags = vec!["web".into()];
    assert_eq!(h_base, hash_desired_state(&r), "tags must not affect hash");

    r.depends_on = vec!["dep".into()];
    assert_eq!(h_base, hash_desired_state(&r), "deps must not affect hash");
}

#[test]
fn test_handler_invariant_package_runtime() {
    use crate::core::planner::hash_desired_state;
    use crate::core::types::{Resource, ResourceType};

    let mut r1 = Resource::default();
    r1.resource_type = ResourceType::Package;
    r1.packages = vec!["nginx".into()];

    let mut r2 = r1.clone();
    r2.tags = vec!["web".into()];

    assert_eq!(
        hash_desired_state(&r1),
        hash_desired_state(&r2),
        "tags must not affect package hash"
    );
}

#[test]
fn test_handler_invariant_service_runtime() {
    use crate::core::planner::hash_desired_state;
    use crate::core::types::{Resource, ResourceType};

    let mut r = Resource::default();
    r.resource_type = ResourceType::Service;
    r.name = Some("nginx".into());
    let h_base = hash_desired_state(&r);

    let mut r2 = r.clone();
    r2.tags = vec!["production".into()];
    r2.depends_on = vec!["nginx-pkg".into()];
    assert_eq!(h_base, hash_desired_state(&r2), "tags/deps must not affect service hash");
}

#[test]
fn test_dag_ordering_determinism_runtime() {
    use crate::core::resolver::build_execution_order;
    use crate::core::types::*;

    let yaml = r#"
version: "1.0"
name: dag-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    user: root
    arch: x86_64
resources:
  res-a:
    type: file
    machine: local
    path: /etc/a
    content: "a"
  res-b:
    type: file
    machine: local
    path: /etc/b
    content: "b"
    depends_on: [res-a]
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();

    let o1 = build_execution_order(&config).unwrap();
    let o2 = build_execution_order(&config).unwrap();
    assert_eq!(o1, o2);
    let pos_a = o1.iter().position(|s| s == "res-a").unwrap();
    let pos_b = o1.iter().position(|s| s == "res-b").unwrap();
    assert!(pos_a < pos_b, "dependency must come first");
}
