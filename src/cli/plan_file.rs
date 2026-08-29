//! FJ-1250: the saved plan file — `forjar plan --out` writes it,
//! `forjar apply --plan-file` reads it back.
//!
//! Extracted from `plan.rs`, which was at 514 lines against a 500-line gate.
//!
//! # Refs #356 / #358 — v1 verified the config and nothing else
//!
//! A `forjar-plan-v1` document carried a `config_hash` and, underneath it,
//! `changes` / `execution_order` / three counters as plain unauthenticated
//! JSON. `apply --plan-file` checked the hash and then read the body as if it
//! were trustworthy, so editing three integers — leaving `config_hash`
//! byte-identical — made a requested apply print "Plan has no changes to
//! apply." and exit 0 having converged nothing.
//!
//! `forjar-plan-v2` adds a `seal` object binding the config, the state locks
//! the planner READ, and the body itself. See `core::plan_seal` for what that
//! does and does not prove.
//!
//! It also carries a `selectors` object — the `-m`/`-r`/`-t`/`-g` the plan was
//! produced under — sealed with the body. `apply --plan-file` re-plans under it
//! to check what the document claims, and without it a legitimate `plan -r X
//! --out` over a converged X is byte-identical to an honest whole-stack plan
//! with the pending lines deleted out of it.
//!
//! v1 documents still load, with a warning: their config check is real, and
//! refusing them outright would strand plans written by an installed binary.
//! What a v1 document may NOT do is carry a v2 key. It has no seal, so nothing
//! in it is authenticated beyond its config hash — and `load_plan_file` was
//! reading `selectors` out of one, BEFORE the `sealed` branch, and re-planning
//! under it. That handed the forger the filters their forgery was checked
//! against; see [`reject_v2_keys_on_v1`] for the measurement. Both `seal` and
//! `selectors` are now refused on a v1 document, so a v1 plan is always checked
//! against the whole config.
//!
//! forjar never downgrades a document on its own: a v2 document whose seal does
//! not verify is an error, never a fallback to v1 checking. An EDITOR still
//! can, by relabelling `format` and deleting the `seal` — the format tag is the
//! one field no check can cover, because it selects the checks. Measured
//! against the branch binary, downgrading both kinds of plan:
//!
//! ```text
//!   plan -r alpha --out, relabelled v1, seal+selectors deleted
//!     → error: PLAN_STALE … finds 1 change(s) this plan file does not name:
//!               bravo on box (CREATE)                                EXIT=1
//!   plan --out (whole stack), relabelled the same way
//!     → Plan applied: 2 converged, 0 unchanged, 0 failed             EXIT=0
//! ```
//!
//! So the downgrade drops the state and expiry legs and buys the editor
//! nothing: the second document is complete and true, and the first is refused
//! by the unfiltered re-plan. What the dropped state leg costs is narrow and
//! worth stating — a v2 plan is refused the moment a lock moves, a downgraded
//! one only when the move changes the planner's answer for a pair the body
//! names or omits.
//!
//! # Every other field a v1 document offers (Refs #358)
//!
//! Same question of each: is it doing work, and does anything authenticate it?
//!
//! | field | what reads it | held true by |
//! |---|---|---|
//! | `format` | picks the branch above | nothing — it selects the checks |
//! | `config_hash` | [`check_config_hash`] | compared to the LIVE config's hash |
//! | `changes[].resource_id`/`.machine` | the executed scope | the re-plan, both directions |
//! | `changes[].action` | the scope, the preview | the re-plan |
//! | `changes[].resource_type`/`.description` | the `--dry-run` preview | the re-plan, as of this round |
//! | the four counters | `check_body_partition`, `-v` output | must partition `changes` |
//! | `execution_order` | nothing — `executor::apply_scoped` rebuilds it from the config | n/a |
//! | `name`, `config_file` | nothing on this path | n/a |
//!
//! The two `changes` fields in that third row are the other half of this
//! round's fix: `check_plan_still_holds` compared `action` alone, so a v1
//! document could describe an honest change as anything it liked and
//! `--plan-file --dry-run` would print it. See
//! `cli::apply_from_plan_checks::divergence`.

use crate::core::plan_seal::{self, PlanSeal};
use crate::core::plan_selectors::PlanSelectors;
use crate::core::types;
use std::path::Path;

/// The original, config-hash-only plan document.
pub(crate) const FORMAT_V1: &str = "forjar-plan-v1";
/// The sealed plan document.
pub(crate) const FORMAT_V2: &str = "forjar-plan-v2";

