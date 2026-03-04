//! Tests for FJ-1240: State encryption with age.

#![allow(unused_imports)]
use super::*;
use std::path::Path;

#[test]
fn test_walk_yaml_files_finds_state_files() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();
    let m1 = state.join("m1");
    std::fs::create_dir_all(&m1).unwrap();

    std::fs::write(m1.join("state.lock.yaml"), "test").unwrap();
    std::fs::write(m1.join("last-apply.yaml"), "test").unwrap();
    std::fs::write(state.join("forjar.lock.yaml"), "test").unwrap();
    std::fs::write(state.join("not-yaml.txt"), "test").unwrap();

    let files = walk_yaml_files(state);
    assert_eq!(files.len(), 3, "should find 3 yaml files, got {files:?}");
}

#[test]
fn test_walk_yaml_files_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let files = walk_yaml_files(dir.path());
    assert!(files.is_empty());
}

#[test]
fn test_walk_age_files_finds_encrypted_files() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path();
    let m1 = state.join("m1");
    std::fs::create_dir_all(&m1).unwrap();

    std::fs::write(m1.join("state.lock.yaml.age"), "encrypted").unwrap();
    std::fs::write(state.join("forjar.lock.yaml.age"), "encrypted").unwrap();
    std::fs::write(state.join("state.lock.yaml"), "not encrypted").unwrap();

    let files = walk_age_files(state);
    assert_eq!(files.len(), 2, "should find 2 .age files, got {files:?}");
}

#[test]
fn test_encrypt_state_files_requires_env_var() {
    let dir = tempfile::tempdir().unwrap();
    std::env::remove_var("FORJAR_AGE_KEY");
    let result = encrypt_state_files(dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("FORJAR_AGE_KEY"));
}

#[test]
fn test_decrypt_state_files_requires_env_var() {
    let dir = tempfile::tempdir().unwrap();
    std::env::remove_var("FORJAR_AGE_IDENTITY");
    let result = decrypt_state_files(dir.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("FORJAR_AGE_IDENTITY"));
}
