//! Static dimension scorers (v2) and report formatting.

use super::scoring::*;
use super::types::{FailurePolicy, ForjarConfig, ResourceType};
use std::collections::HashSet;

// ============================================================================
// SAF — Safety (25%). Starts at 100, deductions applied.
// ============================================================================

pub(super) fn score_safety(config: &ForjarConfig) -> DimensionScore {
    let mut score: i32 = 100;
    let mut has_critical = false;
    for resource in config.resources.values() {
        let (deduction, critical) = safety_audit_resource(resource);
        score -= deduction;
        has_critical |= critical;
    }
    // v2: plaintext secrets penalty
    score -= safety_plaintext_secrets_penalty(config);

    if has_critical && score > 40 {
        score = 40;
    }
    let score = score.clamp(0, 100) as u32;
    DimensionScore {
        code: "SAF",
        name: "Safety",
        score,
        weight: 0.25,
    }
}

fn safety_audit_resource(resource: &super::types::Resource) -> (i32, bool) {
    let mut deduction: i32 = 0;
    let mut critical = false;
    if let Some(ref mode) = resource.mode {
        if mode == "0777" || mode == "777" {
            deduction += 30;
            critical = true;
        }
    }
    if let Some(ref content) = resource.content {
        if content.contains("curl") && content.contains("bash") {
            deduction += 30;
            critical = true;
        }
    }
    if resource.resource_type == ResourceType::File && resource.mode.is_none() {
        deduction += 5;
    }
    if resource.resource_type == ResourceType::File && resource.owner.is_none() {
        deduction += 3;
    }
    if resource.resource_type == ResourceType::Package && resource.version.is_none() {
        deduction += 3;
    }
    (deduction, critical)
}

/// v2: -10 per param whose name matches secret patterns with non-template value.
fn safety_plaintext_secrets_penalty(config: &ForjarConfig) -> i32 {
    let secret_patterns = ["password", "token", "secret", "key", "api_key"];
    let mut penalty: i32 = 0;
    for (name, value) in &config.params {
        let lower = name.to_lowercase();
        if secret_patterns.iter().any(|p| lower.contains(p)) {
            let val_str = value.as_str().unwrap_or("");
            if !val_str.contains("{{") {
                penalty += 10;
            }
        }
    }
    penalty
}

// ============================================================================
// OBS — Observability (20%).
// ============================================================================

pub(super) fn score_observability(config: &ForjarConfig) -> DimensionScore {
    let mut score: u32 = 0;

    if config.policy.tripwire {
        score += 15;
    }
    if config.policy.lock_file {
        score += 15;
    }
    if !config.outputs.is_empty() {
        score += 10;
    }

    // File mode coverage
    let file_count = config
        .resources
        .values()
        .filter(|r| r.resource_type == ResourceType::File)
        .count();
    if file_count > 0 {
        let mode_count = config
            .resources
            .values()
            .filter(|r| r.resource_type == ResourceType::File && r.mode.is_some())
            .count();
        let mode_pct = (mode_count * 100) / file_count;
        score += ((mode_pct as u32) * 15) / 100;

        let owner_count = config
            .resources
            .values()
            .filter(|r| r.resource_type == ResourceType::File && r.owner.is_some())
            .count();
        let owner_pct = (owner_count * 100) / file_count;
        score += ((owner_pct as u32) * 15) / 100;
    }

    // Notify hooks (up to 20 pts)
    let notify = &config.policy.notify;
    let mut notify_pts: u32 = 0;
    if notify.on_success.is_some() {
        notify_pts += 7;
    }
    if notify.on_failure.is_some() {
        notify_pts += 7;
    }
    if notify.on_drift.is_some() {
        notify_pts += 6;
    }
    score += notify_pts;

    // v2: output descriptions present (+10)
    let has_output_descriptions = config.outputs.values().any(|o| o.description.is_some());
    if has_output_descriptions {
        score += 10;
    }

    DimensionScore {
        code: "OBS",
        name: "Observability",
        score: score.min(100),
        weight: 0.20,
    }
}

