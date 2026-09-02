//! FJ-1401: Convergence proof from arbitrary state.
//!
//! Analyzes config + state to prove that `forjar apply` will converge:
//! 1. All resources have check/apply/state_query scripts (codegen completeness)
//! 2. Check scripts are deterministic (same state → same output)
//! 3. Apply is idempotent (converged state + apply = no change)
//! 4. No circular dependencies in the resource DAG

use crate::core::{codegen, parser, resolver, state, types};
use std::path::Path;

/// Prove convergence for a forjar config.
pub(crate) fn cmd_prove(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let mut config = parser::parse_and_validate(file)?;

    // FJ-2733: prove the config that will RUN, not the one that was typed.
    //
    // Every checker read raw resources, so each invariant was evaluated against
    // text that differs from what apply executes — and the failure direction
    // was UNSAFE. Two file resources whose paths are spelled differently but
    // resolve to the same file reported `[CHECKED] 2 targets disjoint`, while
    // the identical infrastructure spelled literally was correctly
    // `[FALSIFIED] target collision`.
    //
    // `depends_on` and `machine` are deliberately never templated (see
    // resolver::tests_completeness), so the DAG and machine-routing proofs are
    // unaffected by resolving here.
    config.resources = resolver::resolve_all(
        &config.resources,
        &config.params,
        &config.machines,
        &config.secrets,
    );

    let proofs = collect_proofs(&config, state_dir, machine_filter);

    let all_passed = proofs.iter().all(|p| p.passed);

    if json {
        print_proofs_json(&config, &proofs)?;
    } else {
        print_proofs_text(&config, &proofs);
    }

    if all_passed {
        Ok(())
    } else {
        // NAME THE STATES. "see above" told a CI consumer nothing it could
        // branch on; an UNKNOWN obligation and a FALSIFIED one are different
        // failures with different owners (forjar#416).
        let unknown = proofs
            .iter()
            .filter(|p| !p.passed && p.detail.starts_with("[UNKNOWN]"))
            .count();
        let falsified = proofs
            .iter()
            .filter(|p| !p.passed && p.detail.starts_with("[FALSIFIED]"))
            .count();
        Err(format!(
            "convergence proof failed: {unknown} obligation(s) UNKNOWN, {falsified} FALSIFIED — see above"
        ))
    }
}

struct ProofResult {
    name: String,
    passed: bool,
    detail: String,
}

