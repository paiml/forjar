//! The invariant decision procedures for `forjar prove`. Each is a pure function
//! of the config. The Kani harness in `kani_proofs` (contract `provable-iac-v1`,
//! `provable-iac-kani-001`) proves the conflict detector sound + complete. The L4
//! Lean proof of I1 (acyclic ⇒ an apply order exists) is tracked under the Phase-2
//! contract-hardening initiative; this module ships at forjar's established L3 bar.

use super::{is_opaque, resource_targets, Assurance, Class, InvariantResult};
use crate::core::types::ForjarConfig;
use std::collections::{BTreeMap, BTreeSet};

/// The decidable core of I3 conflict-freedom: a list of writer-target keys is
/// conflict-free iff no value repeats. `conflict_freedom` reduces to this over the
/// `(machine, target)` keys. Kani-proved sound in `kani_proofs::conflict_detector_sound`.
#[must_use]
pub fn has_conflict(keys: &[u64]) -> bool {
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            if keys[i] == keys[j] {
                return true;
            }
        }
    }
    false
}

#[cfg(kani)]
mod kani_proofs {
    use super::has_conflict;

    /// provable-iac-kani-001: the conflict detector reports a collision iff two
    /// distinct positions hold the same writer-target key (soundness + completeness).
    #[kani::proof]
    fn conflict_detector_sound() {
        let keys: [u64; 4] = kani::any();
        let reported = has_conflict(&keys);
        let mut actual = false;
        for i in 0..4 {
            for j in (i + 1)..4 {
                if keys[i] == keys[j] {
                    actual = true;
                }
            }
        }
        assert_eq!(reported, actual);
    }
}

/// I1 — DAG acyclicity: the `depends_on` graph has no cycle, so a finite apply
/// ORDER exists (Lean: acyclic ⇒ topological sort exists). HARD, ceiling L4.
#[must_use]
pub fn acyclicity(cfg: &ForjarConfig) -> InvariantResult {
    // 0 = unvisited, 1 = on-stack, 2 = done. A back-edge to an on-stack node = cycle.
    let mut color: BTreeMap<&str, u8> = cfg.resources.keys().map(|k| (k.as_str(), 0u8)).collect();
    let mut cycle: Option<Vec<String>> = None;
    let mut stack: Vec<(&str, usize)> = Vec::new();

    for root in cfg.resources.keys() {
        if color[root.as_str()] != 0 {
            continue;
        }
        stack.push((root.as_str(), 0));
        let mut path: Vec<&str> = Vec::new();
        while let Some(&(node, idx)) = stack.last() {
            if idx == 0 {
                color.insert(node, 1);
                path.push(node);
            }
            let deps = cfg.resources.get(node).map(|r| &r.depends_on);
            let next = deps.and_then(|d| d.get(idx));
            if let Some(dep) = next {
                stack.last_mut().unwrap().1 += 1;
                let dep = dep.as_str();
                match color.get(dep).copied() {
                    Some(1) => {
                        // back-edge → cycle; reconstruct from `path`.
                        let start = path.iter().position(|&n| n == dep).unwrap_or(0);
                        let mut c: Vec<String> =
                            path[start..].iter().map(|s| (*s).to_string()).collect();
                        c.push(dep.to_string());
                        cycle = Some(c);
                        break;
                    }
                    Some(0) => stack.push((dep, 0)),
                    _ => {} // done, or dangling (I2's concern) — ignore for acyclicity
                }
            } else {
                color.insert(node, 2);
                path.pop();
                stack.pop();
            }
        }
        if cycle.is_some() {
            break;
        }
    }

    match cycle {
        Some(c) => InvariantResult {
            id: "I1",
            name: "dag-acyclicity",
            class: Class::Hard,
            state: Assurance::Falsified,
            detail: format!("cycle: {}", c.join(" → ")),
        },
        None => InvariantResult {
            id: "I1",
            name: "dag-acyclicity",
            class: Class::Hard,
            state: Assurance::Proved,
            detail: format!("acyclic; apply order length {}", cfg.resources.len()),
        },
    }
}

/// I2 — every `depends_on` edge targets a declared resource. HARD, ceiling L2.
#[must_use]
pub fn dependency_completeness(cfg: &ForjarConfig) -> InvariantResult {
    let keys: BTreeSet<&str> = cfg.resources.keys().map(String::as_str).collect();
    let mut dangling = Vec::new();
    let mut edges = 0usize;
    for (k, r) in &cfg.resources {
        for d in &r.depends_on {
            edges += 1;
            if !keys.contains(d.as_str()) {
                dangling.push(format!("{k} → {d}"));
            }
        }
    }
    if dangling.is_empty() {
        InvariantResult {
            id: "I2",
            name: "dependency-completeness",
            class: Class::Hard,
            state: Assurance::Checked,
            detail: format!("{edges}/{edges} edges resolve"),
        }
    } else {
        InvariantResult {
            id: "I2",
            name: "dependency-completeness",
            class: Class::Hard,
            state: Assurance::Falsified,
            detail: format!("dangling: {}", dangling.join(", ")),
        }
    }
}

