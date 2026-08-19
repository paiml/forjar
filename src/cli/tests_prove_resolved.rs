//! FJ-2733 (PMAT-200): `forjar prove` must prove the config that will RUN.
//!
//! # The defect
//!
//! `cmd_prove` parsed and validated, then handed RAW resources to every
//! checker. Templates were never expanded, so each invariant was evaluated
//! against text that differs from what `apply` executes.
//!
//! It produced a live FALSE PASS, not merely a cosmetic one. Two file
//! resources whose paths are spelled differently but resolve to the SAME file:
//!
//! ```yaml
//! params: { d1: /srv, d2: /srv }
//! a: { path: "{{params.d1}}/same.txt", content: A }
//! b: { path: "{{params.d2}}/same.txt", content: B }
//! ```
//!
//! Verified on the published 1.12.1 binary:
//!
//! ```text
//!   [PASS] I3 conflict-freedom: [CHECKED] 2 targets disjoint
//!   9/9 proofs passed
//! ```
//!
//! `[CHECKED]` is an explicit claim to have verified. The identical
//! infrastructure written literally is correctly `[FALSIFIED] target
//! collision`. forjar's provable-IaC story was proving something other than
//! what apply runs, and the failure direction was UNSAFE — it passed a real
//! same-file write conflict.

use std::path::Path;

fn write(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p
}

/// Two resources whose targets collide only after template resolution.
fn colliding_via_params(dir: &Path) -> std::path::PathBuf {
    write(
        dir,
        "templated.yaml",
        &format!(
            r#"
version: "1.0"
name: collide
machines:
  local:
    hostname: localhost
    addr: localhost
params:
  d1: "{d}"
  d2: "{d}"
resources:
  a:
    type: file
    machine: local
    path: "{{{{params.d1}}}}/same.txt"
    content: "A"
  b:
    type: file
    machine: local
    path: "{{{{params.d2}}}}/same.txt"
    content: "B"
"#,
            d = dir.display()
        ),
    )
}

/// The same infrastructure, spelled literally.
fn colliding_literally(dir: &Path) -> std::path::PathBuf {
    write(
        dir,
        "literal.yaml",
        &format!(
            r#"
version: "1.0"
name: collide
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  a:
    type: file
    machine: local
    path: "{d}/same.txt"
    content: "A"
  b:
    type: file
    machine: local
    path: "{d}/same.txt"
    content: "B"
"#,
            d = dir.display()
        ),
    )
}

fn prove(cfg: &Path) -> Result<(), String> {
    super::prove::cmd_prove(cfg, Path::new("state"), None, false)
}

#[test]
fn a_collision_hidden_behind_params_is_still_falsified() {
    // THE test. If prove reads raw resources this passes, and a genuine
    // same-file write conflict ships as PROVED.
    let d = tempfile::tempdir().unwrap();
    let cfg = colliding_via_params(d.path());
    assert!(
        prove(&cfg).is_err(),
        "two resources whose params resolve to the same path are in conflict; \
         prove must falsify it rather than report `[CHECKED] targets disjoint`"
    );
}

#[test]
fn the_literal_spelling_of_the_same_config_agrees() {
    // The control. Both spellings describe identical infrastructure, so both
    // verdicts must match — that equivalence is the whole point.
    let d = tempfile::tempdir().unwrap();
    assert!(prove(&colliding_literally(d.path())).is_err());
}

#[test]
fn a_templated_config_with_no_collision_still_proves() {
    // The guard against "fixed" meaning "always falsifies".
    let d = tempfile::tempdir().unwrap();
    let cfg = write(
        d.path(),
        "ok.yaml",
        &format!(
            r#"
version: "1.0"
name: fine
machines:
  local:
    hostname: localhost
    addr: localhost
params:
  d: "{dd}"
resources:
  a:
    type: file
    machine: local
    path: "{{{{params.d}}}}/one.txt"
    content: "A"
  b:
    type: file
    machine: local
    path: "{{{{params.d}}}}/two.txt"
    content: "B"
"#,
            dd = d.path().display()
        ),
    );
    assert!(
        prove(&cfg).is_ok(),
        "distinct resolved paths are not a conflict"
    );
}

// ── FJ-2733: `forjar lock` must write apply's hash universe ─────────────────

/// `lock` hashed the RAW resource; `executor::record_success` hashes the
/// RESOLVED one. For one templated file resource, measured on 1.12.1:
///
/// ```text
///   lock  wrote blake3:74dc260d…
///   apply wrote blake3:8dbba7db…
/// ```
///
/// Two hash universes for the same resource, so `lock --verify`, `lock-diff`
/// and `lock-integrity` compared unlike with unlike.
///
/// NOTE this is NOT what makes a freshly-locked config plan `1 to change` —
/// `lock` deliberately writes `status: unknown`, and the planner treats any
/// non-Converged status as needing work. That is honest: `lock` records which
/// resources exist, it does not claim they converged.
#[test]
fn lock_and_apply_hash_the_same_resource_identically() {
    use crate::core::planner::hash_desired_state;

    let d = tempfile::tempdir().unwrap();
    let cfg = write(
        d.path(),
        "forjar.yaml",
        &format!(
            r#"
version: "1.0"
name: lt
machines:
  local:
    hostname: localhost
    addr: localhost
params:
  d: "{dd}"
resources:
  f:
    type: file
    machine: local
    path: "{{{{params.d}}}}/out.txt"
    content: "hello"
"#,
            dd = d.path().display()
        ),
    );

    let config = super::helpers::parse_and_validate(&cfg).unwrap();
    let raw = &config.resources["f"];
    let resolved = crate::core::resolver::resolve_all(
        &config.resources,
        &config.params,
        &config.machines,
        &config.secrets,
    );

    assert_ne!(
        hash_desired_state(raw),
        hash_desired_state(&resolved["f"]),
        "the fixture must actually be templated, or this test proves nothing"
    );

    // What `lock` writes must be what `apply` writes: the resolved hash.
    let state_dir = d.path().join("state");
    super::lock_core::cmd_lock(&cfg, &state_dir, None, None, false, false, false).expect("lock");
    let lock = crate::core::state::load_lock(&state_dir, "local")
        .unwrap()
        .expect("lock written");

    assert_eq!(
        lock.resources["f"].hash,
        hash_desired_state(&resolved["f"]),
        "lock must write apply's hash universe, not the unresolved one"
    );
}
