//! Tests: FJ-1432 recipe signing.

#![allow(unused_imports)]
use super::recipe_signing::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_recipe() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "version: \"1.0\"\nname: test\n").unwrap();
        let sig = sign_recipe(&recipe, "test-user").unwrap();
        assert_eq!(sig.algorithm, "blake3-hmac");
        assert_eq!(sig.signer, "test-user");
        assert_eq!(sig.blake3_hash.len(), 64);
    }

    #[test]
    fn test_verify_recipe_valid() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "version: \"1.0\"\nname: test\n").unwrap();
        sign_recipe(&recipe, "signer").unwrap();
        let result = verify_recipe(&recipe).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_verify_recipe_tampered() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "version: \"1.0\"\nname: test\n").unwrap();
        sign_recipe(&recipe, "signer").unwrap();
        std::fs::write(&recipe, "version: \"2.0\"\nname: hacked\n").unwrap();
        let result = verify_recipe(&recipe).unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn test_verify_no_signature() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "test").unwrap();
        let result = verify_recipe(&recipe).unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn test_cmd_recipe_sign() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "test content").unwrap();
        let result = cmd_recipe_sign(&recipe, false, Some("ci"), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_recipe_sign_verify_json() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "test content").unwrap();
        // Sign first, then verify
        cmd_recipe_sign(&recipe, false, Some("ci"), false).unwrap();
        let result = cmd_recipe_sign(&recipe, true, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_recipe_sign_verify_text() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "test content").unwrap();
        cmd_recipe_sign(&recipe, false, Some("ci"), false).unwrap();
        let result = cmd_recipe_sign(&recipe, true, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_recipe_sign_verify_tampered() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "original").unwrap();
        cmd_recipe_sign(&recipe, false, Some("ci"), false).unwrap();
        std::fs::write(&recipe, "tampered").unwrap();
        let result = cmd_recipe_sign(&recipe, true, None, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_recipe_sign_default_signer() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "test").unwrap();
        // signer=None defaults to "local"
        let result = cmd_recipe_sign(&recipe, false, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_recipe_sign_text_output() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "content").unwrap();
        let result = cmd_recipe_sign(&recipe, false, Some("dev"), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_result_empty_signer() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = dir.path().join("recipe.yaml");
        std::fs::write(&recipe, "test").unwrap();
        // No sig file → valid=false, signer=""
        let result = verify_recipe(&recipe).unwrap();
        assert!(!result.valid);
        assert!(result.signer.is_empty());
    }

    #[test]
    fn test_recipe_signature_serde() {
        let sig = RecipeSignature {
            recipe_path: "/tmp/r.yaml".to_string(),
            blake3_hash: "a".repeat(64),
            algorithm: "blake3-hmac".to_string(),
            signer: "test".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            signature: "b".repeat(64),
        };
        let json = serde_json::to_string(&sig).unwrap();
        let round: RecipeSignature = serde_json::from_str(&json).unwrap();
        assert_eq!(round.signer, "test");
    }
}
