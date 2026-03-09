//! DAG tests.

#![allow(unused_imports)]
use super::dag::{build_execution_order, compute_parallel_waves};
use super::tests_helpers::{dag_config, make_base_resource};
use super::*;
use std::collections::HashMap;

#[test]
fn test_fj003_topo_linear() {
    // Linear chain: a -> b -> c
    let config = dag_config(&["a", "b", "c"], &[("a", "b"), ("b", "c")]);
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order, vec!["a", "b", "c"]);
}

#[test]
fn test_fj003_topo_parallel() {
    // Two independent resources — alphabetical tie-breaking
    let config = dag_config(&["alpha", "beta"], &[]);
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order, vec!["alpha", "beta"]);
}

#[test]
fn test_fj003_topo_diamond() {
    // Diamond: top -> left, top -> right, left -> bottom, right -> bottom
    let config = dag_config(
        &["top", "left", "right", "bottom"],
        &[
            ("top", "left"),
            ("top", "right"),
            ("left", "bottom"),
            ("right", "bottom"),
        ],
    );
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order[0], "top");
    assert_eq!(order[3], "bottom");
    // left and right between, alphabetical
    assert_eq!(order[1], "left");
    assert_eq!(order[2], "right");
}

#[test]
fn test_fj003_topo_cycle() {
    // Cycle: a -> b -> a
    let config = dag_config(&["a", "b"], &[("a", "b"), ("b", "a")]);
    let result = build_execution_order(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cycle"));
}

#[test]
fn test_fj003_self_dependency_is_cycle() {
    let config = dag_config(&["self-ref"], &[("self-ref", "self-ref")]);
    let result = build_execution_order(&config);
    assert!(result.is_err(), "self-dependency must be detected as cycle");
    assert!(result.unwrap_err().contains("cycle"));
}

#[test]
fn test_fj003_transitive_3_level_chain() {
    // database -> schema -> app
    let config = dag_config(
        &["database", "schema", "app"],
        &[("database", "schema"), ("schema", "app")],
    );
    let order = build_execution_order(&config).unwrap();
    let pos_db = order.iter().position(|x| x == "database").unwrap();
    let pos_schema = order.iter().position(|x| x == "schema").unwrap();
    let pos_app = order.iter().position(|x| x == "app").unwrap();
    assert!(pos_db < pos_schema, "database before schema");
    assert!(pos_schema < pos_app, "schema before app");
}

#[test]
fn test_fj003_empty_depends_on_vs_missing() {
    // Both empty depends_on and missing depends_on should work the same
    // dag_config creates resources with no deps when no edges point to them
    let config = dag_config(&["explicit-empty", "implicit-empty"], &[]);
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order.len(), 2);
    // Both should be independent -- alphabetical tie-break
    assert_eq!(order[0], "explicit-empty");
    assert_eq!(order[1], "implicit-empty");
}

#[test]
fn test_fj003_single_resource_no_deps() {
    let config = dag_config(&["solo"], &[]);
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order, vec!["solo"]);
}

// -- Falsification tests (DAG Ordering Contract) -----

#[test]
fn test_fj003_wide_fan_out_dag() {
    // One node depended on by many
    let config = dag_config(
        &["root", "a", "b", "c", "d"],
        &[("root", "a"), ("root", "b"), ("root", "c"), ("root", "d")],
    );
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order[0], "root");
    // a, b, c, d in alphabetical order after root
    assert_eq!(order[1], "a");
    assert_eq!(order[2], "b");
    assert_eq!(order[3], "c");
    assert_eq!(order[4], "d");
}

#[test]
fn test_fj003_wide_fan_in_dag() {
    // Many nodes converging to one
    let config = dag_config(
        &["a", "b", "c", "leaf"],
        &[("a", "leaf"), ("b", "leaf"), ("c", "leaf")],
    );
    let order = build_execution_order(&config).unwrap();
    // a, b, c independent -- alphabetical
    assert_eq!(order[0], "a");
    assert_eq!(order[1], "b");
    assert_eq!(order[2], "c");
    assert_eq!(order[3], "leaf");
}

