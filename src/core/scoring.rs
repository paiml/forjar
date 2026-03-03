//! Forjar Score — multi-dimensional recipe quality grading (A–F).
//!
//! Implements the scoring spec from the Forjar Cookbook: 8 dimensions with
//! weighted composite scoring and minimum-per-dimension grade gates.

use super::types::ForjarConfig;
use std::path::Path;

// ============================================================================
// Types
// ============================================================================

/// Input data for scoring. Static fields come from config analysis;
/// runtime fields come from actual apply results (optional).
pub struct ScoringInput {
    /// Recipe qualification status: "qualified", "blocked", "pending".
    pub status: String,
    /// Idempotency class: "strong", "weak", "eventual".
    pub idempotency: String,
    /// Performance budget in milliseconds (0 = no budget).
    pub budget_ms: u64,
    /// Runtime data from actual apply (None = static-only scoring).
    pub runtime: Option<RuntimeData>,
}

/// Runtime data collected from actual apply/qualify runs.
pub struct RuntimeData {
    pub validate_pass: bool,
    pub plan_pass: bool,
    pub first_apply_pass: bool,
    pub second_apply_pass: bool,
    pub zero_changes_on_reapply: bool,
    pub hash_stable: bool,
    pub all_resources_converged: bool,
    pub state_lock_written: bool,
    pub warning_count: u32,
    pub changed_on_reapply: u32,
    pub first_apply_ms: u64,
    pub second_apply_ms: u64,
}

/// Per-dimension score (0–100).
#[derive(Debug, Clone)]
pub struct DimensionScore {
    pub code: &'static str,
    pub name: &'static str,
    pub score: u32,
    pub weight: f64,
}

/// Complete scoring result.
#[derive(Debug, Clone)]
pub struct ScoringResult {
    pub dimensions: Vec<DimensionScore>,
    pub composite: u32,
    pub grade: char,
    pub hard_fail: bool,
    pub hard_fail_reason: Option<String>,
}

// ============================================================================
// Scoring engine
// ============================================================================

/// Compute Forjar Score for a config file.
pub fn compute_from_file(file: &Path, input: &ScoringInput) -> Result<ScoringResult, String> {
    let raw = std::fs::read_to_string(file).map_err(|e| format!("read: {e}"))?;
    let config: ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| format!("parse: {e}"))?;
    Ok(compute(&config, input))
}

/// Compute Forjar Score from a parsed config.
pub fn compute(config: &ForjarConfig, input: &ScoringInput) -> ScoringResult {
    // Hard-fail checks
    if let Some(reason) = check_hard_fail(input) {
        return ScoringResult {
            dimensions: all_zero_dimensions(),
            composite: 0,
            grade: 'F',
            hard_fail: true,
            hard_fail_reason: Some(reason),
        };
    }

    let dims = vec![
        score_correctness(input),
        score_idempotency(input),
        score_performance(input),
        score_safety(config),
        score_observability(config),
        score_documentation(config),
        score_resilience(config),
        score_composability(config),
    ];

    let composite = compute_composite(&dims);
    let min_dim = dims.iter().map(|d| d.score).min().unwrap_or(0);
    let grade = determine_grade(composite, min_dim);

    ScoringResult {
        dimensions: dims,
        composite,
        grade,
        hard_fail: false,
        hard_fail_reason: None,
    }
}

pub(super) fn check_hard_fail(input: &ScoringInput) -> Option<String> {
    match input.status.as_str() {
        "blocked" => Some("status is blocked".to_string()),
        "pending" => Some("status is pending (never qualified)".to_string()),
        _ => None,
    }
}

pub(super) fn all_zero_dimensions() -> Vec<DimensionScore> {
    vec![
        DimensionScore {
            code: "COR",
            name: "Correctness",
            score: 0,
            weight: 0.20,
        },
        DimensionScore {
            code: "IDM",
            name: "Idempotency",
            score: 0,
            weight: 0.20,
        },
        DimensionScore {
            code: "PRF",
            name: "Performance",
            score: 0,
            weight: 0.15,
        },
        DimensionScore {
            code: "SAF",
            name: "Safety",
            score: 0,
            weight: 0.15,
        },
        DimensionScore {
            code: "OBS",
            name: "Observability",
            score: 0,
            weight: 0.10,
        },
        DimensionScore {
            code: "DOC",
            name: "Documentation",
            score: 0,
            weight: 0.08,
        },
        DimensionScore {
            code: "RES",
            name: "Resilience",
            score: 0,
            weight: 0.07,
        },
        DimensionScore {
            code: "CMP",
            name: "Composability",
            score: 0,
            weight: 0.05,
        },
    ]
}

