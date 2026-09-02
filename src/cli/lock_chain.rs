//! Lock chain of custody — `forjar lock-verify-chain`.
//!
//! Split out of `lock_security.rs`: the chain check is the only lock command
//! that has to reason about what a signature PROVES rather than what it looks
//! like, and it carries the exit-code policy for absent evidence.

use super::helpers::*;
use std::path::Path;

// ── FJ-535: lock verify-chain ──

/// One machine's chain-of-custody verdict.
struct ChainVerdict {
    machine: String,
    valid: bool,
    detail: String,
}

impl ChainVerdict {
    fn new(machine: &str, valid: bool, detail: impl Into<String>) -> Self {
        Self {
            machine: machine.to_string(),
            valid,
            detail: detail.into(),
        }
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "machine": self.machine,
            "valid": self.valid,
            "detail": self.detail,
        })
    }
}

/// Machine directories holding a lock file **or** a signature.
///
/// Deliberately not [`discover_machines`], which lists only directories that
/// still contain a `state.lock.yaml`: a signature left behind by a deleted
/// lock is exactly the break in custody this command exists to catch, and that
/// filter hides it — the old code's "lock file missing" branch was unreachable.
fn chain_candidates(state_dir: &Path) -> Vec<String> {
    let mut machines = Vec::new();
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if path.join("state.lock.yaml").exists() || path.join("lock.sig").exists() {
                machines.push(entry.file_name().to_string_lossy().to_string());
            }
        }
    }
    machines.sort();
    machines
}

/// A short preview of an untrusted file's contents, cut on a CHARACTER
/// boundary — `&s[..20]` panics when byte 20 lands inside a multibyte char,
/// which a signature file is free to contain.
fn sig_preview(sig: &str) -> String {
    sig.chars().take(20).collect()
}

/// The bare digest of a `blake3:`-prefixed hash. `hasher::hash_string` emits
/// the prefix and `lock-sign` stores what it emits, so both sides of the
/// comparison must be normalised — comparing a stripped file against an
/// unstripped expectation never matches, and "never matches" is just the
/// always-fails twin of the always-passes bug.
fn bare_digest(hash: &str) -> &str {
    hash.strip_prefix("blake3:").unwrap_or(hash)
}

/// The hash a signature file carries, if it is a well-formed BLAKE3 digest.
fn well_formed_sig(sig: &str) -> Option<&str> {
    let hash = bare_digest(sig);
    let ok = hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit());
    ok.then_some(hash)
}

/// Verify one machine's link in the chain.
///
/// `key` is `None` in presence-only mode, where the verdict says so rather
/// than claiming a verification that did not happen.
fn chain_verdict(state_dir: &Path, m: &str, key: Option<&str>) -> ChainVerdict {
    use crate::tripwire::hasher;
    let lock_path = state_dir.join(m).join("state.lock.yaml");
    let sig_path = state_dir.join(m).join("lock.sig");

    if !lock_path.exists() {
        return ChainVerdict::new(m, false, "lock file missing — a signature signs nothing");
    }
    if !sig_path.exists() {
        return ChainVerdict::new(m, false, "signature file missing — lock was never signed");
    }
    let sig_raw = std::fs::read_to_string(&sig_path)
        .unwrap_or_default()
        .trim()
        .to_string();
    let Some(sig) = well_formed_sig(&sig_raw) else {
        return ChainVerdict::new(
            m,
            false,
            format!("malformed signature: {}", sig_preview(&sig_raw)),
        );
    };
    let Some(key) = key else {
        return ChainVerdict::new(
            m,
            true,
            "signature present and well-formed (presence only — chain NOT verified)",
        );
    };
    let content = match std::fs::read_to_string(&lock_path) {
        Ok(c) => c,
        Err(e) => return ChainVerdict::new(m, false, format!("lock file unreadable: {e}")),
    };
    let expected = hasher::hash_string(&format!("{content}{key}"));
    if sig == bare_digest(&expected) {
        ChainVerdict::new(m, true, "signature verified against lock content")
    } else {
        ChainVerdict::new(
            m,
            false,
            "signature does not match the lock — wrong key, or the lock changed after signing",
        )
    }
}

