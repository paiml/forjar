//! GH-251: every contract must actually be checkable.
//!
//! Six of them were not. `pv validate` failed on
//! `apply-summary-distinguishability-v1`, `destroy-undo-roundtrip-v1`,
//! `idempotent-apply-v1`, `plan-apply-equivalence-v1` and `provable-iac-v1`
//! because their `proof_obligations[].type` used `test` and `honesty`, which
//! are not in the schema's vocabulary. Those files had therefore **never**
//! passed validation, and nothing noticed, because nothing ran it.
//!
//! That matters beyond tidiness. `apply-summary-distinguishability-v1` is the
//! planned-vs-actual contract cited as the correct precedent in GH-249, and
//! `provable-iac-v1` backs `forjar prove`'s structural invariants — its results
//! are mapped straight into the `N/N proofs passed` line a user acts on. Both
//! read as authority while being checked by nothing.
//!
//! This is the same failure shape as GH-242 one level down: proofs cited by
//! name as evidence that no CI job ran, and which had stopped compiling.
//!
//! Implemented against the YAML directly rather than by shelling out to `pv`,
//! deliberately. A test that needs an external tool installed is a test that
//! silently stops running when the tool is missing — which is precisely how the
//! contracts went unvalidated in the first place, and how the kani/lean gate in
//! GH-242 currently reports red.
//!
//! **That choice has a cost, and it was paid.** Re-running `pv validate` over
//! `contracts/` on 2026-08-17 found the SAME five files still rejected — for
//! different reasons than GH-251 (a flat `enforcement` block where the schema
//! wants named rule structs; `bound: "4 keys"` in a u32 field;
//! `strategy: bounded` against a vocabulary of four; and kernel-by-default
//! contracts with nothing bounded to prove). Every test here passed throughout,
//! because each asserted a hand-copied *fragment* of the schema and the files
//! got those fragments right.
//!
//! A proxy only covers what you thought to copy. So the rule for this file:
//! when `pv` rejects a contract, do not just fix the contract — add the check
//! that should have caught it here, and confirm it fails on the old file.

use std::path::{Path, PathBuf};

/// The proof-obligation vocabulary the contract schema accepts.
///
/// Copied from the validator's own error message rather than invented. If the
/// schema gains a variant this list must grow with it — and until then, a
/// contract using an unknown one does not validate, which is the whole point.
const KNOWN_OBLIGATION_TYPES: &[&str] = &[
    "invariant",
    "equivalence",
    "bound",
    "monotonicity",
    "idempotency",
    "linearity",
    "symmetry",
    "associativity",
    "conservation",
    "ordering",
    "completeness",
    "soundness",
    "involution",
    "determinism",
    "roundtrip",
    "state_machine",
    "classification",
    "independence",
    "termination",
    "safety",
    "liveness",
    "precondition",
    "postcondition",
    "frame",
    "loop_invariant",
    "loop_variant",
    "old_state",
    "subcontract",
];

/// Files under `contracts/` that are deliberately NOT contracts.
///
/// `binding.yaml` is a binding REGISTRY — it maps contract equations to the
/// Rust items implementing them (`contract_coverage.rs` reads it). Validating
/// it as a contract is a category error, not a defect in the file, and its
/// "missing field `metadata`" failure is that category error reporting itself.
const NOT_CONTRACTS: &[&str] = &["binding.yaml"];

fn contracts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("contracts")
}

fn contract_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(contracts_dir())
        .expect("contracts/ must exist")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .filter(|p| {
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            !NOT_CONTRACTS.contains(&name.as_str())
        })
        .collect();
    files.sort();
    files
}

#[test]
fn there_are_contracts_to_check() {
    // Guards the guard. A glob that silently matches nothing would make every
    // test below pass by checking zero files — the vacuous-green shape that
    // `provable-iac-v1` itself has an obligation about.
    let files = contract_files();
    assert!(
        files.len() >= 20,
        "expected the full contract set, found {}: {:?}",
        files.len(),
        files
    );
}

#[test]
fn every_contract_parses_as_yaml() {
    for path in contract_files() {
        let text = std::fs::read_to_string(&path).expect("readable");
        let parsed: Result<serde_yaml_ng::Value, _> = serde_yaml_ng::from_str(&text);
        assert!(
            parsed.is_ok(),
            "{} is not parseable YAML: {}",
            path.display(),
            parsed.unwrap_err()
        );
    }
}