pub(crate) fn compute_composite(dims: &[DimensionScore]) -> u32 {
    let weighted: f64 = dims.iter().map(|d| d.score as f64 * d.weight).sum();
    weighted.round() as u32
}

pub(crate) fn determine_grade(composite: u32, min_dim: u32) -> char {
    if composite >= 90 && min_dim >= 80 {
        'A'
    } else if composite >= 75 && min_dim >= 60 {
        'B'
    } else if composite >= 60 && min_dim >= 40 {
        'C'
    } else if composite >= 40 {
        'D'
    } else {
        'F'
    }
}

// ============================================================================
// Dimension scorers
// ============================================================================

pub(super) fn score_correctness(input: &ScoringInput) -> DimensionScore {
    let score = match &input.runtime {
        Some(rt) => {
            let mut s: i32 = 0;
            if rt.validate_pass {
                s += 20;
            }
            if rt.plan_pass {
                s += 20;
            }
            if rt.first_apply_pass {
                s += 40;
            }
            if rt.all_resources_converged {
                s += 10;
            }
            if rt.state_lock_written {
                s += 10;
            }
            s -= (rt.warning_count.min(5) * 2) as i32;
            s.clamp(0, 100) as u32
        }
        None => 0,
    };
    DimensionScore {
        code: "COR",
        name: "Correctness",
        score,
        weight: 0.20,
    }
}

pub(super) fn score_idempotency(input: &ScoringInput) -> DimensionScore {
    let score = match &input.runtime {
        Some(rt) => {
            let mut s: i32 = 0;
            if rt.second_apply_pass {
                s += 30;
            }
            if rt.zero_changes_on_reapply {
                s += 30;
            }
            if rt.hash_stable {
                s += 20;
            }
            s += match input.idempotency.as_str() {
                "strong" => 20,
                "weak" => 10,
                _ => 0,
            };
            s -= (rt.changed_on_reapply.min(5) * 10) as i32;
            s.clamp(0, 100) as u32
        }
        None => {
            // Static-only: award idempotency class bonus only
            match input.idempotency.as_str() {
                "strong" => 20,
                "weak" => 10,
                _ => 0,
            }
        }
    };
    DimensionScore {
        code: "IDM",
        name: "Idempotency",
        score,
        weight: 0.20,
    }
}

pub(super) fn score_performance(input: &ScoringInput) -> DimensionScore {
    let score = match &input.runtime {
        Some(rt) if input.budget_ms > 0 => {
            let budget_pts = perf_budget_points(rt.first_apply_ms, input.budget_ms);
            let idem_pts = perf_idempotent_points(rt.second_apply_ms);
            let eff_pts = perf_efficiency_points(rt.second_apply_ms, rt.first_apply_ms);
            (budget_pts + idem_pts + eff_pts).min(100)
        }
        _ => 0,
    };
    DimensionScore {
        code: "PRF",
        name: "Performance",
        score,
        weight: 0.15,
    }
}

/// Points for first-apply vs budget ratio.
pub(super) fn perf_budget_points(first_ms: u64, budget_ms: u64) -> u32 {
    let ratio_pct = (first_ms * 100) / budget_ms.max(1);
    match ratio_pct {
        0..=50 => 50,
        51..=75 => 40,
        76..=100 => 30,
        101..=150 => 15,
        _ => 0,
    }
}

/// Points for idempotent re-apply speed.
pub(super) fn perf_idempotent_points(idem_ms: u64) -> u32 {
    match idem_ms {
        0..=2000 => 30,
        2001..=5000 => 25,
        5001..=10000 => 15,
        _ => 0,
    }
}

/// Points for efficiency ratio (re-apply / first-apply).
pub(super) fn perf_efficiency_points(second_ms: u64, first_ms: u64) -> u32 {
    let ratio = if first_ms > 0 {
        (second_ms * 100) / first_ms
    } else {
        0
    };
    match ratio {
        0..=5 => 20,
        6..=10 => 15,
        11..=25 => 10,
        _ => 0,
    }
}

pub use super::scoring_b::*;
