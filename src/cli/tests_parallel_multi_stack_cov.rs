//! Coverage tests for parallel_multi_stack.rs — extract_data_deps,
//! compute_waves, partition_ready, print_parallel_plan.

use super::parallel_multi_stack::*;
use std::collections::BTreeSet;

fn make_stack(name: &str, deps: &[&str]) -> StackInfo {
    StackInfo {
        name: name.to_string(),
        path: format!("{name}.yaml"),
        resources: 3,
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
    }
}

// ── extract_data_deps ────────────────────────────────────────────

#[test]
fn extract_no_data_sources() {
    let cfg: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(
        "version: '1.0'\nname: test\nmachines: {}\nresources: {}\n",
    )
    .unwrap();
    let deps = extract_data_deps(&cfg);
    assert!(deps.is_empty());
}

// ── partition_ready ──────────────────────────────────────────────

#[test]
fn partition_all_ready() {
    let stacks = vec![make_stack("net", &[]), make_stack("compute", &[])];
    let placed = BTreeSet::new();
    let refs: Vec<&StackInfo> = stacks.iter().collect();
    let (ready, still) = partition_ready(&refs, &placed);
    assert_eq!(ready.len(), 2);
    assert!(still.is_empty());
}

#[test]
fn partition_none_ready() {
    let stacks = vec![
        make_stack("compute", &["net"]),
        make_stack("storage", &["compute"]),
    ];
    let placed = BTreeSet::new();
    let refs: Vec<&StackInfo> = stacks.iter().collect();
    let (ready, still) = partition_ready(&refs, &placed);
    assert!(ready.is_empty());
    assert_eq!(still.len(), 2);
}

#[test]
fn partition_some_ready() {
    let stacks = vec![
        make_stack("net", &[]),
        make_stack("compute", &["net"]),
    ];
    let placed = BTreeSet::new();
    let refs: Vec<&StackInfo> = stacks.iter().collect();
    let (ready, still) = partition_ready(&refs, &placed);
    assert_eq!(ready, vec!["net"]);
    assert_eq!(still.len(), 1);
}

#[test]
fn partition_with_placed() {
    let stacks = vec![make_stack("compute", &["net"])];
    let mut placed = BTreeSet::new();
    placed.insert("net".to_string());
    let refs: Vec<&StackInfo> = stacks.iter().collect();
    let (ready, still) = partition_ready(&refs, &placed);
    assert_eq!(ready, vec!["compute"]);
    assert!(still.is_empty());
}

// ── compute_waves ────────────────────────────────────────────────

#[test]
fn waves_empty() {
    let waves = compute_waves(&[], 4);
    assert!(waves.is_empty());
}

#[test]
fn waves_single_stack() {
    let stacks = vec![make_stack("net", &[])];
    let waves = compute_waves(&stacks, 4);
    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0].stacks, vec!["net"]);
    assert!(!waves[0].parallel);
}

#[test]
fn waves_independent_stacks() {
    let stacks = vec![
        make_stack("net", &[]),
        make_stack("compute", &[]),
        make_stack("storage", &[]),
    ];
    let waves = compute_waves(&stacks, 10);
    assert_eq!(waves.len(), 1);
    assert_eq!(waves[0].stacks.len(), 3);
    assert!(waves[0].parallel);
}

#[test]
fn waves_linear_deps() {
    let stacks = vec![
        make_stack("net", &[]),
        make_stack("compute", &["net"]),
        make_stack("storage", &["compute"]),
    ];
    let waves = compute_waves(&stacks, 10);
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0].stacks, vec!["net"]);
    assert_eq!(waves[1].stacks, vec!["compute"]);
    assert_eq!(waves[2].stacks, vec!["storage"]);
}

#[test]
fn waves_max_parallel_1() {
    let stacks = vec![
        make_stack("a", &[]),
        make_stack("b", &[]),
    ];
    let waves = compute_waves(&stacks, 1);
    assert_eq!(waves.len(), 2);
    assert!(!waves[0].parallel);
    assert!(!waves[1].parallel);
}

#[test]
fn waves_circular_deps() {
    let stacks = vec![
        make_stack("a", &["b"]),
        make_stack("b", &["a"]),
    ];
    let waves = compute_waves(&stacks, 10);
    assert!(!waves.is_empty());
    // Should force-break the cycle
    let last = waves.last().unwrap();
    assert!(!last.parallel);
}

// ── print_parallel_plan ──────────────────────────────────────────

#[test]
fn print_plan_runs() {
    let plan = ParallelPlan {
        stacks: vec![
            make_stack("net", &[]),
            make_stack("compute", &["net"]),
        ],
        waves: vec![
            Wave { index: 0, stacks: vec!["net".to_string()], parallel: false },
            Wave { index: 1, stacks: vec!["compute".to_string()], parallel: false },
        ],
        total_stacks: 2,
        max_parallelism: 1,
    };
    print_parallel_plan(&plan);
}

#[test]
fn print_plan_parallel() {
    let plan = ParallelPlan {
        stacks: vec![
            make_stack("a", &[]),
            make_stack("b", &[]),
        ],
        waves: vec![
            Wave { index: 0, stacks: vec!["a".to_string(), "b".to_string()], parallel: true },
        ],
        total_stacks: 2,
        max_parallelism: 2,
    };
    print_parallel_plan(&plan);
}

// ── cmd_parallel_stacks ──────────────────────────────────────────

#[test]
fn parallel_stacks_text() {
    let dir = tempfile::tempdir().unwrap();
    let f1 = dir.path().join("a.yaml");
    std::fs::write(&f1, "version: '1.0'\nname: a\nmachines: {}\nresources: {}\n").unwrap();
    let f2 = dir.path().join("b.yaml");
    std::fs::write(&f2, "version: '1.0'\nname: b\nmachines: {}\nresources: {}\n").unwrap();
    let result = cmd_parallel_stacks(&[f1, f2], 4, false);
    assert!(result.is_ok());
}

#[test]
fn parallel_stacks_json() {
    let dir = tempfile::tempdir().unwrap();
    let f1 = dir.path().join("a.yaml");
    std::fs::write(&f1, "version: '1.0'\nname: a\nmachines: {}\nresources: {}\n").unwrap();
    let result = cmd_parallel_stacks(&[f1], 2, true);
    assert!(result.is_ok());
}