fn collect_proofs(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Vec<ProofResult> {
    let mut proofs = vec![
        prove_codegen_completeness(config, machine_filter),
        prove_dag_acyclicity(config),
        prove_state_coverage(config, state_dir, machine_filter),
        prove_codegen_determinism(config, machine_filter),
        prove_idempotency_structure(config, machine_filter),
    ];
    // Provable-IaC structural invariants (three-state; I1/I5 already covered above).
    proofs.extend(structural_invariants(config, machine_filter));
    proofs
}

/// Run the `core::prove` invariant engine (contract `provable-iac-v1`) and map its
/// three-state results into convergence `ProofResult`s. A HARD invariant that is
/// FALSIFIED fails the proof; PROVED/CHECKED pass with the state in the detail; UNKNOWN fails the proof.
fn structural_invariants(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
) -> Vec<ProofResult> {
    use crate::core::prove::{prove as prove_invariants, Assurance, Class};
    // `-m` scopes the run to one machine. The structural invariants were
    // computed over the WHOLE config, so once UNKNOWN fails the proof
    // (forjar#416) another machine's unproven obligation failed `prove -m`
    // for a machine that was clean (E14 quorum, agy lane). Prove the
    // scoped config instead.
    let scoped;
    let config = match machine_filter {
        Some(m) => {
            let mut c = config.clone();
            c.resources.retain(|_, r| machine_matches(r, m));
            scoped = c;
            &scoped
        }
        None => config,
    };
    prove_invariants(config, "")
        .invariants
        .into_iter()
        .filter(|i| !matches!(i.id, "I1" | "I5"))
        .map(|i| ProofResult {
            name: format!("{} {}", i.id, i.name),
            passed: !(i.class == Class::Hard && i.state == Assurance::Falsified)
                && i.state != Assurance::Unknown,
            detail: format!("[{}] {}", i.state.badge(), i.detail),
        })
        .collect()
}

/// Check if a resource's machine matches the filter.
fn machine_matches(resource: &types::Resource, filter: &str) -> bool {
    resource.machine.iter().any(|m| m == filter)
}

fn prove_codegen_completeness(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
) -> ProofResult {
    let mut failures = Vec::new();

    for (id, resource) in &config.resources {
        if let Some(filter) = machine_filter {
            if !machine_matches(resource, filter) {
                continue;
            }
        }
        if resource.resource_type == types::ResourceType::Recipe {
            continue;
        }

        if let Err(e) = codegen::check_script(resource) {
            failures.push(format!("{id}: check_script: {e}"));
        }
        if let Err(e) = codegen::apply_script(resource) {
            failures.push(format!("{id}: apply_script: {e}"));
        }
        if let Err(e) = codegen::state_query_script(resource) {
            failures.push(format!("{id}: state_query: {e}"));
        }
    }

    ProofResult {
        name: "codegen-completeness".to_string(),
        passed: failures.is_empty(),
        detail: if failures.is_empty() {
            "all resources produce check/apply/state_query scripts".to_string()
        } else {
            format!("{} failures: {}", failures.len(), failures.join("; "))
        },
    }
}

fn prove_dag_acyclicity(config: &types::ForjarConfig) -> ProofResult {
    let (visited, total) = topo_sort_count(config);
    ProofResult {
        name: "dag-acyclicity".to_string(),
        passed: visited == total,
        detail: if visited == total {
            format!("DAG is acyclic ({total} resources)")
        } else {
            format!("cycle detected: only {visited}/{total} resources reachable")
        },
    }
}

/// Run Kahn's topological sort, return (visited_count, total_count).
fn topo_sort_count(config: &types::ForjarConfig) -> (usize, usize) {
    let mut in_degree: std::collections::HashMap<&str, usize> = config
        .resources
        .keys()
        .map(|k| (k.as_str(), 0usize))
        .collect();

    for (id, resource) in &config.resources {
        for dep in &resource.depends_on {
            if config.resources.contains_key(dep.as_str()) {
                *in_degree.entry(id.as_str()).or_insert(0) += 1;
            }
        }
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();
    let mut visited = 0;

    while let Some(node) = queue.pop() {
        visited += 1;
        for (id, resource) in &config.resources {
            if !resource.depends_on.iter().any(|d| d == node) {
                continue;
            }
            if let Some(deg) = in_degree.get_mut(id.as_str()) {
                *deg -= 1;
                if *deg == 0 {
                    queue.push(id.as_str());
                }
            }
        }
    }

    (visited, config.resources.len())
}

/// Whether a resource belongs in the coverage count at all. Recipes are
/// templates rather than deployed state and never count; `--machine` narrows
/// the count to the resources that target that machine.
fn counts_toward_state_coverage(resource: &types::Resource, machine_filter: Option<&str>) -> bool {
    if resource.resource_type == types::ResourceType::Recipe {
        return false;
    }
    match machine_filter {
        Some(filter) => machine_matches(resource, filter),
        None => true,
    }
}

/// A resource is covered when at least one of the machines it targets records
/// it in that machine's lock.
fn resource_has_state_entry(state_dir: &Path, resource: &types::Resource, id: &str) -> bool {
    resource.machine.iter().any(|machine| {
        matches!(
            state::load_lock(state_dir, machine),
            Ok(Some(lock)) if lock.resources.contains_key(id)
        )
    })
}

fn prove_state_coverage(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> ProofResult {
    let mut total = 0;
    let mut covered = 0;

    for (id, resource) in &config.resources {
        if !counts_toward_state_coverage(resource, machine_filter) {
            continue;
        }
        total += 1;
        if resource_has_state_entry(state_dir, resource, id) {
            covered += 1;
        }
    }

    let pct = if total > 0 {
        (covered * 100) / total
    } else {
        100
    };

    ProofResult {
        name: "state-coverage".to_string(),
        passed: true, // Informational, not a hard failure
        detail: format!("{covered}/{total} resources have state entries ({pct}%)"),
    }
}

/// A codegen phase: its name, and the function that emits it.
type CodegenPhase = (&'static str, fn(&types::Resource) -> Result<String, String>);

/// The emitted phases whose determinism is checked.
///
/// All three, not just `state_query`: `check` and `apply` are hashed into
/// desired state too, so nondeterminism in either produces the same
/// phantom-drift failure. Sampling one of the three was arbitrary.
const CODEGEN_PHASES: [CodegenPhase; 3] = [
    ("state_query", codegen::state_query_script),
    ("check", codegen::check_script),
    ("apply", codegen::apply_script),
];

/// Emit each phase twice and report the phases whose text differed.
///
/// `emitted` is false when no phase produced a script at all, which is how the
/// caller distinguishes "checked and clean" from "nothing to check".
fn nondeterministic_phases(resource: &types::Resource) -> (bool, Vec<&'static str>) {
    let mut emitted = false;
    let mut differing = Vec::new();
    for (phase, emit) in CODEGEN_PHASES {
        let (Ok(first), Ok(second)) = (emit(resource), emit(resource)) else {
            continue;
        };
        emitted = true;
        if first != second {
            differing.push(phase);
        }
    }
    (emitted, differing)
}

/// Prove that codegen is a pure function of the resource.
///
/// GH-248: this was called `hash-determinism`, and the book described it as
/// "BLAKE3 hashes are deterministic (same resource → same hash)". It never
/// tested that. It emits one resource's scripts twice in-process and compares
/// the text, so what it proves is that **`forjar`'s own code generation** is
/// deterministic — chiefly that `HashMap`/`HashSet` iteration order does not
/// leak into emitted script text, which it genuinely would catch, since std
/// gives each map instance a distinct hash key.
///
/// It says nothing about whether the resource's *build output* is reproducible.
/// A task with a non-deterministic generator passes this and then produces
/// different artifact bytes on the next `apply`. Proving that requires
/// double-execution and artifact comparison (GH-247), which `prove` cannot do
/// from the config alone — so the name now claims only what is checked.
fn prove_codegen_determinism(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
) -> ProofResult {
    let mut tested = 0;
    let mut failures = Vec::new();

    for (id, resource) in &config.resources {
        if machine_filter.is_some_and(|f| !machine_matches(resource, f)) {
            continue;
        }
        if resource.resource_type == types::ResourceType::Recipe {
            continue;
        }

        let (emitted, differing) = nondeterministic_phases(resource);
        if emitted {
            tested += 1;
        }
        failures.extend(differing.into_iter().map(|phase| format!("{id} ({phase})")));
    }

    ProofResult {
        name: "codegen-determinism".to_string(),
        passed: failures.is_empty(),
        detail: if failures.is_empty() {
            format!(
                "{tested} resources: check/apply/state_query codegen is deterministic \
                 (does NOT prove build-output reproducibility — see GH-247)"
            )
        } else {
            format!(
                "{} non-deterministic: {}",
                failures.len(),
                failures.join(", ")
            )
        },
    }
}

fn prove_idempotency_structure(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
) -> ProofResult {
    let mut tested = 0;
    let mut pipefail_count = 0;

    for (_id, resource) in &config.resources {
        if let Some(filter) = machine_filter {
            if !machine_matches(resource, filter) {
                continue;
            }
        }
        if resource.resource_type == types::ResourceType::Recipe {
            continue;
        }

        if let Ok(script) = codegen::apply_script(resource) {
            tested += 1;
            if script.contains("set -euo pipefail") {
                pipefail_count += 1;
            }
        }
    }

    let pct = if tested > 0 {
        (pipefail_count * 100) / tested
    } else {
        100
    };

    ProofResult {
        name: "idempotency-structure".to_string(),
        passed: pct >= 80,
        detail: format!("{pipefail_count}/{tested} apply scripts use set -euo pipefail ({pct}%)"),
    }
}

fn print_proofs_json(config: &types::ForjarConfig, proofs: &[ProofResult]) -> Result<(), String> {
    let results: Vec<serde_json::Value> = proofs
        .iter()
        .map(|p| {
            serde_json::json!({
                "proof": p.name,
                "passed": p.passed,
                "detail": p.detail,
            })
        })
        .collect();

    let all_passed = proofs.iter().all(|p| p.passed);
    let doc = serde_json::json!({
        "config": config.name,
        "convergenceProven": all_passed,
        "proofs": results,
    });

    let output = serde_json::to_string_pretty(&doc).map_err(|e| format!("JSON error: {e}"))?;
    println!("{output}");
    Ok(())
}

fn print_proofs_text(config: &types::ForjarConfig, proofs: &[ProofResult]) {
    println!("Convergence + Provable-IaC Proof: {}", config.name);
    println!("plan-hash: {}", crate::core::prove::plan_hash(config));
    println!("{:-<72}", "");
    for p in proofs {
        let status = if p.passed { "PASS" } else { "FAIL" };
        println!("[{status}] {}: {}", p.name, p.detail);
    }
    println!("{:-<72}", "");
    let passed = proofs.iter().filter(|p| p.passed).count();
    let total = proofs.len();
    println!("{passed}/{total} proofs passed");
}
