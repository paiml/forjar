use super::scoring::*;
use super::types::{FailurePolicy, ForjarConfig, ResourceType};

pub(super) fn score_safety(config: &ForjarConfig) -> DimensionScore {
    let mut score: i32 = 100;
    let mut has_critical = false;
    for resource in config.resources.values() {
        let (deduction, critical) = safety_audit_resource(resource);
        score -= deduction;
        has_critical |= critical;
    }
    if has_critical && score > 40 {
        score = 40;
    }
    let score = score.clamp(0, 100) as u32;
    DimensionScore {
        code: "SAF",
        name: "Safety",
        score,
        weight: 0.15,
    }
}

/// Returns (deduction, is_critical) for a single resource.
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

    DimensionScore {
        code: "OBS",
        name: "Observability",
        score: score.min(100),
        weight: 0.10,
    }
}

pub(super) fn score_documentation(config: &ForjarConfig) -> DimensionScore {
    let mut score: u32 = 0;

    // Comment ratio — approximate from raw YAML content (we only have parsed config,
    // so check description fields as a proxy).
    // description field present: +15
    if config.description.is_some() {
        score += 15;
    }

    // Header metadata checks — we check name quality
    let name = &config.name;
    let is_generic = name == "unnamed" || name == "default" || name == "config" || name.is_empty();
    if !is_generic {
        score += 5;
    }

    // For static scoring we give partial credit for presence of documentation-like metadata.
    // Full comment-ratio scoring requires raw YAML analysis (done at runtime).
    // Award baseline for having a non-empty name and description.
    if config.description.as_ref().is_some_and(|d| !d.is_empty()) {
        score += 25; // Combined header/comment credit for having good description
    }

    DimensionScore {
        code: "DOC",
        name: "Documentation",
        score: score.min(100),
        weight: 0.08,
    }
}

pub(super) fn score_resilience(config: &ForjarConfig) -> DimensionScore {
    let mut score: u32 = 0;

    // failure policy (continue_independent): +20
    if config.policy.failure == FailurePolicy::ContinueIndependent {
        score += 20;
    }

    // ssh_retries > 1: +10
    if config.policy.ssh_retries > 1 {
        score += 10;
    }

    // Dependency DAG ratio
    let total = config.resources.len();
    if total > 0 {
        let with_deps = config
            .resources
            .values()
            .filter(|r| !r.depends_on.is_empty())
            .count();
        let ratio_pct = (with_deps * 100) / total;
        if ratio_pct >= 50 {
            score += 30;
        } else if ratio_pct >= 30 {
            score += 20;
        }
    }

    // pre_apply hook: +10
    if config.policy.pre_apply.is_some() {
        score += 10;
    }

    // post_apply hook: +10
    if config.policy.post_apply.is_some() {
        score += 10;
    }

    // Also check per-resource lifecycle hooks
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
        weight: 0.07,
    }
}

pub(super) fn score_composability(config: &ForjarConfig) -> DimensionScore {
    let mut score: u32 = 0;

    // params with defaults: +20
    if !config.params.is_empty() {
        score += 20;
    }

    // templates used (check for {{ in resource content): +10
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

    // recipe nesting: +15
    let has_recipes = config
        .resources
        .values()
        .any(|r| r.resource_type == ResourceType::Recipe);
    if has_recipes {
        score += 15;
    }

    DimensionScore {
        code: "CMP",
        name: "Composability",
        score: score.min(100),
        weight: 0.05,
    }
}

// ============================================================================
// Report formatting
// ============================================================================

/// Format a human-readable score report.
pub fn format_score_report(result: &ScoringResult) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "\nForjar Score: {} (Grade {})\n",
        result.composite, result.grade
    ));
    out.push_str(&format!("{}\n", "=".repeat(40)));

    if result.hard_fail {
        if let Some(ref reason) = result.hard_fail_reason {
            out.push_str(&format!("HARD FAIL: {}\n", reason));
        }
        return out;
    }

    for dim in &result.dimensions {
        let bar = score_bar(dim.score);
        out.push_str(&format!(
            "  {} {:14} {:>3}/100  {:.0}%w  {}\n",
            dim.code,
            dim.name,
            dim.score,
            dim.weight * 100.0,
            bar,
        ));
    }

    out.push_str(&format!("\n  Composite: {}/100\n", result.composite));
    out.push_str(&format!("  Grade:     {}\n", result.grade));

    out
}

pub fn score_bar(score: u32) -> String {
    let filled = (score / 5) as usize;
    let empty = 20_usize.saturating_sub(filled);
    format!("[{}{}]", "#".repeat(filled), ".".repeat(empty))
}
