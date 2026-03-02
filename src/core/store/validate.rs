//! FJ-1306 / FJ-1329: Validation commands for purity and reproducibility.
//!
//! Implements `forjar validate --check-recipe-purity` and
//! `forjar validate --check-reproducibility-score` logic.

use super::purity::{classify, level_label, recipe_purity, PurityLevel, PurityResult, PuritySignals};
use super::repro_score::{compute_score, grade, ReproInput, ReproScore};
use serde::{Deserialize, Serialize};

/// A single resource's purity validation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourcePurityReport {
    pub name: String,
    pub level: PurityLevel,
    pub label: String,
    pub reasons: Vec<String>,
}

/// Overall recipe purity validation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PurityValidation {
    pub resources: Vec<ResourcePurityReport>,
    pub recipe_purity: PurityLevel,
    pub recipe_label: String,
    pub pass: bool,
    pub required_level: Option<PurityLevel>,
}

/// Validate recipe purity, optionally requiring a minimum level.
pub fn validate_purity(
    signals: &[(&str, &PuritySignals)],
    min_level: Option<PurityLevel>,
) -> PurityValidation {
    let mut resources = Vec::new();
    let mut levels = Vec::new();

    for (name, sig) in signals {
        let result: PurityResult = classify(name, sig);
        levels.push(result.level);
        resources.push(ResourcePurityReport {
            name: name.to_string(),
            level: result.level,
            label: level_label(result.level).to_string(),
            reasons: result.reasons,
        });
    }

    let overall = recipe_purity(&levels);
    let pass = min_level.is_none_or(|min| purity_ord(overall) <= purity_ord(min));

    PurityValidation {
        resources,
        recipe_purity: overall,
        recipe_label: level_label(overall).to_string(),
        pass,
        required_level: min_level,
    }
}

/// Validate reproducibility score, optionally requiring a minimum score.
pub fn validate_repro_score(
    inputs: &[ReproInput],
    min_score: Option<f64>,
) -> ReproValidation {
    let score = compute_score(inputs);
    let pass = min_score.is_none_or(|min| score.composite >= min);

    ReproValidation {
        score: score.clone(),
        grade: grade(score.composite).to_string(),
        pass,
        required_min: min_score,
    }
}

/// Reproducibility score validation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReproValidation {
    pub score: ReproScore,
    pub grade: String,
    pub pass: bool,
    pub required_min: Option<f64>,
}

/// Format purity validation for display.
pub fn format_purity_report(validation: &PurityValidation) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Recipe purity: {} ({})",
        validation.recipe_label,
        if validation.pass { "PASS" } else { "FAIL" }
    ));
    for r in &validation.resources {
        lines.push(format!(
            "  {}: {} — {}",
            r.name,
            r.label,
            r.reasons.join("; ")
        ));
    }
    if let Some(required) = validation.required_level {
        lines.push(format!(
            "  Required: {} or better",
            level_label(required)
        ));
    }
    lines.join("\n")
}

/// Format reproducibility validation for display.
pub fn format_repro_report(validation: &ReproValidation) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Reproducibility: {:.1}/100 (Grade {}) — {}",
        validation.score.composite,
        validation.grade,
        if validation.pass { "PASS" } else { "FAIL" }
    ));
    lines.push(format!("  Purity:  {:.1}", validation.score.purity_score));
    lines.push(format!("  Store:   {:.1}", validation.score.store_score));
    lines.push(format!("  Lock:    {:.1}", validation.score.lock_score));
    if let Some(min) = validation.required_min {
        lines.push(format!("  Required: >= {:.1}", min));
    }
    lines.join("\n")
}

fn purity_ord(level: PurityLevel) -> u8 {
    match level {
        PurityLevel::Pure => 0,
        PurityLevel::Pinned => 1,
        PurityLevel::Constrained => 2,
        PurityLevel::Impure => 3,
    }
}
