//! Coverage tests for recipe_signing.rs — sign, verify, cmd.

use super::recipe_signing::*;

// ── sign_recipe ─────────────────────────────────────────────────────

#[test]
fn sign_creates_sig_file() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "version: 1.0\nname: test\n").unwrap();
    let sig = sign_recipe(&recipe, "tester").unwrap();
    assert_eq!(sig.signer, "tester");
    assert_eq!(sig.algorithm, "blake3-hmac");
    assert!(!sig.blake3_hash.is_empty());
    assert!(!sig.signature.is_empty());
    assert!(recipe.with_extension("sig.json").exists());
}

#[test]
fn sign_deterministic_hash() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "content: hello").unwrap();
    let sig1 = sign_recipe(&recipe, "signer").unwrap();
    let sig2 = sign_recipe(&recipe, "signer").unwrap();
    assert_eq!(sig1.blake3_hash, sig2.blake3_hash);
    assert_eq!(sig1.signature, sig2.signature);
}

#[test]
fn sign_different_signer_different_sig() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "content: hello").unwrap();
    let sig1 = sign_recipe(&recipe, "alice").unwrap();
    let sig2 = sign_recipe(&recipe, "bob").unwrap();
    assert_eq!(sig1.blake3_hash, sig2.blake3_hash);
    assert_ne!(sig1.signature, sig2.signature);
}

#[test]
fn sign_nonexistent_file() {
    let result = sign_recipe(std::path::Path::new("/nonexistent/recipe.yaml"), "x");
    assert!(result.is_err());
}

// ── verify_recipe ───────────────────────────────────────────────────

#[test]
fn verify_valid_signature() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "version: 1.0\n").unwrap();
    sign_recipe(&recipe, "tester").unwrap();
    let result = verify_recipe(&recipe).unwrap();
    assert!(result.valid);
    assert_eq!(result.signer, "tester");
    assert!(result.reason.contains("matches"));
}

#[test]
fn verify_tampered_recipe() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "version: 1.0\n").unwrap();
    sign_recipe(&recipe, "tester").unwrap();
    // Tamper with the recipe
    std::fs::write(&recipe, "version: 2.0\n").unwrap();
    let result = verify_recipe(&recipe).unwrap();
    assert!(!result.valid);
    assert!(result.reason.contains("mismatch"));
}

#[test]
fn verify_no_signature_file() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "version: 1.0\n").unwrap();
    let result = verify_recipe(&recipe).unwrap();
    assert!(!result.valid);
    assert!(result.reason.contains("no signature"));
}

// ── cmd_recipe_sign ─────────────────────────────────────────────────

#[test]
fn cmd_sign_text_output() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "name: test\n").unwrap();
    assert!(cmd_recipe_sign(&recipe, false, Some("admin"), false).is_ok());
}

#[test]
fn cmd_sign_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "name: test\n").unwrap();
    assert!(cmd_recipe_sign(&recipe, false, Some("admin"), true).is_ok());
}

#[test]
fn cmd_sign_default_signer() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "name: test\n").unwrap();
    assert!(cmd_recipe_sign(&recipe, false, None, false).is_ok());
}

#[test]
fn cmd_verify_valid_text() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "name: test\n").unwrap();
    sign_recipe(&recipe, "ci").unwrap();
    assert!(cmd_recipe_sign(&recipe, true, None, false).is_ok());
}

#[test]
fn cmd_verify_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "name: test\n").unwrap();
    sign_recipe(&recipe, "ci").unwrap();
    assert!(cmd_recipe_sign(&recipe, true, None, true).is_ok());
}

#[test]
fn cmd_verify_fails_no_sig() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "name: test\n").unwrap();
    let result = cmd_recipe_sign(&recipe, true, None, false);
    assert!(result.is_err());
}

#[test]
fn cmd_verify_fails_tampered() {
    let dir = tempfile::tempdir().unwrap();
    let recipe = dir.path().join("recipe.yaml");
    std::fs::write(&recipe, "name: original\n").unwrap();
    sign_recipe(&recipe, "admin").unwrap();
    std::fs::write(&recipe, "name: tampered\n").unwrap();
    let result = cmd_recipe_sign(&recipe, true, None, false);
    assert!(result.is_err());
}

// ── RecipeSignature serde ───────────────────────────────────────────

#[test]
fn signature_serde_roundtrip() {
    let sig = RecipeSignature {
        recipe_path: "/etc/forjar/recipe.yaml".to_string(),
        blake3_hash: "abc123".to_string(),
        algorithm: "blake3-hmac".to_string(),
        signer: "ci-bot".to_string(),
        timestamp: "2026-01-01".to_string(),
        signature: "def456".to_string(),
    };
    let json = serde_json::to_string(&sig).unwrap();
    let deserialized: RecipeSignature = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.blake3_hash, "abc123");
    assert_eq!(deserialized.signer, "ci-bot");
}