/// Which check `--key` / `--presence-only` selected.
///
/// A signature is `BLAKE3(lock content || key)`, so without the key NOTHING
/// ties a signature to the lock it claims to sign: 64 zeros is well-formed hex
/// and is a chain of custody for nobody. Refusing the bare invocation is the
/// point — an absent verifier is a NO-GO, never a pass.
fn chain_mode(key: Option<&str>, presence_only: bool) -> Result<Option<&str>, String> {
    match (key, presence_only) {
        (Some(_), true) => Err(
            "--presence-only cannot be combined with --key: presence-only does not verify \
             the signature against the lock, which is the whole point of passing a key"
                .to_string(),
        ),
        (Some(k), false) => Ok(Some(k)),
        (None, true) => Ok(None),
        (None, false) => Err(
            "chain of custody cannot be verified without the signing key: pass --key <KEY> \
             (the key lock-sign used), or --presence-only to check only that every lock \
             carries a well-formed signature"
                .to_string(),
        ),
    }
}

/// Emit a machine-readable failure so that `--json | jq` keeps working on the
/// paths that matter most — the ones where the gate is going red.
fn chain_json_failure(mode: &str, error: &str) {
    let out = serde_json::json!({
        "mode": mode,
        "machines": [],
        "all_valid": false,
        "error": error,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
}

fn print_chain_json(mode: &str, verdicts: &[ChainVerdict], all_valid: bool) {
    let out = serde_json::json!({
        "mode": mode,
        "machines": verdicts.iter().map(ChainVerdict::json).collect::<Vec<_>>(),
        "all_valid": all_valid,
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
}

fn print_chain_text(mode: &str, verdicts: &[ChainVerdict]) {
    println!("Lock chain verification ({mode}):\n");
    for v in verdicts {
        let icon = if v.valid { green("✓") } else { red("✗") };
        println!("  {icon} {} — {}", v.machine, v.detail);
    }
}

/// FJ-535: Lock verify chain — verify the chain of custody of every machine's
/// lock signature.
///
/// ## What a NONEXISTENT state directory means here
///
/// It is a FAILURE, and so is an existing directory holding no locks. A chain
/// verification that found no evidence has verified nothing; reporting success
/// on absent evidence turns every CI or release gate that calls this command
/// into a no-op, green precisely when the state it was meant to check is gone.
/// `lock-verify` and `lock-info` already exit 1 on a missing state dir — this
/// command now matches them, and deliberately breaks with `lock-validate`'s
/// "All 0 lock files are valid".
pub(crate) fn cmd_lock_verify_chain(
    state_dir: &Path,
    key: Option<&str>,
    presence_only: bool,
    json: bool,
) -> Result<(), String> {
    let key = chain_mode(key, presence_only)?;
    let mode = if key.is_some() {
        "verified"
    } else {
        "presence-only"
    };

    if let Err(e) = require_state_dir(state_dir) {
        if json {
            chain_json_failure(mode, &e);
        }
        return Err(e);
    }
    let machines = chain_candidates(state_dir);
    if machines.is_empty() {
        let e = format!(
            "no lock files found in {} — a chain over zero locks verifies nothing",
            state_dir.display()
        );
        if json {
            chain_json_failure(mode, &e);
        }
        return Err(e);
    }

    let verdicts: Vec<ChainVerdict> = machines
        .iter()
        .map(|m| chain_verdict(state_dir, m, key))
        .collect();
    let failed = verdicts.iter().filter(|v| !v.valid).count();

    if json {
        print_chain_json(mode, &verdicts, failed == 0);
    } else {
        print_chain_text(mode, &verdicts);
    }
    if failed == 0 {
        Ok(())
    } else {
        Err(format!(
            "chain of custody broken for {failed} of {} machine(s)",
            verdicts.len()
        ))
    }
}