// ============================================================================
// DOC — Documentation (15%). v2: quality signals, not volume.
// ============================================================================

pub(super) fn score_documentation(config: &ForjarConfig, raw_yaml: &str) -> DimensionScore {
    let mut score: u32 = 0;

    // Header metadata checks from first 5 lines
    let first_lines: String = raw_yaml.lines().take(5).collect::<Vec<_>>().join("\n");
    if first_lines.contains("Recipe") {
        score += 8;
    }
    if first_lines.contains("Tier") {
        score += 8;
    }
    if first_lines.contains("Idempotency") || first_lines.contains("idempotency") {
        score += 8;
    }
    if first_lines.contains("Budget") || first_lines.contains("budget") {
        score += 8;
    }

    // description field present: +15
    if config.description.is_some() {
        score += 15;
    }

    // Name is kebab-case (contains dash): +3
    if !config.name.is_empty() && config.name.contains('-') {
        score += 3;
    }

    // v2: unique inline comments (≥3 distinct): +15
    let unique_comments: HashSet<&str> = raw_yaml
        .lines()
        .filter_map(|l| {
            let trimmed = l.trim();
            if trimmed.starts_with('#') {
                Some(trimmed)
            } else {
                None
            }
        })
        .collect();
    if unique_comments.len() >= 3 {
        score += 15;
    }

    // v2: output descriptions (≥50% have descriptions): +10
    if !config.outputs.is_empty() {
        let with_desc = config
            .outputs
            .values()
            .filter(|o| o.description.is_some())
            .count();
        let ratio = (with_desc * 100) / config.outputs.len();
        if ratio >= 50 {
            score += 10;
        }
    }

    // v2: param documentation (≥3 params with non-empty values): +10
    let documented_params = config
        .params
        .values()
        .filter(|v| v.as_str().is_some_and(|s| !s.is_empty()))
        .count();
    if documented_params >= 3 {
        score += 10;
    }

    DimensionScore {
        code: "DOC",
        name: "Documentation",
        score: score.min(100),
        weight: 0.15,
    }
}

// ============================================================================
// RES — Resilience (20%). v2: context-aware, no DAG bias.
// ============================================================================

pub(super) fn score_resilience(config: &ForjarConfig) -> DimensionScore {
    let mut score: u32 = 0;

    // failure policy (continue_independent): +15
    if config.policy.failure == FailurePolicy::ContinueIndependent {
        score += 15;
    }

    // ssh_retries > 1: +10
    if config.policy.ssh_retries > 1 {
        score += 10;
    }

    // v2: EITHER deep DAG OR tagged independence scores +20 (not both required)
    let total = config.resources.len();
    if total > 0 {
        let with_deps = config
            .resources
            .values()
            .filter(|r| !r.depends_on.is_empty())
            .count();
        let dep_ratio = (with_deps * 100) / total;

        let tagged_independent = config
            .resources
            .values()
            .filter(|r| !r.tags.is_empty() && r.resource_group.is_some())
            .count();
        let tag_ratio = (tagged_independent * 100) / total;

        if dep_ratio >= 50 || tag_ratio >= 50 {
            score += 20;
        } else if dep_ratio >= 30 || tag_ratio >= 30 {
            score += 10;
        }
    }

    // pre_apply hook: +8
    if config.policy.pre_apply.is_some() {
        score += 8;
    }

    // post_apply hook: +8
    if config.policy.post_apply.is_some() {
        score += 8;
    }

    // v2: deny_paths present: +10
    if !config.policy.deny_paths.is_empty() {
        score += 10;
    }

    // v2: multi-machine with parallel_machines: +5
    if config.machines.len() > 1 && config.policy.parallel_machines {
        score += 5;
    }

    // Per-resource lifecycle hooks: +10
    let has_resource_hooks = config
        .resources
        .values()
        .any(|r| r.pre_apply.is_some() || r.post_apply.is_some());
    if has_resource_hooks {
        score += 10;
    }

    DimensionScore {
        code: "RES",
        name: "Resilience",
        score: score.min(100),
        weight: 0.20,
    }
}

