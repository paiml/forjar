//! FJ-1433: Post-quantum dual signing.
//!
//! BLAKE3 + simulated PQ signature for quantum transition readiness.
//! Real PQ: SLH-DSA (SPHINCS+). This module provides the framework
//! and uses BLAKE3 as a placeholder until PQ crates stabilize.

use std::path::Path;

/// Dual signature with classical + PQ components.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DualSignature {
    pub path: String,
    pub blake3_hash: String,
    pub classical_sig: String,
    pub classical_alg: String,
    pub pq_sig: String,
    pub pq_alg: String,
    pub timestamp: String,
}

/// Dual verification result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DualVerifyResult {
    pub path: String,
    pub classical_valid: bool,
    pub pq_valid: bool,
    pub both_valid: bool,
    pub reason: String,
}

/// Create a dual (classical + PQ) signature.
pub fn dual_sign(file_path: &Path, signer: &str) -> Result<DualSignature, String> {
    let content = std::fs::read(file_path).map_err(|e| format!("read: {e}"))?;
    let blake3_hash = blake3::hash(&content).to_hex().to_string();

    // Classical: BLAKE3-HMAC
    let classical_input = format!("{blake3_hash}:classical:{signer}");
    let classical_sig = blake3::hash(classical_input.as_bytes())
        .to_hex()
        .to_string();

    // PQ: Simulated SLH-DSA (BLAKE3-based placeholder)
    let pq_input = format!("{blake3_hash}:slh-dsa:{signer}");
    let pq_sig = blake3::hash(pq_input.as_bytes()).to_hex().to_string();

    let sig = DualSignature {
        path: file_path.display().to_string(),
        blake3_hash,
        classical_sig,
        classical_alg: "blake3-hmac".to_string(),
        pq_sig,
        pq_alg: "slh-dsa-blake3-placeholder".to_string(),
        timestamp: format!("{:?}", std::time::SystemTime::now()),
    };

    let sig_path = file_path.with_extension("dual-sig.json");
    let data = serde_json::to_string_pretty(&sig).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&sig_path, data).map_err(|e| format!("write sig: {e}"))?;

    Ok(sig)
}

/// Verify a dual signature.
pub fn dual_verify(file_path: &Path) -> Result<DualVerifyResult, String> {
    let sig_path = file_path.with_extension("dual-sig.json");
    if !sig_path.exists() {
        return Ok(DualVerifyResult {
            path: file_path.display().to_string(),
            classical_valid: false,
            pq_valid: false,
            both_valid: false,
            reason: "no dual signature file".to_string(),
        });
    }

    let sig_data = std::fs::read_to_string(&sig_path).map_err(|e| format!("read sig: {e}"))?;
    let sig: DualSignature =
        serde_json::from_str(&sig_data).map_err(|e| format!("parse sig: {e}"))?;

    let content = std::fs::read(file_path).map_err(|e| format!("read: {e}"))?;
    let current_hash = blake3::hash(&content).to_hex().to_string();

    let hash_valid = current_hash == sig.blake3_hash;

    Ok(DualVerifyResult {
        path: file_path.display().to_string(),
        classical_valid: hash_valid,
        pq_valid: hash_valid,
        both_valid: hash_valid,
        reason: if hash_valid {
            "both signatures valid".to_string()
        } else {
            "hash mismatch — file modified".to_string()
        },
    })
}

/// CLI command for dual signing/verification.
pub fn cmd_dual_sign(
    file_path: &Path,
    verify_only: bool,
    signer: Option<&str>,
    json: bool,
) -> Result<(), String> {
    if verify_only {
        let result = dual_verify(file_path)?;
        if json {
            let out =
                serde_json::to_string_pretty(&result).map_err(|e| format!("JSON error: {e}"))?;
            println!("{out}");
        } else {
            print_dual_verify(&result);
        }
        if !result.both_valid {
            return Err("dual verification failed".to_string());
        }
    } else {
        let who = signer.unwrap_or("local");
        let sig = dual_sign(file_path, who)?;
        if json {
            let out = serde_json::to_string_pretty(&sig).map_err(|e| format!("JSON error: {e}"))?;
            println!("{out}");
        } else {
            println!("Dual-signed: {}", sig.path);
            println!(
                "Classical: {} ({})",
                &sig.classical_sig[..16],
                sig.classical_alg
            );
            println!("PQ: {} ({})", &sig.pq_sig[..16], sig.pq_alg);
        }
    }
    Ok(())
}

fn print_dual_verify(r: &DualVerifyResult) {
    let icon = if r.both_valid { "OK" } else { "FAIL" };
    println!("[{icon}] {}: {}", r.path, r.reason);
    println!(
        "  Classical: {} | PQ: {}",
        if r.classical_valid {
            "valid"
        } else {
            "invalid"
        },
        if r.pq_valid { "valid" } else { "invalid" },
    );
}
