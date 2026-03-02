//! FJ-1329: Reproducibility score.
//!
//! Computes a 0-100 score reflecting how reproducible a recipe is, based on:
//! - Purity level distribution (50% weight)
//! - Store coverage (30% weight)
//! - Input lock coverage (20% weight)

use super::purity::PurityLevel;

/// Per-resource breakdown for the reproducibility report.
#[derive(Debug, Clone)]
pub struct ResourceScore {
    pub name: String,
    pub purity: PurityLevel,
    pub has_store: bool,
    pub has_lock_pin: bool,
    pub score: f64,
}

/// Overall reproducibility score for a recipe.
#[derive(Debug, Clone)]
pub struct ReproScore {
    pub composite: f64,
    pub purity_score: f64,
    pub store_score: f64,
    pub lock_score: f64,
    pub resources: Vec<ResourceScore>,
}

/// Signals for a single resource needed to compute its reproducibility.
#[derive(Debug, Clone)]
pub struct ReproInput {
    pub name: String,
    pub purity: PurityLevel,
    pub has_store: bool,
    pub has_lock_pin: bool,
}

/// Compute reproducibility score from resource inputs.
///
/// Scoring weights: purity 50%, store coverage 30%, lock coverage 20%.
pub fn compute_score(inputs: &[ReproInput]) -> ReproScore {
    if inputs.is_empty() {
        return ReproScore {
            composite: 100.0,
            purity_score: 100.0,
            store_score: 100.0,
            lock_score: 100.0,
            resources: vec![],
        };
    }

    let n = inputs.len() as f64;

    // Purity score: Pure=100, Pinned=75, Constrained=25, Impure=0
    let purity_total: f64 = inputs.iter().map(|r| purity_points(r.purity)).sum();
    let purity_score = purity_total / n;

    // Store coverage: percentage with store enabled
    let store_count = inputs.iter().filter(|r| r.has_store).count() as f64;
    let store_score = (store_count / n) * 100.0;

    // Lock coverage: percentage with lock file pin
    let lock_count = inputs.iter().filter(|r| r.has_lock_pin).count() as f64;
    let lock_score = (lock_count / n) * 100.0;

    // Weighted composite
    let composite = purity_score * 0.5 + store_score * 0.3 + lock_score * 0.2;

    let resources = inputs
        .iter()
        .map(|r| {
            let score = purity_points(r.purity) * 0.5
                + if r.has_store { 100.0 } else { 0.0 } * 0.3
                + if r.has_lock_pin { 100.0 } else { 0.0 } * 0.2;
            ResourceScore {
                name: r.name.clone(),
                purity: r.purity,
                has_store: r.has_store,
                has_lock_pin: r.has_lock_pin,
                score,
            }
        })
        .collect();

    ReproScore {
        composite,
        purity_score,
        store_score,
        lock_score,
        resources,
    }
}

/// Map a purity level to a 0-100 score.
fn purity_points(level: PurityLevel) -> f64 {
    match level {
        PurityLevel::Pure => 100.0,
        PurityLevel::Pinned => 75.0,
        PurityLevel::Constrained => 25.0,
        PurityLevel::Impure => 0.0,
    }
}

/// Grade label from composite score.
pub fn grade(score: f64) -> &'static str {
    if score >= 90.0 {
        "A"
    } else if score >= 75.0 {
        "B"
    } else if score >= 50.0 {
        "C"
    } else if score >= 25.0 {
        "D"
    } else {
        "F"
    }
}
