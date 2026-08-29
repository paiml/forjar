//! The three legs of a plan seal, and the composition that binds them.
//!
//! Each leg answers one question about the world the planner saw:
//!
//! | leg    | binds the plan to …                                        |
//! |--------|------------------------------------------------------------|
//! | config | the `forjar.yaml` it was planned from                       |
//! | state  | the lock files it READ to decide create vs update vs no-op  |
//! | diff   | its own body — the changes, the counters, and the selectors |
//!
//! Only the config leg shipped before (`core::config_hash`, GH-212). A plan
//! whose config hash still matched could nonetheless be stale (the lock moved
//! under it) or edited (the body is plain JSON), and both were accepted.

use crate::core::config_hash;
use crate::core::plan_selectors::PlanSelectors;
use crate::core::state;
use crate::core::types::{ExecutionPlan, ForjarConfig};
use std::path::Path;

/// Version-tagged domain separator.
///
/// Mixed into every leg and into the composition, so a hash computed here can
/// never collide with a hash of the same bytes computed for another purpose,
/// and so a future schema change invalidates old seals instead of colliding
/// with them.
pub const SEAL_DOMAIN: &str = "forjar-plan-seal-v1";

/// Framing tag for a machine whose lock file is present.
const LOCK_PRESENT: &[u8] = b"\x01";
/// Framing tag for a machine that has no lock file yet.
///
/// Distinct from a zero-length lock: "never applied" and "applied, empty" are
/// different worlds and must not hash the same.
const LOCK_ABSENT: &[u8] = b"\x00";

/// Leg 1 — the config the plan was built from.
///
/// Delegates to the shipped canonical hash rather than re-deriving one: a
/// second expression for "the hash of this config" is exactly what GH-212 was.
pub fn config_leg(config: &ForjarConfig) -> Result<String, String> {
    config_hash::config_hash(config)
}

/// Leg 2 — the state the planner READ.
///
/// Folds the raw bytes of every declared machine's lock file, in sorted machine
/// order, with an explicit present/absent tag and a length prefix so no two
/// different state directories can frame to the same byte stream.
///
/// Hashes the LOCK, not its `.b3` sidecar: a sidecar can be re-written by
/// `forjar reseal`, so sealing the sidecar would let a resealed tamper through.
pub fn state_leg(config: &ForjarConfig, state_dir: &Path) -> Result<String, String> {
    let mut names: Vec<&str> = config.machines.keys().map(String::as_str).collect();
    names.sort_unstable();

    let mut hasher = blake3::Hasher::new();
    hasher.update(SEAL_DOMAIN.as_bytes());
    hasher.update(b"\0state\0");
    for name in names {
        hasher.update(name.as_bytes());
        hasher.update(b"\0");
        absorb_lock(&mut hasher, &state::lock_file_path(state_dir, name))?;
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

/// Fold one machine's lock file into the state leg.
///
/// A missing file is the `LOCK_ABSENT` sentinel — that is a legitimate state,
/// not an error. Any OTHER read failure IS an error: a lock forjar cannot read
/// is a lock it cannot vouch for, and hashing "unreadable" as "absent" would
/// make a permissions change look like a fresh machine.
fn absorb_lock(hasher: &mut blake3::Hasher, path: &Path) -> Result<(), String> {
    match std::fs::read(path) {
        Ok(bytes) => {
            hasher.update(LOCK_PRESENT);
            hasher.update(&(bytes.len() as u64).to_le_bytes());
            hasher.update(&bytes);
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            hasher.update(LOCK_ABSENT);
            Ok(())
        }
        Err(e) => Err(format!("cannot read {}: {e}", path.display())),
    }
}

/// Leg 3 — the plan body itself, and the selectors it was produced under.
///
/// `ExecutionPlan`/`PlannedChange`/`PlanSelectors` hold only `String`, `Vec`,
/// `Option<String>` and `u32` — no maps — so `serde_json` emits struct fields
/// in declaration order and is already canonical. Nothing extra is needed to
/// make this reproducible.
///
/// # Refs #358 — why the selectors are in HERE rather than in a fourth leg
///
/// They answer the question this leg already asks: *was this document's body
/// edited?* `PlanSelectors` is not an input the planner read from the world —
/// it is part of what the document ASSERTS about itself, exactly like the
/// counters, and `apply --plan-file` re-plans under it. A fourth leg would need
/// its own [`super::Leg`] variant, its own remedy sentence and its own slot in
/// [`compose`] to say the same thing the `diff` leg's remedy already says.
///
/// The two are framed apart inside the hash, so a change list that happens to
/// serialise to the same bytes as a selector record cannot be swapped for one.
pub fn diff_leg(plan: &ExecutionPlan, selectors: &PlanSelectors) -> Result<String, String> {
    let body = serde_json::to_vec(plan).map_err(|e| format!("serialize plan: {e}"))?;
    let sel = serde_json::to_vec(selectors).map_err(|e| format!("serialize selectors: {e}"))?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(SEAL_DOMAIN.as_bytes());
    hasher.update(b"\0diff\0");
    hasher.update(&body);
    hasher.update(b"\0selectors\0");
    hasher.update(&sel);
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

/// Bind the three legs and the validity window into one value.
///
/// NUL-delimited so concatenation is unambiguous: `("ab", "c")` and
/// `("a", "bc")` must not compose to the same seal.
///
/// `sealed_at` and `ttl` are INSIDE the composition, not beside it. Moving the
/// expiry of a sealed plan is therefore a hash mismatch, not a longer life.
pub fn compose(config: &str, state: &str, diff: &str, sealed_at: u64, ttl: u64) -> String {
    let sealed_at = sealed_at.to_string();
    let ttl = ttl.to_string();
    let mut hasher = blake3::Hasher::new();
    for part in [SEAL_DOMAIN, config, state, diff, &sealed_at, &ttl] {
        hasher.update(part.as_bytes());
        hasher.update(b"\0");
    }
    format!("blake3:{}", hasher.finalize().to_hex())
}

/// A short, content-derived handle for a sealed plan.
///
/// The first 16 bytes of the seal, hex-encoded. Content-derived and not random,
/// so two seals of the same inputs at the same instant carry the same id and a
/// test never has to special-case an RNG.
pub fn plan_id(seal: &str) -> String {
    seal.strip_prefix("blake3:")
        .unwrap_or(seal)
        .chars()
        .take(32)
        .collect()
}
