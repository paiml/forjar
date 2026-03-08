//! Forjar Score v2 — two-tier recipe quality grading.
//!
//! Static grade (design quality, always available) + Runtime grade
//! (operational quality, after apply). Addresses five v1 structural defects.
//! Spec: FJ-2800–FJ-2803.

use super::types::ForjarConfig;
use std::path::Path;

/// Scoring algorithm version.
pub const SCORE_VERSION: &str = "2.0";

// ============================================================================
// Types
// ============================================================================

/// Input data for scoring a recipe across 8 quality dimensions.
pub struct ScoringInput {
    /// Recipe qualification status: "qualified", "blocked", "pending".
    pub status: String,
    /// Idempotency class: "strong", "weak", "eventual".
    pub idempotency: String,
    /// Performance budget in milliseconds (0 = no budget).
    pub budget_ms: u64,
    /// Runtime data from actual apply (None = static-only scoring).
    pub runtime: Option<RuntimeData>,
    /// Raw YAML text for DOC scoring (header metadata, unique comments).
    pub raw_yaml: Option<String>,
}

/// Runtime data collected from actual apply/qualify runs.
#[derive(Clone)]
pub struct RuntimeData {
    /// Whether config validation passed.
    pub validate_pass: bool,
    /// Whether plan generation passed.
    pub plan_pass: bool,
    /// Whether first apply succeeded.
    pub first_apply_pass: bool,
    /// Whether second (idempotency) apply succeeded.
    pub second_apply_pass: bool,
    /// Whether re-apply produced zero changes.
    pub zero_changes_on_reapply: bool,
    /// Whether state hashes are stable across runs.
    pub hash_stable: bool,
    /// Whether all resources converged.
    pub all_resources_converged: bool,
    /// Whether the state lock file was written.
    pub state_lock_written: bool,
    /// Number of warnings emitted.
    pub warning_count: u32,
    /// Number of resources changed on re-apply.
    pub changed_on_reapply: u32,
    /// First apply duration in milliseconds.
    pub first_apply_ms: u64,
    /// Second apply duration in milliseconds.
    pub second_apply_ms: u64,
}

/// Per-dimension score (0–100).
#[derive(Debug, Clone)]
pub struct DimensionScore {
    /// Short code (e.g., "COR", "IDM").
    pub code: &'static str,
    /// Full dimension name.
    pub name: &'static str,
    /// Score value (0-100).
    pub score: u32,
    /// Weight in composite calculation (v2 static or runtime weight).
    pub weight: f64,
}

/// Complete scoring result with two-tier grading.
#[derive(Debug, Clone)]
pub struct ScoringResult {
    /// Per-dimension score breakdown (all 8 dimensions).
    pub dimensions: Vec<DimensionScore>,
    /// Static composite score (SAF+OBS+DOC+RES+CMP, 0-100).
    pub static_composite: u32,
    /// Static letter grade.
    pub static_grade: char,
    /// Runtime composite score (COR+IDM+PRF, 0-100). None if no runtime data.
    pub runtime_composite: Option<u32>,
    /// Runtime letter grade. None if no runtime data.
    pub runtime_grade: Option<char>,
    /// Overall grade: min(static, runtime) or static with "-pending" suffix.
    pub grade: String,
    /// Legacy composite (weighted across all 8 dims for backward compat).
    pub composite: u32,
    /// Whether a hard-fail condition was triggered.
    pub hard_fail: bool,
    /// Reason for hard failure, if any.
    pub hard_fail_reason: Option<String>,
}

// ============================================================================
// v2 weights
// ============================================================================

// Static tier weights (must sum to 100)
const W_SAF: u32 = 25;
const W_OBS: u32 = 20;
const W_DOC: u32 = 15;
const W_RES: u32 = 20;
const W_CMP: u32 = 20;

// Runtime tier weights (must sum to 100)
const W_COR: u32 = 35;
const W_IDM: u32 = 35;
const W_PRF: u32 = 30;

// ============================================================================
// Scoring engine
// ============================================================================

/// Compute Forjar Score for a config file.
pub fn compute_from_file(file: &Path, input: &ScoringInput) -> Result<ScoringResult, String> {
    let raw = std::fs::read_to_string(file).map_err(|e| format!("read: {e}"))?;
    let config: ForjarConfig = serde_yaml_ng::from_str(&raw).map_err(|e| format!("parse: {e}"))?;
    // Use file contents as raw_yaml if not provided
    if input.raw_yaml.is_some() {
        return Ok(compute(&config, input));
    }
    let input_with_yaml = ScoringInput {
        status: input.status.clone(),
        idempotency: input.idempotency.clone(),
        budget_ms: input.budget_ms,
        runtime: input.runtime.clone(),
        raw_yaml: Some(raw),
    };
    Ok(compute(&config, &input_with_yaml))
}