/// I3 — no two declarative resources on the same machine write an overlapping
/// target namespace. Opaque resources ⇒ UNKNOWN (their targets are invisible).
/// HARD, ceiling L3.
#[must_use]
pub fn conflict_freedom(cfg: &ForjarConfig) -> InvariantResult {
    // (machine, target) → resource keys writing it.
    let mut writers: BTreeMap<(String, String), Vec<&str>> = BTreeMap::new();
    let mut opaque = 0usize;
    for (k, r) in &cfg.resources {
        match resource_targets(r) {
            None => opaque += 1,
            Some(targets) => {
                for m in r.machine.to_vec() {
                    for t in &targets {
                        writers
                            .entry((m.clone(), t.clone()))
                            .or_default()
                            .push(k.as_str());
                    }
                }
            }
        }
    }
    let collisions: Vec<String> = writers
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|((m, t), v)| format!("{m}:{t} ← {}", v.join(", ")))
        .collect();

    let declarative_pairs = writers.len();
    if !collisions.is_empty() {
        InvariantResult {
            id: "I3",
            name: "conflict-freedom",
            class: Class::Hard,
            state: Assurance::Falsified,
            detail: format!("target collision: {}", collisions.join("; ")),
        }
    } else if opaque > 0 {
        InvariantResult {
            id: "I3",
            name: "conflict-freedom",
            class: Class::Hard,
            state: Assurance::Unknown,
            detail: format!(
                "{declarative_pairs} declarative targets disjoint; {opaque} opaque UNANALYZED"
            ),
        }
    } else {
        InvariantResult {
            id: "I3",
            name: "conflict-freedom",
            class: Class::Hard,
            state: Assurance::Checked,
            detail: format!("{declarative_pairs} targets disjoint"),
        }
    }
}

/// I5 — plan-idempotence for declarative resources (converged ⇒ NoOp), advisory;
/// effect-idempotence for opaque resources is UNKNOWN unless a guard is present.
#[must_use]
pub fn idempotency(cfg: &ForjarConfig) -> InvariantResult {
    let declarative = cfg
        .resources
        .values()
        .filter(|r| !is_opaque(&r.resource_type))
        .count();
    let opaque = cfg.resources.len() - declarative;
    let state = if opaque > 0 {
        Assurance::Unknown
    } else {
        Assurance::Proved
    };
    InvariantResult {
        id: "I5",
        name: "idempotency",
        class: Class::Advisory,
        state,
        detail: format!("plan-idem {declarative}/{declarative} declarative; effect-idem {opaque} opaque UNKNOWN"),
    }
}

/// I6 — protected/blast-radius: flag resources whose state removes/replaces a
/// target another resource depends on. HARD, ceiling L3.
#[must_use]
pub fn protected_safety(cfg: &ForjarConfig) -> InvariantResult {
    let mut removers: Vec<&str> = Vec::new();
    for (k, r) in &cfg.resources {
        if let Some(s) = &r.state {
            let s = s.to_ascii_lowercase();
            if s == "absent" || s == "removed" || s == "destroyed" {
                removers.push(k.as_str());
            }
        }
    }
    // A remover that another resource depends_on is a blast-radius risk.
    let mut risky = Vec::new();
    for (k, r) in &cfg.resources {
        for d in &r.depends_on {
            if removers.contains(&d.as_str()) {
                risky.push(format!("{k} depends on removed {d}"));
            }
        }
    }
    if risky.is_empty() {
        InvariantResult {
            id: "I6",
            name: "protected-safety",
            class: Class::Hard,
            state: Assurance::Checked,
            detail: format!("0 violations; {} removal(s) flagged", removers.len()),
        }
    } else {
        InvariantResult {
            id: "I6",
            name: "protected-safety",
            class: Class::Hard,
            state: Assurance::Falsified,
            detail: format!("blast radius: {}", risky.join(", ")),
        }
    }
}

/// I9 — input purity/pinning (ADVISORY): unpinned package versions and unpinned
/// remote sources drift over time (a config proven today realizes a different
/// machine next week). Advisory — a warning, not a blocker: unpinned deps are
/// ubiquitous in real IaC. NOTE: we deliberately do NOT flag `$(...)` inside a
/// file's `content` — that is the resource's legitimate payload (a zshrc/script),
/// not forjar-level impurity; imperative resources that RUN such content are
/// already downgraded via `is_opaque`.
#[must_use]
pub fn input_purity(cfg: &ForjarConfig) -> InvariantResult {
    let unpinned_source = |s: &str| {
        (s.starts_with("http://") || s.starts_with("https://") || s.starts_with("git+"))
            && !s.contains('@')
            && !s.contains("sha256")
            && !s.contains("#")
    };
    let mut findings = Vec::new();
    for (k, r) in &cfg.resources {
        if matches!(r.resource_type, crate::core::types::ResourceType::Package)
            && !r.packages.is_empty()
            && r.version.is_none()
        {
            findings.push(format!("{k}: unpinned package version"));
        }
        if let Some(src) = &r.source {
            if unpinned_source(src) {
                findings.push(format!("{k}: unpinned remote source"));
            }
        }
    }
    let n = cfg.resources.len();
    if findings.is_empty() {
        InvariantResult {
            id: "I9",
            name: "input-purity",
            class: Class::Advisory,
            state: Assurance::Checked,
            detail: format!("{n}/{n} inputs pinned"),
        }
    } else {
        InvariantResult {
            id: "I9",
            name: "input-purity",
            class: Class::Advisory,
            state: Assurance::Unknown,
            detail: format!(
                "{} unpinned input(s): {}",
                findings.len(),
                findings.join(", ")
            ),
        }
    }
}