/// A plan file that passed its integrity checks.
#[derive(Debug)]
pub(crate) struct LoadedPlan {
    /// The plan body, as it will be executed.
    pub plan: types::ExecutionPlan,
    /// True when the body itself was sealed (v2), false for a v1 document
    /// where only the config was verified.
    ///
    /// The caller needs this: "this plan has no changes" is a legitimate
    /// instruction from a sealed plan and an unauthenticated one from a v1
    /// document, and acting on the second is how a requested apply exits 0
    /// having done nothing.
    pub sealed: bool,
    /// Refs #358: the filters this plan was produced under.
    ///
    /// The caller re-plans under exactly these to decide whether the body is
    /// still true. Only a v2 document can carry them, because only a v2
    /// document seals them: a v1 plan always reads back as
    /// [`PlanSelectors::default`] — the whole config, the strictest reading
    /// available — and one that CLAIMS to be narrow is refused rather than
    /// believed.
    pub selectors: PlanSelectors,
}

/// FJ-1250: Save an execution plan to a JSON file, sealed against the config,
/// the state it was planned from, and its own body.
///
/// The seal carries no wall-clock expiry (`ttl_secs: 0`). A plan file routinely
/// crosses CI stages, and forjar has no trusted clock — the state leg already
/// invalidates a plan the moment the world it reasoned about moves, which is a
/// far better staleness signal than an age in seconds.
pub(crate) fn save_plan_file(
    plan: &types::ExecutionPlan,
    selectors: &PlanSelectors,
    config: &types::ForjarConfig,
    config_path: &Path,
    state_dir: &Path,
    out_path: &Path,
) -> Result<(), String> {
    let sealed = plan_seal::seal(plan, selectors, config, state_dir, None)?;

    let changes: Vec<serde_json::Value> = plan
        .changes
        .iter()
        .map(|c| {
            serde_json::json!({
                "resource_id": c.resource_id,
                "machine": c.machine,
                "resource_type": c.resource_type,
                "action": c.action,
                "description": c.description,
            })
        })
        .collect();

    let output = serde_json::json!({
        "format": FORMAT_V2,
        "config_file": config_path.display().to_string(),
        // GH-212: canonical (sorted-map) hash — the plain serialisation varied
        // per process, so `apply --plan-file` rejected plans nobody had
        // touched. Kept at its v1 key and value; the seal carries the same
        // string and the two are checked against each other on load.
        "config_hash": sealed.config_hash,
        "name": plan.name,
        "to_create": plan.to_create,
        "to_update": plan.to_update,
        "to_destroy": plan.to_destroy,
        "unchanged": plan.unchanged,
        "execution_order": plan.execution_order,
        "changes": changes,
        // Refs #358: what this plan was filtered by, so `apply --plan-file` can
        // recompute the plan it claims to be rather than guessing.
        "selectors": selectors,
        "seal": sealed,
    });

    let json = serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {e}"))?;
    std::fs::write(out_path, json).map_err(|e| format!("write plan file: {e}"))?;
    Ok(())
}

/// A plan-file string field, or `default` when absent or not a string.
fn plan_str<'a>(entry: &'a serde_json::Value, key: &str, default: &'a str) -> &'a str {
    entry.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}

/// A plan-file unsigned field, or 0 when absent or not a number.
fn plan_u32(doc: &serde_json::Value, key: &str) -> u32 {
    doc.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as u32
}

