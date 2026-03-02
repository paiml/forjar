//! Tests for FJ-1307: Input closure tracking.

use super::closure::{all_closures, closure_hash, input_closure, ResourceInputs};
use std::collections::BTreeMap;

fn simple_graph() -> BTreeMap<String, ResourceInputs> {
    let mut g = BTreeMap::new();
    g.insert(
        "a".to_string(),
        ResourceInputs {
            input_hashes: vec!["blake3:aaa".to_string()],
            depends_on: vec![],
        },
    );
    g.insert(
        "b".to_string(),
        ResourceInputs {
            input_hashes: vec!["blake3:bbb".to_string()],
            depends_on: vec!["a".to_string()],
        },
    );
    g.insert(
        "c".to_string(),
        ResourceInputs {
            input_hashes: vec!["blake3:ccc".to_string()],
            depends_on: vec!["b".to_string()],
        },
    );
    g
}

#[test]
fn test_fj1307_closure_leaf_node() {
    let g = simple_graph();
    let closure = input_closure("a", &g);
    assert_eq!(closure, vec!["blake3:aaa"]);
}

#[test]
fn test_fj1307_closure_one_dep() {
    let g = simple_graph();
    let closure = input_closure("b", &g);
    assert_eq!(closure, vec!["blake3:aaa", "blake3:bbb"]);
}

#[test]
fn test_fj1307_closure_transitive() {
    let g = simple_graph();
    let closure = input_closure("c", &g);
    assert_eq!(closure, vec!["blake3:aaa", "blake3:bbb", "blake3:ccc"]);
}

#[test]
fn test_fj1307_closure_unknown_resource() {
    let g = simple_graph();
    let closure = input_closure("unknown", &g);
    assert!(closure.is_empty());
}

#[test]
fn test_fj1307_closure_deduplicates() {
    let mut g = BTreeMap::new();
    g.insert(
        "shared".to_string(),
        ResourceInputs {
            input_hashes: vec!["blake3:shared".to_string()],
            depends_on: vec![],
        },
    );
    g.insert(
        "a".to_string(),
        ResourceInputs {
            input_hashes: vec!["blake3:aaa".to_string()],
            depends_on: vec!["shared".to_string()],
        },
    );
    g.insert(
        "b".to_string(),
        ResourceInputs {
            input_hashes: vec!["blake3:bbb".to_string()],
            depends_on: vec!["shared".to_string()],
        },
    );
    g.insert(
        "top".to_string(),
        ResourceInputs {
            input_hashes: vec![],
            depends_on: vec!["a".to_string(), "b".to_string()],
        },
    );
    let closure = input_closure("top", &g);
    // "shared" should appear once, not twice
    assert_eq!(closure, vec!["blake3:aaa", "blake3:bbb", "blake3:shared"]);
}

#[test]
fn test_fj1307_closure_cycle_protection() {
    let mut g = BTreeMap::new();
    g.insert(
        "a".to_string(),
        ResourceInputs {
            input_hashes: vec!["blake3:aaa".to_string()],
            depends_on: vec!["b".to_string()],
        },
    );
    g.insert(
        "b".to_string(),
        ResourceInputs {
            input_hashes: vec!["blake3:bbb".to_string()],
            depends_on: vec!["a".to_string()],
        },
    );
    // Should not infinite loop
    let closure = input_closure("a", &g);
    assert_eq!(closure, vec!["blake3:aaa", "blake3:bbb"]);
}

#[test]
fn test_fj1307_closure_sorted() {
    let mut g = BTreeMap::new();
    g.insert(
        "x".to_string(),
        ResourceInputs {
            input_hashes: vec![
                "blake3:zzz".to_string(),
                "blake3:aaa".to_string(),
                "blake3:mmm".to_string(),
            ],
            depends_on: vec![],
        },
    );
    let closure = input_closure("x", &g);
    assert_eq!(closure, vec!["blake3:aaa", "blake3:mmm", "blake3:zzz"]);
}

#[test]
fn test_fj1307_closure_hash_deterministic() {
    let g = simple_graph();
    let c1 = input_closure("c", &g);
    let c2 = input_closure("c", &g);
    assert_eq!(closure_hash(&c1), closure_hash(&c2));
}

#[test]
fn test_fj1307_closure_hash_different() {
    let g = simple_graph();
    let ca = input_closure("a", &g);
    let cb = input_closure("b", &g);
    assert_ne!(closure_hash(&ca), closure_hash(&cb));
}

#[test]
fn test_fj1307_closure_hash_starts_with_blake3() {
    let closure = vec!["blake3:aaa".to_string()];
    let hash = closure_hash(&closure);
    assert!(hash.starts_with("blake3:"));
}

#[test]
fn test_fj1307_all_closures() {
    let g = simple_graph();
    let closures = all_closures(&g);
    assert_eq!(closures.len(), 3);
    assert_eq!(closures["a"], vec!["blake3:aaa"]);
    assert_eq!(closures["b"], vec!["blake3:aaa", "blake3:bbb"]);
    assert_eq!(
        closures["c"],
        vec!["blake3:aaa", "blake3:bbb", "blake3:ccc"]
    );
}

#[test]
fn test_fj1307_empty_graph() {
    let g: BTreeMap<String, ResourceInputs> = BTreeMap::new();
    let closures = all_closures(&g);
    assert!(closures.is_empty());
}

#[test]
fn test_fj1307_empty_closure_hash() {
    let hash = closure_hash(&[]);
    assert!(hash.starts_with("blake3:"));
}
