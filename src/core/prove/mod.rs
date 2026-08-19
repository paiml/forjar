//! `forjar prove` — Provable IaC.
//!
//! Runs the proof-ladder invariant checkers over a parsed `forjar.yaml`. This is
//! config **validation raised to the proof ladder** — the honest analog of
//! `terraform validate` with machine-checked validators — NOT "safe to apply":
//! every invariant here is a pure function of the config, and the remote shell
//! that performs the actual mutation is outside the trusted computing base.
//!
//! Assurance is three-state and never collapsed (spec: `docs/specifications/provable-iac.md`):
//!   * `Proved`   — a Lean theorem transfers to this instance (I1: acyclic ⇒ an apply order exists).
//!   * `Checked`  — a (Kani-verified) decision procedure ran on this concrete config and found nothing.
//!   * `Unknown`  — an opaque/impure resource the analysis cannot see through.
//!   * `Falsified`— the invariant does not hold for this config (a HARD one blocks apply).
//!
//! Contract: `contracts/provable-iac-v1.yaml`; Lean: `lean/ProvableIac.lean`.

use crate::core::types::{ForjarConfig, Resource, ResourceType};
use std::collections::{BTreeMap, BTreeSet};

mod checkers;
#[cfg(test)]
mod tests;

pub use checkers::{
    acyclicity, conflict_freedom, dependency_completeness, has_conflict, input_purity,
};

/// Per-invariant assurance state. Ordering: Proved > Checked > Unknown; Falsified is failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assurance {
    Proved,
    Checked,
    Unknown,
    Falsified,
}

impl Assurance {
    #[must_use]
    pub fn badge(self) -> &'static str {
        match self {
            Self::Proved => "PROVED",
            Self::Checked => "CHECKED",
            Self::Unknown => "UNKNOWN",
            Self::Falsified => "FALSIFIED",
        }
    }
}

/// Enforcement class. A falsified HARD invariant blocks apply; ADV only warns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Hard,
    Advisory,
}

/// One invariant's result on a specific config.
#[derive(Debug, Clone)]
pub struct InvariantResult {
    pub id: &'static str,
    pub name: &'static str,
    pub class: Class,
    pub state: Assurance,
    pub detail: String,
}

/// The full provability report for a config.
#[derive(Debug, Clone)]
pub struct ProofReport {
    pub file: String,
    pub resource_count: usize,
    pub declarative: usize,
    pub opaque: usize,
    pub invariants: Vec<InvariantResult>,
    /// blake3 over the canonical (sorted) resource-key + target set — the plan-artifact
    /// hash `forjar apply` must consume to make the L5 binding real (spec must-fix #1).
    pub plan_hash: String,
}

impl ProofReport {
    /// A config PASSES iff no HARD invariant is falsified.
    #[must_use]
    pub fn passed(&self) -> bool {
        !self
            .invariants
            .iter()
            .any(|i| i.class == Class::Hard && i.state == Assurance::Falsified)
    }

    /// Whether any opaque (imperative) resource is present — degrades several invariants.
    #[must_use]
    pub fn has_opaque(&self) -> bool {
        self.opaque > 0
    }
}

/// Resource kinds whose target and effect are opaque to static analysis (imperative).
/// Their presence downgrades I3/I5/I6/I7 to UNKNOWN — a vacuous green is worse than no gate.
#[must_use]
pub fn is_opaque(t: &ResourceType) -> bool {
    matches!(
        t,
        ResourceType::Cron | ResourceType::Task | ResourceType::Recipe | ResourceType::Build
    )
}

/// The normalized target(s) a declarative resource writes (for conflict-freedom).
/// `None` ⇒ opaque / no analyzable target.
#[must_use]
pub fn resource_targets(r: &Resource) -> Option<Vec<String>> {
    if is_opaque(&r.resource_type) {
        return None;
    }
    let mut out = Vec::new();
    match &r.resource_type {
        ResourceType::File => {
            if let Some(p) = &r.path {
                out.push(format!("path:{}", normalize_path(p)));
            }
        }
        ResourceType::Mount => {
            if let Some(t) = r.target.as_ref().or(r.path.as_ref()) {
                out.push(format!("mount:{}", normalize_path(t)));
            }
        }
        ResourceType::Package => {
            for p in &r.packages {
                out.push(format!("pkg:{p}"));
            }
        }
        ResourceType::Service => {
            if let Some(n) = &r.name {
                out.push(format!("svc:{n}"));
            }
        }
        ResourceType::User => {
            if let Some(n) = &r.name {
                out.push(format!("user:{n}"));
            }
        }
        ResourceType::Network | ResourceType::OverlayInterface => {
            if let Some(n) = r.name.as_ref().or(r.target.as_ref()) {
                out.push(format!("net:{n}"));
            }
        }
        _ => {
            // Other declarative kinds: use name/target/path if present, else no target.
            if let Some(n) = r.name.as_ref().or(r.target.as_ref()).or(r.path.as_ref()) {
                out.push(format!("res:{n}"));
            }
        }
    }
    Some(out)
}

/// Canonical path normalization for target subsumption (trailing slash, `.` segments).
#[must_use]
pub fn normalize_path(p: &str) -> String {
    let mut segs: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                segs.pop();
            }
            s => segs.push(s),
        }
    }
    let lead = if p.starts_with('/') { "/" } else { "" };
    format!("{lead}{}", segs.join("/"))
}

/// Run every invariant checker over `cfg` and produce the report.
#[must_use]
pub fn prove(cfg: &ForjarConfig, file: &str) -> ProofReport {
    let declarative = cfg
        .resources
        .values()
        .filter(|r| !is_opaque(&r.resource_type))
        .count();
    let opaque = cfg.resources.len() - declarative;

    let invariants = vec![
        acyclicity(cfg),
        dependency_completeness(cfg),
        conflict_freedom(cfg),
        checkers::idempotency(cfg),
        checkers::protected_safety(cfg),
        input_purity(cfg),
    ];

    ProofReport {
        file: file.to_string(),
        resource_count: cfg.resources.len(),
        declarative,
        opaque,
        invariants,
        plan_hash: plan_hash(cfg),
    }
}

/// blake3 over the sorted (key, type, targets, deps) tuples — stable, order-independent.
#[must_use]
pub fn plan_hash(cfg: &ForjarConfig) -> String {
    let mut items: BTreeMap<&String, String> = BTreeMap::new();
    for (k, r) in &cfg.resources {
        let targets = resource_targets(r).unwrap_or_default().join(",");
        let mut deps: Vec<&String> = r.depends_on.iter().collect();
        deps.sort();
        let deps: BTreeSet<&&String> = deps.iter().collect();
        items.insert(
            k,
            format!(
                "{}|{}|{targets}|{}",
                r.resource_type,
                r.machine,
                deps.iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        );
    }
    let mut hasher = blake3::Hasher::new();
    for (k, v) in &items {
        hasher.update(k.as_bytes());
        hasher.update(b"\0");
        hasher.update(v.as_bytes());
        hasher.update(b"\n");
    }
    format!("b3:{}", &hasher.finalize().to_hex()[..16])
}