#[test]
fn test_fj003_dag_unknown_dependency() {
    let config = dag_config(&["a"], &[("nonexistent", "a")]);
    let result = build_execution_order(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown"));
}

#[test]
fn test_fj132_build_dag_unknown_dependency() {
    let mut resources = indexmap::IndexMap::new();
    let mut r = make_base_resource();
    r.depends_on = vec!["nonexistent".to_string()];
    resources.insert("my-file".to_string(), r);
    let config = ForjarConfig {
        version: "1.0".to_string(),
        name: "test".to_string(),
        description: None,
        params: HashMap::new(),
        machines: indexmap::IndexMap::new(),
        resources,
        policy: Policy::default(),
        outputs: indexmap::IndexMap::new(),
        policies: vec![],
        data: indexmap::IndexMap::new(),
        includes: vec![],
        include_provenance: HashMap::new(),
        checks: indexmap::IndexMap::new(),
        moved: vec![],
        secrets: Default::default(),
        environments: indexmap::IndexMap::new(),
    };
    let result = build_execution_order(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown"));
}

#[test]
fn test_fj132_kahn_sort_diamond_dependency() {
    // A -> B, A -> C, B -> D, C -> D (diamond shape)
    let mut resources = indexmap::IndexMap::new();
    let mut r_a = make_base_resource();
    r_a.resource_type = ResourceType::Package;
    resources.insert("a".to_string(), r_a);
    let mut r_b = make_base_resource();
    r_b.resource_type = ResourceType::Package;
    r_b.depends_on = vec!["a".to_string()];
    resources.insert("b".to_string(), r_b);
    let mut r_c = make_base_resource();
    r_c.resource_type = ResourceType::Package;
    r_c.depends_on = vec!["a".to_string()];
    resources.insert("c".to_string(), r_c);
    let mut r_d = make_base_resource();
    r_d.resource_type = ResourceType::Package;
    r_d.depends_on = vec!["b".to_string(), "c".to_string()];
    resources.insert("d".to_string(), r_d);
    let config = ForjarConfig {
        version: "1.0".to_string(),
        name: "test".to_string(),
        description: None,
        params: HashMap::new(),
        machines: indexmap::IndexMap::new(),
        resources,
        policy: Policy::default(),
        outputs: indexmap::IndexMap::new(),
        policies: vec![],
        data: indexmap::IndexMap::new(),
        includes: vec![],
        include_provenance: HashMap::new(),
        checks: indexmap::IndexMap::new(),
        moved: vec![],
        secrets: Default::default(),
        environments: indexmap::IndexMap::new(),
    };
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order, vec!["a", "b", "c", "d"]);
}

#[test]
fn test_fj132_build_execution_order_empty() {
    let config = ForjarConfig {
        version: "1.0".to_string(),
        name: "test".to_string(),
        description: None,
        params: HashMap::new(),
        machines: indexmap::IndexMap::new(),
        resources: indexmap::IndexMap::new(),
        policy: Policy::default(),
        outputs: indexmap::IndexMap::new(),
        policies: vec![],
        data: indexmap::IndexMap::new(),
        includes: vec![],
        include_provenance: HashMap::new(),
        checks: indexmap::IndexMap::new(),
        moved: vec![],
        secrets: Default::default(),
        environments: indexmap::IndexMap::new(),
    };
    let order = build_execution_order(&config).unwrap();
    assert!(order.is_empty());
}

#[test]
fn test_fj132_unclosed_template() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("hello {{params.name", &params, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unclosed template"));
}

#[test]
fn test_fj132_resolve_template_secret_missing_error() {
    let params = HashMap::new();
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("token={{secrets.zzz-missing-99}}", &params, &machines);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("FORJAR_SECRET_ZZZ_MISSING_99"));
}

#[test]
fn test_resolve_template_nested_braces() {
    let mut params = HashMap::new();
    params.insert(
        "x".to_string(),
        serde_yaml_ng::Value::String("value".to_string()),
    );
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{params.x}}_suffix", &params, &machines).unwrap();
    assert_eq!(result, "value_suffix");
}

#[test]
fn test_resolve_template_multiple() {
    let mut params = HashMap::new();
    params.insert(
        "a".to_string(),
        serde_yaml_ng::Value::String("hello".to_string()),
    );
    params.insert(
        "b".to_string(),
        serde_yaml_ng::Value::String("world".to_string()),
    );
    let machines = indexmap::IndexMap::new();
    let result = resolve_template("{{params.a}}-{{params.b}}", &params, &machines).unwrap();
    assert_eq!(result, "hello-world");
}

#[test]
fn test_build_execution_order_empty() {
    // Empty resources with a machine
    let config = dag_config(&[], &[]);
    let order = build_execution_order(&config).unwrap();
    assert!(order.is_empty());
}

#[test]
fn test_build_execution_order_no_deps() {
    // Three independent resources -- alphabetical tie-breaking
    let config = dag_config(&["alpha", "beta", "gamma"], &[]);
    let order = build_execution_order(&config).unwrap();
    assert_eq!(order.len(), 3);
    assert_eq!(order, vec!["alpha", "beta", "gamma"]);
}

// ================================================================
// FJ-216: parallel wave computation tests
// ================================================================