// ============================================================================
// CMP — Composability (20%).
// ============================================================================

pub(super) fn score_composability(config: &ForjarConfig) -> DimensionScore {
    let mut score: u32 = 0;

    // params: +15
    if !config.params.is_empty() {
        score += 15;
    }

    // templates used: +10
    let has_templates = config.resources.values().any(|r| {
        r.content.as_ref().is_some_and(|c| c.contains("{{"))
            || r.path.as_ref().is_some_and(|p| p.contains("{{"))
    });
    if has_templates {
        score += 10;
    }

    // includes: +10
    if !config.includes.is_empty() {
        score += 10;
    }

    // tags on resources: +15
    let has_tags = config.resources.values().any(|r| !r.tags.is_empty());
    if has_tags {
        score += 15;
    }

    // resource_groups: +15
    let has_groups = config
        .resources
        .values()
        .any(|r| r.resource_group.is_some());
    if has_groups {
        score += 15;
    }

    // multi-machine: +10
    let has_multi = config.machines.len() > 1
        || config.resources.values().any(
            |r| matches!(&r.machine, crate::core::types::MachineTarget::Multiple(v) if v.len() > 1),
        );
    if has_multi {
        score += 10;
    }

    // recipe nesting: +10
    let has_recipes = config
        .resources
        .values()
        .any(|r| r.resource_type == ResourceType::Recipe);
    if has_recipes {
        score += 10;
    }

    // v2: secrets via {{ secrets.* }} template: +5
    let has_secrets_template = config.resources.values().any(|r| {
        r.content
            .as_ref()
            .is_some_and(|c| c.contains("{{ secrets.") || c.contains("{{secrets."))
    });
    if has_secrets_template {
        score += 5;
    }

    DimensionScore {
        code: "CMP",
        name: "Composability",
        score: score.min(100),
        weight: 0.20,
    }
}

// ============================================================================
// Report formatting
// ============================================================================

/// Format a human-readable score report.
pub fn format_score_report(result: &ScoringResult) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "\nForjar Score v2: {} (Grade {})\n",
        result.composite, result.grade
    ));
    out.push_str(&format!("{}\n", "=".repeat(50)));

    if result.hard_fail {
        if let Some(ref reason) = result.hard_fail_reason {
            out.push_str(&format!("HARD FAIL: {reason}\n"));
        }
    }

    // Static dimensions
    out.push_str("\n  Static Grade: ");
    out.push(result.static_grade);
    out.push_str(&format!(" (composite {})\n", result.static_composite));
    for dim in &result.dimensions {
        if matches!(dim.code, "SAF" | "OBS" | "DOC" | "RES" | "CMP") {
            let bar = score_bar(dim.score);
            out.push_str(&format!(
                "    {} {:14} {:>3}/100  {:.0}%w  {}\n",
                dim.code,
                dim.name,
                dim.score,
                dim.weight * 100.0,
                bar,
            ));
        }
    }

    // Runtime dimensions
    match result.runtime_grade {
        Some(rg) => {
            out.push_str(&format!(
                "\n  Runtime Grade: {} (composite {})\n",
                rg,
                result.runtime_composite.unwrap_or(0)
            ));
            for dim in &result.dimensions {
                if matches!(dim.code, "COR" | "IDM" | "PRF") {
                    let bar = score_bar(dim.score);
                    out.push_str(&format!(
                        "    {} {:14} {:>3}/100  {:.0}%w  {}\n",
                        dim.code,
                        dim.name,
                        dim.score,
                        dim.weight * 100.0,
                        bar,
                    ));
                }
            }
        }
        None => {
            out.push_str("\n  Runtime Grade: pending (no runtime data)\n");
        }
    }

    out.push_str(&format!("\n  Overall: {}\n", result.grade));

    out
}

/// Render a 20-char ASCII bar for a score (0-100).
pub fn score_bar(score: u32) -> String {
    let filled = (score / 5) as usize;
    let empty = 20_usize.saturating_sub(filled);
    format!("[{}{}]", "#".repeat(filled), ".".repeat(empty))
}
