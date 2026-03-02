//! Tests for FJ-1302: profile generations.

use super::profile::{create_generation, current_generation, list_generations, rollback};

#[test]
fn test_fj1302_first_generation() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    let gen = create_generation(&profiles, "/store/abc123").unwrap();
    assert_eq!(gen, 0, "first generation must be 0");
}

#[test]
fn test_fj1302_multiple_generations() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    let g0 = create_generation(&profiles, "/store/aaa").unwrap();
    let g1 = create_generation(&profiles, "/store/bbb").unwrap();
    let g2 = create_generation(&profiles, "/store/ccc").unwrap();
    assert_eq!(g0, 0);
    assert_eq!(g1, 1);
    assert_eq!(g2, 2);
}

#[test]
fn test_fj1302_current_follows_latest() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    create_generation(&profiles, "/store/aaa").unwrap();
    assert_eq!(current_generation(&profiles), Some(0));

    create_generation(&profiles, "/store/bbb").unwrap();
    assert_eq!(current_generation(&profiles), Some(1));
}

#[test]
fn test_fj1302_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    create_generation(&profiles, "/store/aaa").unwrap();
    create_generation(&profiles, "/store/bbb").unwrap();
    assert_eq!(current_generation(&profiles), Some(1));

    let rolled = rollback(&profiles).unwrap();
    assert_eq!(rolled, 0);
    assert_eq!(current_generation(&profiles), Some(0));
}

#[test]
fn test_fj1302_rollback_no_previous() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    create_generation(&profiles, "/store/aaa").unwrap();
    let result = rollback(&profiles);
    assert!(result.is_err(), "cannot rollback past generation 0");
}

#[test]
fn test_fj1302_rollback_empty() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    std::fs::create_dir_all(&profiles).unwrap();
    let result = rollback(&profiles);
    assert!(result.is_err(), "rollback on empty profiles must fail");
}

#[test]
fn test_fj1302_list_generations() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    create_generation(&profiles, "/store/aaa").unwrap();
    create_generation(&profiles, "/store/bbb").unwrap();
    let gens = list_generations(&profiles).unwrap();
    assert_eq!(gens.len(), 2);
    assert_eq!(gens[0].0, 0);
    assert_eq!(gens[0].1, "/store/aaa");
    assert_eq!(gens[1].0, 1);
    assert_eq!(gens[1].1, "/store/bbb");
}

#[test]
fn test_fj1302_list_empty() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("nonexistent");
    let gens = list_generations(&profiles).unwrap();
    assert!(gens.is_empty());
}

#[test]
fn test_fj1302_current_generation_empty() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("empty");
    std::fs::create_dir_all(&profiles).unwrap();
    assert_eq!(current_generation(&profiles), None);
}

#[test]
fn test_fj1302_generation_target_stored() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    create_generation(&profiles, "/store/deadbeef").unwrap();
    let target_content = std::fs::read_to_string(profiles.join("0").join("target")).unwrap();
    assert_eq!(target_content, "/store/deadbeef");
}
