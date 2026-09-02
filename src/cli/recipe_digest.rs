//! Recipe content digest — tamper EVIDENCE, not a signature.
//!
//! Was `cli::recipe_signing` (FJ-1432), which shipped a `sign` verb whose
//! `--verify` deserialised `signature` and `signer` and then never read either
//! one: it compared `current_hash == sig.blake3_hash` and reported
//! `valid: true`. Setting the sidecar's `signature` to `"deadbeef"` and its
//! `signer` to `"root@prod"` still verified. See paiml/forjar#405 (audit E03).
//!
//! What the code did — and all it ever did — is record a BLAKE3 hash of a file
//! and re-check it later. That is worth having and this module keeps it, under
//! the name it earns. What it is NOT:
//!
//!   * It is not a signature. There is no key. Anyone who can edit the recipe
//!     can recompute the sidecar beside it, which is why the `signature`,
//!     `signer` and `algorithm: blake3-hmac` fields are gone rather than
//!     merely unverified — a consumer reading them was reading a lie.
//!   * It is therefore not an authenticity check, and cannot gate an apply
//!     against an attacker with write access.
//!
//! For a keyed check over state locks, forjar has `lock-sign` /
//! `lock-verify-sig`, which recomputes `blake3(content ++ key)` and compares
//! it. That one is real, within the limits of a shared secret.

use std::path::Path;

/// The only algorithm `digest` computes. Recorded, and required on verify.
const ALGORITHM: &str = "blake3";

/// A recorded content digest of a recipe.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecipeDigest {
    pub recipe_path: String,
    pub blake3_hash: String,
    pub algorithm: String,
    pub timestamp: String,
}

/// Digest verification result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VerifyResult {
    pub recipe_path: String,
    pub valid: bool,
    pub algorithm: String,
    pub reason: String,
}

/// Sidecar path for a recipe: `f.yaml` -> `f.digest.json`.
fn sidecar_path(recipe_path: &Path) -> std::path::PathBuf {
    recipe_path.with_extension("digest.json")
}

/// Record a recipe's BLAKE3 digest in a sidecar next to it.
pub fn digest_recipe(recipe_path: &Path) -> Result<RecipeDigest, String> {
    let content = std::fs::read(recipe_path).map_err(|e| format!("read recipe: {e}"))?;
    let blake3_hash = blake3::hash(&content).to_hex().to_string();

    let digest = RecipeDigest {
        recipe_path: recipe_path.display().to_string(),
        blake3_hash,
        algorithm: ALGORITHM.to_string(),
        timestamp: format!("{:?}", std::time::SystemTime::now()),
    };

    let data = serde_json::to_string_pretty(&digest).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(sidecar_path(recipe_path), data).map_err(|e| format!("write digest: {e}"))?;

    Ok(digest)
}

/// Re-check a recipe against its recorded digest.
pub fn verify_digest(recipe_path: &Path) -> Result<VerifyResult, String> {
    let path = sidecar_path(recipe_path);
    if !path.exists() {
        return Ok(VerifyResult {
            recipe_path: recipe_path.display().to_string(),
            valid: false,
            algorithm: String::new(),
            reason: "no digest file found".to_string(),
        });
    }

    let data = std::fs::read_to_string(&path).map_err(|e| format!("read digest: {e}"))?;
    let recorded: RecipeDigest =
        serde_json::from_str(&data).map_err(|e| format!("parse digest: {e}"))?;

    // The check below is BLAKE3 and only BLAKE3. A sidecar that names any other
    // algorithm has not been checked by it, so it cannot be reported valid, and
    // the name it carries is never echoed into the result — the whole point of
    // paiml/forjar#405 is that an unverified sidecar field must not appear
    // inside a verification verdict.
    if recorded.algorithm != ALGORITHM {
        return Ok(VerifyResult {
            recipe_path: recipe_path.display().to_string(),
            valid: false,
            algorithm: ALGORITHM.to_string(),
            reason: format!(
                "digest file records an unsupported algorithm; forjar digest only computes {ALGORITHM}"
            ),
        });
    }

    let content = std::fs::read(recipe_path).map_err(|e| format!("read recipe: {e}"))?;
    let current_hash = blake3::hash(&content).to_hex().to_string();

    let valid = current_hash == recorded.blake3_hash;
    Ok(VerifyResult {
        recipe_path: recipe_path.display().to_string(),
        valid,
        algorithm: ALGORITHM.to_string(),
        reason: if valid {
            "digest matches".to_string()
        } else {
            "digest mismatch — recipe or digest file changed since recording".to_string()
        },
    })
}

/// `forjar digest <RECIPE> [--verify] [--json]`.
pub fn cmd_recipe_digest(recipe_path: &Path, verify_only: bool, json: bool) -> Result<(), String> {
    if verify_only {
        let result = verify_digest(recipe_path)?;
        if json {
            let out =
                serde_json::to_string_pretty(&result).map_err(|e| format!("JSON error: {e}"))?;
            println!("{out}");
        } else {
            let icon = if result.valid { "OK" } else { "FAIL" };
            println!("[{icon}] {}: {}", result.recipe_path, result.reason);
        }
        if !result.valid {
            return Err("digest verification failed".to_string());
        }
    } else {
        let digest = digest_recipe(recipe_path)?;
        if json {
            let out =
                serde_json::to_string_pretty(&digest).map_err(|e| format!("JSON error: {e}"))?;
            println!("{out}");
        } else {
            println!("Recorded: {}", digest.recipe_path);
            println!("BLAKE3: {}", digest.blake3_hash);
        }
    }
    Ok(())
}