#[test]
fn every_proof_obligation_uses_a_known_type() {
    // The GH-251 regression itself. `type: test` and `type: honesty` are not in
    // the schema, so five contracts failed to parse and had never been checked.
    let mut offenders = Vec::new();

    for path in contract_files() {
        let text = std::fs::read_to_string(&path).expect("readable");
        let doc: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&text) {
            Ok(d) => d,
            Err(_) => continue, // reported by the parse test above
        };
        let Some(obligations) = doc.get("proof_obligations").and_then(|v| v.as_sequence()) else {
            continue;
        };
        for (i, ob) in obligations.iter().enumerate() {
            let Some(ty) = ob.get("type").and_then(|v| v.as_str()) else {
                offenders.push(format!("{}: obligation[{i}] has no `type`", path.display()));
                continue;
            };
            if !KNOWN_OBLIGATION_TYPES.contains(&ty) {
                offenders.push(format!(
                    "{}: obligation[{i}] type `{ty}` is not in the schema vocabulary",
                    path.display()
                ));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "contracts using unknown obligation types are never validated by anything:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn every_contract_declares_metadata() {
    // The other half of what `pv validate` enforces, and the reason
    // binding.yaml is excluded rather than "fixed": a contract without
    // `metadata` is not a contract.
    for path in contract_files() {
        let text = std::fs::read_to_string(&path).expect("readable");
        let doc: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&text) {
            Ok(d) => d,
            Err(_) => continue,
        };
        assert!(
            doc.get("metadata").is_some(),
            "{} has no `metadata` block; if it is not a contract it belongs in \
             NOT_CONTRACTS with a reason, not in the contract set",
            path.display()
        );
    }
}

/// Kani strategies the schema accepts, copied from its own error message.
const KNOWN_KANI_STRATEGIES: &[&str] =
    &["exhaustive", "stub_float", "compositional", "bounded_int"];

#[test]
fn every_enforcement_entry_is_a_rule_not_a_scalar() {
    // `enforcement` is a MAP OF NAMED RULES, each a struct. Four contracts
    // instead used a flat `layer:/failure_mode:/ci_gate:/notes:` block, so the
    // schema read `layer` as a rule name whose value should have been a struct
    // and rejected the file outright.
    //
    // This test exists because the checks above did NOT catch that: they
    // validate obligation types and `metadata`, both of which those files got
    // right. They passed while `pv validate` rejected all four — a green test
    // asserting a *copy* of part of the schema, which is the same
    // proxy-instead-of-artifact shape this whole file was written about.
    let mut offenders = Vec::new();
    for path in contract_files() {
        let text = std::fs::read_to_string(&path).expect("readable");
        let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else {
            continue;
        };
        let Some(rules) = doc.get("enforcement").and_then(|v| v.as_mapping()) else {
            continue;
        };
        for (name, rule) in rules {
            if !rule.is_mapping() {
                offenders.push(format!(
                    "{}: enforcement.{} is a scalar; it must be a rule with \
                     description/check/severity",
                    path.display(),
                    name.as_str().unwrap_or("?")
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "malformed enforcement blocks never validate:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn every_kani_harness_declares_a_numeric_bound_and_known_strategy() {
    // `bound: "4 keys"` (a unit smuggled into a u32 field) and
    // `strategy: bounded` (against a 12-to-1 majority using `bounded_int`)
    // each rejected a whole contract — including provable-iac-v1, whose
    // results are mapped straight into the `N/N proofs passed` line.
    let mut offenders = Vec::new();
    for path in contract_files() {
        let text = std::fs::read_to_string(&path).expect("readable");
        let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else {
            continue;
        };
        let Some(harnesses) = doc.get("kani_harnesses").and_then(|v| v.as_sequence()) else {
            continue;
        };
        for (i, h) in harnesses.iter().enumerate() {
            if let Some(bound) = h.get("bound") {
                if !bound.is_u64() {
                    offenders.push(format!(
                        "{}: kani_harnesses[{i}].bound is not an integer: {bound:?}",
                        path.display()
                    ));
                }
            }
            if let Some(s) = h.get("strategy").and_then(|v| v.as_str()) {
                if !KNOWN_KANI_STRATEGIES.contains(&s) {
                    offenders.push(format!(
                        "{}: kani_harnesses[{i}].strategy `{s}` is not in the schema vocabulary",
                        path.display()
                    ));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "malformed kani harness declarations never validate:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn a_contract_without_kani_harnesses_declares_itself_a_pattern() {
    // PROVABILITY-001: `pv validate` defaults to KERNEL, where equations and
    // kani harnesses are mandatory. A cross-cutting behavioural contract that
    // proves nothing bounded must say `metadata.kind: pattern` — otherwise it
    // is silently judged against a bar it was never meant to meet, and fails.
    let mut offenders = Vec::new();
    for path in contract_files() {
        let text = std::fs::read_to_string(&path).expect("readable");
        let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else {
            continue;
        };
        let has_harnesses = doc
            .get("kani_harnesses")
            .and_then(|v| v.as_sequence())
            .is_some_and(|s| !s.is_empty());
        let kind = doc
            .get("metadata")
            .and_then(|m| m.get("kind"))
            .and_then(|v| v.as_str());
        if !has_harnesses && kind != Some("pattern") {
            offenders.push(format!(
                "{}: no kani_harnesses and kind is {:?}; declare `kind: pattern`",
                path.display(),
                kind
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "contracts judged as kernel with nothing to prove:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn excluded_files_are_excluded_for_a_reason_and_still_exist() {
    // A stale exclusion is a hole. If binding.yaml is ever renamed or deleted,
    // this list must be updated deliberately rather than quietly covering
    // nothing.
    for name in NOT_CONTRACTS {
        let path = contracts_dir().join(name);
        assert!(
            path.exists(),
            "{} is excluded from contract validation but does not exist — \
             remove the stale exclusion",
            path.display()
        );
    }
}
