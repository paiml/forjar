//! Tests: FJ-1433 post-quantum dual signing.

#![allow(unused_imports)]
use super::pq_signing::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_sign() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("recipe.yaml");
        std::fs::write(&f, "test content").unwrap();
        let sig = dual_sign(&f, "test-signer").unwrap();
        assert_eq!(sig.classical_alg, "blake3-hmac");
        assert_eq!(sig.pq_alg, "slh-dsa-blake3-placeholder");
        assert_eq!(sig.blake3_hash.len(), 64);
    }

    #[test]
    fn test_dual_verify_valid() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("test.yaml");
        std::fs::write(&f, "content").unwrap();
        dual_sign(&f, "signer").unwrap();
        let result = dual_verify(&f).unwrap();
        assert!(result.both_valid);
        assert!(result.classical_valid);
        assert!(result.pq_valid);
    }

    #[test]
    fn test_dual_verify_tampered() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("test.yaml");
        std::fs::write(&f, "original").unwrap();
        dual_sign(&f, "signer").unwrap();
        std::fs::write(&f, "tampered").unwrap();
        let result = dual_verify(&f).unwrap();
        assert!(!result.both_valid);
    }

    #[test]
    fn test_dual_verify_no_sig() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("nosig.yaml");
        std::fs::write(&f, "test").unwrap();
        let result = dual_verify(&f).unwrap();
        assert!(!result.both_valid);
    }

    #[test]
    fn test_cmd_dual_sign() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("recipe.yaml");
        std::fs::write(&f, "test").unwrap();
        let result = cmd_dual_sign(&f, false, Some("ci"), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dual_signature_serde() {
        let sig = DualSignature {
            path: "/tmp/test.yaml".to_string(),
            blake3_hash: "a".repeat(64),
            classical_sig: "b".repeat(64),
            classical_alg: "blake3-hmac".to_string(),
            pq_sig: "c".repeat(64),
            pq_alg: "slh-dsa".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&sig).unwrap();
        let round: DualSignature = serde_json::from_str(&json).unwrap();
        assert_eq!(round.pq_alg, "slh-dsa");
    }
}