/// A plan-file array-of-strings field, or empty when absent.
fn plan_str_array(doc: &serde_json::Value, key: &str) -> Vec<String> {
    doc.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Read an enum field back through the SAME serde impl that wrote it.
///
/// # Refs #358 — a hand-written table is a second spelling of the schema
///
/// This replaces two `match`es over string literals. The `ResourceType` one
/// named 12 of that enum's 21 variants and mapped the other 9 to `File`, and
/// that was not a theoretical gap: [`plan_body_from_doc`] feeds the
/// RECONSTRUCTED body to the diff leg, so an honest, untouched, freshly written
/// v2 plan of a config holding a `task` resource could not be applied at all.
/// Measured on the branch binary, with nobody having touched the file:
///
/// ```text
///   $ forjar plan -f forjar.yaml --state-dir state --out p.json
///   Plan saved to p.json
///   $ forjar apply -f forjar.yaml --state-dir state --plan-file p.json --yes
///   error: PLAN_HASH_MISMATCH: the plan body was modified after it was sealed
///          (diff leg: expected blake3:9b7c5d06…, got blake3:dbad3d20…)
/// ```
///
/// `task`, `wasm_bundle`, `image`, `build`, `github_release`,
/// `overlay_interface`, `disk_budget`, `backup_sync` and `nas_archive` were all
/// unreachable this way. Going through `serde` means the reader cannot drift
/// from the writer again when a variant is added.
///
/// A value this build cannot read is an error rather than a default: a change
/// whose action or type forjar does not understand is not one to guess at.
fn plan_enum<T: serde::de::DeserializeOwned>(
    entry: &serde_json::Value,
    key: &str,
) -> Result<T, String> {
    let raw = entry
        .get(key)
        .ok_or_else(|| format!("PLAN_MALFORMED: a change in this plan file has no '{key}'"))?;
    serde_json::from_value(raw.clone())
        .map_err(|e| format!("PLAN_MALFORMED: unreadable '{key}' in a plan change: {e}"))
}

fn planned_change_from_entry(entry: &serde_json::Value) -> Result<types::PlannedChange, String> {
    Ok(types::PlannedChange {
        resource_id: plan_str(entry, "resource_id", "").to_string(),
        machine: plan_str(entry, "machine", "").to_string(),
        resource_type: plan_enum(entry, "resource_type")?,
        action: plan_enum(entry, "action")?,
        description: plan_str(entry, "description", "").to_string(),
    })
}

/// Reconstruct the plan body from the document.
///
/// The RECONSTRUCTED body is what the diff leg is computed over and what gets
/// executed, so the hash covers exactly the value the executor will act on. The
/// reader therefore normalises nothing any more: every field either round-trips
/// to what the writer held or is refused (see [`plan_enum`]), because a field
/// the reader quietly rewrites makes an honest document fail its own seal.
fn plan_body_from_doc(doc: &serde_json::Value) -> Result<types::ExecutionPlan, String> {
    let changes_arr = doc
        .get("changes")
        .and_then(|v| v.as_array())
        .ok_or("plan file missing 'changes' array")?;
    let changes = changes_arr
        .iter()
        .map(planned_change_from_entry)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(types::ExecutionPlan {
        name: plan_str(doc, "name", "").to_string(),
        changes,
        execution_order: plan_str_array(doc, "execution_order"),
        to_create: plan_u32(doc, "to_create"),
        to_update: plan_u32(doc, "to_update"),
        to_destroy: plan_u32(doc, "to_destroy"),
        unchanged: plan_u32(doc, "unchanged"),
    })
}

/// Read the selector record back. Sealed documents only — a v1 document that
/// carries the key never reaches here (see [`reject_v2_keys_on_v1`]).
///
/// An ABSENT `selectors` key reads as the unfiltered record: a document that
/// does not claim to be narrow is held to the whole config. That is not a
/// silent default — the record is inside the diff leg, so a v2 document that
/// omits it verifies only if it was sealed unfiltered.
///
/// A `selectors` value that is not a valid record is an error rather than a
/// fallback to unfiltered: falling back would run the comparison under filters
/// the document did not ask for.
fn selectors_from_doc(doc: &serde_json::Value) -> Result<PlanSelectors, String> {
    let Some(raw) = doc.get("selectors") else {
        return Ok(PlanSelectors::default());
    };
    serde_json::from_value(raw.clone())
        .map_err(|e| format!("PLAN_MALFORMED: unreadable plan selectors: {e}"))
}

/// The keys that arrived WITH the seal, and that a `forjar-plan-v1` document
/// therefore cannot honestly carry.
const V2_ONLY_KEYS: [&str; 2] = ["seal", "selectors"];

/// Refs #358: a v1 document carrying a v2-only key is a downgrade, not a plan.
///
/// `selectors` is the one that mattered. [`load_plan_file`] called
/// [`selectors_from_doc`] unconditionally, BEFORE the `sealed` branch, so a
/// `forjar-plan-v1` document — which has no seal, and whose every field is
/// therefore authenticated by nothing — could name the filters that
/// `check_plan_still_holds` re-planned under, and the re-plan then agreed with
/// the forgery. Measured against the branch binary, on a stack where `alpha`
/// and `bravo` were both pending create: a hand-rolled v1 document naming only
/// `alpha`, with a `config_hash` copied from an honest plan and four lines of
/// JSON added —
///
/// ```text
///   "selectors": {"machine": null, "resource": "alpha", "tag": null, "group": null}
///
///   $ forjar apply -f forjar.yaml --plan-file v1_narrow.json --yes
///   warning: 'forjar-plan-v1' plan file — only the config is verified. …
///   Plan applied: 1 converged, 1 unchanged, 0 failed          EXIT=0
/// ```
///
/// — converged `alpha`, left `bravo`'s create pending, and said nothing about
/// it; `forjar plan` still showed it afterwards. The SAME document with those
/// four lines deleted was refused `PLAN_STALE`, so the key was the entire
/// bypass.
///
/// Refused rather than ignored, because the key cannot have arrived honestly.
/// The v1 writer shipped in 1.21.0 emitted exactly ten keys — `format`,
/// `config_file`, `config_hash`, `name`, the four counters, `execution_order`
/// and `changes` (`git show aba58fb0:src/cli/plan.rs`) — and both `seal` and
/// `selectors` were added by this branch, alongside v2. No forjar has ever
/// written either into a v1 document, so a document carrying one was
/// hand-written, and saying that out loud is more use to an operator than
/// quietly reading the file on different terms than it asks for.
fn reject_v2_keys_on_v1(doc: &serde_json::Value) -> Result<(), String> {
    let carried: Vec<&str> = V2_ONLY_KEYS
        .into_iter()
        .filter(|key| doc.get(key).is_some())
        .collect();
    if carried.is_empty() {
        return Ok(());
    }
    Err(format!(
        "PLAN_MALFORMED: this '{FORMAT_V1}' plan file carries '{}', which only a \
         '{FORMAT_V2}' document has. A v1 document is unsealed, so nothing authenticates \
         that key, and honouring it would let the file choose the terms it is checked \
         against — a v1 plan claiming to be narrow would be re-planned under its own \
         filters and agreed with. Re-run `forjar plan --out` to write a sealed \
         '{FORMAT_V2}' plan.",
        carried.join("', '")
    ))
}

/// Reject a plan whose config hash no longer matches the config being applied.
fn check_config_hash(doc: &serde_json::Value, config: &types::ForjarConfig) -> Result<(), String> {
    let stored_hash = plan_str(doc, "config_hash", "");
    let current_hash = crate::core::config_hash::config_hash(config)?;
    if stored_hash != current_hash {
        return Err(
            "config has changed since plan was created — re-run `forjar plan` to regenerate"
                .to_string(),
        );
    }
    Ok(())
}

/// v1: the config is verified; the body is not. Load it, say so, and still
/// refuse a body whose counters contradict its own change list.
fn check_v1(
    doc: &serde_json::Value,
    plan: &types::ExecutionPlan,
    config: &types::ForjarConfig,
) -> Result<(), String> {
    eprintln!(
        "warning: '{FORMAT_V1}' plan file — only the config is verified. The state it was \
         planned against and the plan body itself are unsealed. Re-run `forjar plan --out` \
         to write a sealed '{FORMAT_V2}' plan."
    );
    reject_v2_keys_on_v1(doc)?;
    check_config_hash(doc, config)?;
    plan_seal::check_body_partition(plan).map_err(|e| e.to_string())
}

/// v2: recompute all three legs from live inputs and compare with the seal.
fn check_v2(
    doc: &serde_json::Value,
    plan: &types::ExecutionPlan,
    selectors: &PlanSelectors,
    config: &types::ForjarConfig,
    state_dir: &Path,
) -> Result<(), String> {
    let raw = doc
        .get("seal")
        .ok_or_else(|| format!("PLAN_MALFORMED: '{FORMAT_V2}' plan file has no 'seal'"))?;
    let sealed: PlanSeal = serde_json::from_value(raw.clone())
        .map_err(|e| format!("PLAN_MALFORMED: unreadable plan seal: {e}"))?;
    if plan_str(doc, "config_hash", "") != sealed.config_hash {
        return Err(
            "PLAN_MALFORMED: the plan's config_hash disagrees with its own seal".to_string(),
        );
    }
    plan_seal::verify(&sealed, plan, selectors, config, state_dir).map_err(|e| e.to_string())
}

/// FJ-1250: Load a saved plan file and verify it against the live world.
///
/// The format tag is checked BEFORE the body is read, so an unrecognised
/// document is reported as an unsupported format rather than as a missing
/// field it was never going to have.
pub(crate) fn load_plan_file(
    plan_path: &Path,
    config: &types::ForjarConfig,
    state_dir: &Path,
) -> Result<LoadedPlan, String> {
    let content = std::fs::read_to_string(plan_path).map_err(|e| format!("read plan file: {e}"))?;
    let doc: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("parse plan file: {e}"))?;

    let format = plan_str(&doc, "format", "").to_string();
    let sealed = match format.as_str() {
        FORMAT_V2 => true,
        FORMAT_V1 => false,
        other => return Err(format!("unsupported plan format: '{other}'")),
    };

    let plan = plan_body_from_doc(&doc)?;
    // Refs #358: the selector record is read INSIDE the sealed branch. Reading
    // it before this point gave an unsealed v1 document a say in the filters it
    // was re-planned under, which is the bypass `reject_v2_keys_on_v1`
    // documents. A v1 plan is checked against the whole config, unfiltered.
    let selectors = if sealed {
        let selectors = selectors_from_doc(&doc)?;
        check_v2(&doc, &plan, &selectors, config, state_dir)?;
        selectors
    } else {
        check_v1(&doc, &plan, config)?;
        PlanSelectors::default()
    };
    Ok(LoadedPlan {
        plan,
        sealed,
        selectors,
    })
}