/// Compute Forjar Score from a parsed config.
pub fn compute(config: &ForjarConfig, input: &ScoringInput) -> ScoringResult {
    let raw = input.raw_yaml.as_deref().unwrap_or("");

    // Always compute static dimensions
    let saf = score_safety(config);
    let obs = score_observability(config);
    let doc = score_documentation(config, raw);
    let res = score_resilience(config);
    let cmp = score_composability(config);

    let static_dims = [&saf, &obs, &doc, &res, &cmp];
    let static_composite = compute_weighted(&static_dims, &[W_SAF, W_OBS, W_DOC, W_RES, W_CMP]);
    let static_min = static_dims.iter().map(|d| d.score).min().unwrap_or(0);
    let static_grade = determine_grade(static_composite, static_min);

    // Compute runtime dimensions if runtime data present
    let cor = score_correctness(input);
    let idm = score_idempotency(input);
    let prf = score_performance(input);

    let (runtime_composite, runtime_grade) = if input.runtime.is_some() {
        let rt_dims = [&cor, &idm, &prf];
        let rt_comp = compute_weighted(&rt_dims, &[W_COR, W_IDM, W_PRF]);
        let rt_min = rt_dims.iter().map(|d| d.score).min().unwrap_or(0);
        (Some(rt_comp), Some(determine_grade(rt_comp, rt_min)))
    } else {
        (None, None)
    };

    // Overall grade: min(static, runtime) or static-pending
    let grade = match runtime_grade {
        Some(rg) => {
            let _overall = grade_min(static_grade, rg);
            format!("{static_grade}/{rg}")
        }
        None => {
            if input.status == "blocked" {
                format!("{static_grade}/blocked")
            } else {
                format!("{static_grade}/pending")
            }
        }
    };

    // Hard-fail only on blocked status (v2: pending gets static grade)
    let (hard_fail, hard_fail_reason) = if input.status == "blocked" {
        (true, Some("status is blocked".to_string()))
    } else {
        (false, None)
    };

    // Legacy composite: blend all 8 dims with v1-compatible weighting
    let all_dims = vec![cor, idm, prf, saf, obs, doc, res, cmp];
    let legacy_composite = compute_composite(&all_dims);

    ScoringResult {
        dimensions: all_dims,
        static_composite,
        static_grade,
        runtime_composite,
        runtime_grade,
        grade,
        composite: legacy_composite,
        hard_fail,
        hard_fail_reason,
    }
}

/// Compute weighted composite from dimension scores and weights.
fn compute_weighted(dims: &[&DimensionScore], weights: &[u32]) -> u32 {
    let weighted: u64 = dims
        .iter()
        .zip(weights.iter())
        .map(|(d, w)| u64::from(d.score) * u64::from(*w))
        .sum();
    #[allow(clippy::cast_possible_truncation)]
    let result = (weighted / 100) as u32;
    result.min(100)
}

pub(crate) fn compute_composite(dims: &[DimensionScore]) -> u32 {
    let total_weight: f64 = dims.iter().map(|d| d.weight).sum();
    if total_weight == 0.0 {
        return 0;
    }
    let weighted: f64 = dims.iter().map(|d| d.score as f64 * d.weight).sum();
    (weighted / total_weight).round() as u32
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

fn grade_min(a: char, b: char) -> char {
    let ord = |g: char| match g {
        'A' => 4,
        'B' => 3,
        'C' => 2,
        'D' => 1,
        _ => 0,
    };
    let min_ord = ord(a).min(ord(b));
    match min_ord {
        4 => 'A',
        3 => 'B',
        2 => 'C',
        1 => 'D',
        _ => 'F',
    }
}

// ============================================================================
// Runtime dimension scorers (v2 weights)
// ============================================================================

pub(super) fn score_correctness(input: &ScoringInput) -> DimensionScore {
    let score = match &input.runtime {
        Some(rt) => {
            let mut s: i32 = 0;
            if rt.validate_pass {
                s += 15;
            }
            if rt.plan_pass {
                s += 15;
            }
            if rt.first_apply_pass {
                s += 40;
            }
            if rt.all_resources_converged {
                s += 15;
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
        weight: W_COR as f64 / 100.0,
    }
}

pub(super) fn score_idempotency(input: &ScoringInput) -> DimensionScore {
    let score = match &input.runtime {
        Some(rt) => {
            let mut s: i32 = 0;
            if rt.second_apply_pass {
                s += 25;
            }
            if rt.zero_changes_on_reapply {
                s += 25;
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
        None => 0,
    };
    DimensionScore {
        code: "IDM",
        name: "Idempotency",
        score,
        weight: W_IDM as f64 / 100.0,
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
        weight: W_PRF as f64 / 100.0,
    }
}

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

pub(super) fn perf_idempotent_points(idem_ms: u64) -> u32 {
    match idem_ms {
        0..=2000 => 30,
        2001..=5000 => 25,
        5001..=10000 => 15,
        _ => 0,
    }
}

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
