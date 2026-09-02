//! Unit tests for `cli::recipe_digest` (paiml/forjar#405).
//!
//! Replaces `tests_recipe_signing` / `tests_recipe_signing_cov` /
//! `tests_pq_signing*`, which between them had ~40 tests and could not fail on
//! a forged signature, because the code under test never read one. The nearest
//! thing they had was `test_cmd_recipe_sign_verify_tampered`, which mutated
//! the RECIPE — the one input that was actually checked.
//!
//! The end-to-end falsifiers live in
//! `tests/falsification_e03_signatures_are_read.rs`.

use super::recipe_digest::*;

fn fixture(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, content).unwrap();
    (dir, recipe)
}

#[test]
fn digest_records_the_blake3_of_the_file() {
    let (dir, recipe) = fixture("version: \"1.0\"\n");
    let d = digest_recipe(&recipe).unwrap();
    assert_eq!(
        d.blake3_hash,
        blake3::hash(b"version: \"1.0\"\n").to_hex().to_string()
    );
    assert_eq!(d.algorithm, "blake3");
    assert!(dir.path().join("recipe.digest.json").exists());
}

#[test]
fn the_sidecar_carries_no_signature_or_signer_field() {
    let (dir, recipe) = fixture("x\n");
    digest_recipe(&recipe).unwrap();
    let text = std::fs::read_to_string(dir.path().join("recipe.digest.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let obj = v.as_object().unwrap();
    assert!(!obj.contains_key("signature"));
    assert!(!obj.contains_key("signer"));
    assert!(!text.to_lowercase().contains("hmac"));
}

#[test]
fn a_fresh_digest_verifies() {
    let (_dir, recipe) = fixture("hello\n");
    digest_recipe(&recipe).unwrap();
    let r = verify_digest(&recipe).unwrap();
    assert!(r.valid);
    assert_eq!(r.reason, "digest matches");
    assert_eq!(r.algorithm, "blake3");
}

#[test]
fn a_modified_recipe_does_not_verify() {
    let (_dir, recipe) = fixture("original\n");
    digest_recipe(&recipe).unwrap();
    std::fs::write(&recipe, "tampered\n").unwrap();
    let r = verify_digest(&recipe).unwrap();
    assert!(!r.valid);
    assert!(r.reason.contains("mismatch"));
}

#[test]
fn a_mutated_recorded_hash_does_not_verify() {
    let (dir, recipe) = fixture("payload\n");
    digest_recipe(&recipe).unwrap();
    let sidecar = dir.path().join("recipe.digest.json");
    let text = std::fs::read_to_string(&sidecar).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let recorded = v["blake3_hash"].as_str().unwrap().to_string();
    let mut chars: Vec<char> = recorded.chars().collect();
    chars[0] = if chars[0] == 'a' { 'b' } else { 'a' };
    v["blake3_hash"] = serde_json::Value::String(chars.into_iter().collect());
    std::fs::write(&sidecar, v.to_string()).unwrap();

    let r = verify_digest(&recipe).unwrap();
    assert!(!r.valid, "a one-byte mutation of the recorded hash verified");
}

#[test]
fn verify_without_a_sidecar_is_not_valid() {
    let (_dir, recipe) = fixture("no sidecar\n");
    let r = verify_digest(&recipe).unwrap();
    assert!(!r.valid);
    assert_eq!(r.reason, "no digest file found");
    assert!(r.algorithm.is_empty());
}

#[test]
fn digest_of_a_missing_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let err = digest_recipe(&dir.path().join("absent.yaml")).unwrap_err();
    assert!(err.starts_with("read recipe:"), "{err}");
}

#[test]
fn verify_with_an_unparseable_sidecar_is_an_error() {
    let (dir, recipe) = fixture("x\n");
    std::fs::write(dir.path().join("recipe.digest.json"), "{ not json").unwrap();
    let err = verify_digest(&recipe).unwrap_err();
    assert!(err.starts_with("parse digest:"), "{err}");
}

#[test]
fn cmd_records_then_verifies_in_both_output_modes() {
    let (_dir, recipe) = fixture("cmd\n");
    assert!(cmd_recipe_digest(&recipe, false, false).is_ok());
    assert!(cmd_recipe_digest(&recipe, false, true).is_ok());
    assert!(cmd_recipe_digest(&recipe, true, false).is_ok());
    assert!(cmd_recipe_digest(&recipe, true, true).is_ok());
}

#[test]
fn cmd_verify_returns_err_when_the_recipe_changed() {
    let (_dir, recipe) = fixture("before\n");
    cmd_recipe_digest(&recipe, false, false).unwrap();
    std::fs::write(&recipe, "after\n").unwrap();
    let err = cmd_recipe_digest(&recipe, true, false).unwrap_err();
    assert_eq!(err, "digest verification failed");
    let err = cmd_recipe_digest(&recipe, true, true).unwrap_err();
    assert_eq!(err, "digest verification failed");
}

#[test]
fn the_digest_round_trips_through_serde() {
    let d = RecipeDigest {
        recipe_path: "r.yaml".to_string(),
        blake3_hash: "ab".repeat(32),
        algorithm: "blake3".to_string(),
        timestamp: "t".to_string(),
    };
    let json = serde_json::to_string(&d).unwrap();
    let back: RecipeDigest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.blake3_hash, d.blake3_hash);
    assert_eq!(back.algorithm, "blake3");
    assert!(format!("{back:?}").contains("RecipeDigest"));
}

#[test]
fn the_verify_result_serialises_valid_as_a_bool() {
    let (_dir, recipe) = fixture("v\n");
    digest_recipe(&recipe).unwrap();
    let r = verify_digest(&recipe).unwrap();
    let json = serde_json::to_string(&r.clone()).unwrap();
    assert!(json.contains("\"valid\":true"), "{json}");
    assert!(format!("{r:?}").contains("VerifyResult"));
}
