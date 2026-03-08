//! Tests for FJ-2701 input/output tracking.

use super::io_tracking::*;

#[test]
fn hash_inputs_empty_patterns() {
    let result = hash_inputs(&[], std::path::Path::new("/tmp")).unwrap();
    assert!(result.is_none());
}

#[test]
fn hash_inputs_single_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "hello world").unwrap();

    let patterns = vec!["test.txt".to_string()];
    let result = hash_inputs(&patterns, dir.path()).unwrap();
    assert!(result.is_some());
    assert!(result.unwrap().starts_with("blake3:"));
}

#[test]
fn hash_inputs_glob_pattern() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.rs"), "fn a() {}").unwrap();
    std::fs::write(dir.path().join("b.rs"), "fn b() {}").unwrap();
    std::fs::write(dir.path().join("c.txt"), "text").unwrap();

    let patterns = vec!["*.rs".to_string()];
    let result = hash_inputs(&patterns, dir.path()).unwrap();
    assert!(result.is_some());
}

#[test]
fn hash_inputs_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("x.txt"), "content").unwrap();
    std::fs::write(dir.path().join("y.txt"), "other").unwrap();

    let patterns = vec!["*.txt".to_string()];
    let h1 = hash_inputs(&patterns, dir.path()).unwrap();
    let h2 = hash_inputs(&patterns, dir.path()).unwrap();
    assert_eq!(h1, h2);
}

#[test]
fn hash_inputs_detects_change() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("data.txt");
    std::fs::write(&file, "version 1").unwrap();

    let patterns = vec!["data.txt".to_string()];
    let h1 = hash_inputs(&patterns, dir.path()).unwrap();

    std::fs::write(&file, "version 2").unwrap();
    let h2 = hash_inputs(&patterns, dir.path()).unwrap();

    assert_ne!(h1, h2);
}

#[test]
fn hash_inputs_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let patterns = vec!["*.nonexistent".to_string()];
    let result = hash_inputs(&patterns, dir.path()).unwrap();
    assert!(result.is_none());
}

#[test]
fn hash_outputs_empty() {
    let result = hash_outputs(&[]).unwrap();
    assert!(result.is_none());
}

#[test]
fn hash_outputs_single_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("output.bin");
    std::fs::write(&file, b"binary data").unwrap();

    let artifacts = vec![file.to_string_lossy().to_string()];
    let result = hash_outputs(&artifacts).unwrap();
    assert!(result.is_some());
    assert!(result.unwrap().starts_with("blake3:"));
}

#[test]
fn hash_outputs_missing_file_ok() {
    let artifacts = vec!["/tmp/does-not-exist-xyz123".to_string()];
    let result = hash_outputs(&artifacts).unwrap();
    assert!(result.is_none()); // missing files silently skipped
}

#[test]
fn hash_outputs_directory() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("artifacts");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("model.bin"), b"weights").unwrap();

    let artifacts = vec![sub.to_string_lossy().to_string()];
    let result = hash_outputs(&artifacts).unwrap();
    assert!(result.is_some());
}

#[test]
fn should_skip_no_cache() {
    assert!(!should_skip_cached(false, None, None));
    assert!(!should_skip_cached(false, Some("a"), Some("a")));
}

#[test]
fn should_skip_cache_match() {
    assert!(should_skip_cached(
        true,
        Some("blake3:abc"),
        Some("blake3:abc")
    ));
}

#[test]
fn should_skip_cache_mismatch() {
    assert!(!should_skip_cached(
        true,
        Some("blake3:abc"),
        Some("blake3:def")
    ));
}

#[test]
fn should_skip_cache_no_stored() {
    assert!(!should_skip_cached(true, Some("blake3:abc"), None));
}

#[test]
fn should_skip_cache_no_current() {
    assert!(!should_skip_cached(true, None, Some("blake3:abc")));
}

#[test]
fn hash_inputs_multiple_patterns() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("code.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.path().join("data.json"), "{}").unwrap();

    let patterns = vec!["*.rs".to_string(), "*.json".to_string()];
    let result = hash_inputs(&patterns, dir.path()).unwrap();
    assert!(result.is_some());
}

#[test]
fn hash_inputs_deduplicates() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("file.txt"), "content").unwrap();

    // Same file matched by two patterns
    let patterns = vec!["file.txt".to_string(), "*.txt".to_string()];
    let h1 = hash_inputs(&patterns, dir.path()).unwrap();

    let patterns2 = vec!["file.txt".to_string()];
    let h2 = hash_inputs(&patterns2, dir.path()).unwrap();

    // Deduplication means both should produce the same hash
    assert_eq!(h1, h2);
}

#[test]
fn hash_outputs_detects_change() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("out.bin");

    std::fs::write(&file, b"v1").unwrap();
    let artifacts = vec![file.to_string_lossy().to_string()];
    let h1 = hash_outputs(&artifacts).unwrap();

    std::fs::write(&file, b"v2").unwrap();
    let h2 = hash_outputs(&artifacts).unwrap();

    assert_ne!(h1, h2);
}

#[test]
fn hash_inputs_absolute_path() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("abs.txt");
    std::fs::write(&file, "absolute").unwrap();

    let patterns = vec![file.to_string_lossy().to_string()];
    let result = hash_inputs(&patterns, dir.path()).unwrap();
    assert!(result.is_some());
}

#[test]
fn hash_inputs_invalid_glob() {
    let dir = tempfile::tempdir().unwrap();
    let patterns = vec!["[invalid".to_string()];
    let result = hash_inputs(&patterns, dir.path());
    assert!(result.is_err());
}
