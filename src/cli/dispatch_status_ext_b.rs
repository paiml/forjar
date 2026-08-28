use super::dispatch_status_b::first_enabled_report;
use super::lock_ops::*;
use super::status_alerts::*;
use super::status_compliance::*;
use super::status_convergence::*;
use super::status_cost::*;
use super::status_drift::*;
use super::status_failures::*;
use super::status_fleet::*;
use super::status_health::*;
use super::status_observability::*;
use super::status_queries::*;
use super::status_resource_detail::*;
use super::status_resources::*;
use super::status_trends::*;
use std::path::Path;

/// Fleet-wide reports take no machine filter. These adapters give them the
/// uniform `(state_dir, machine, json)` report shape so they can sit in the
/// same dispatch table as the per-machine reports, with the ignored filter
/// spelled out rather than hidden.
fn drift_details_all_report(sd: &Path, _machine: Option<&str>, json: bool) -> Result<(), String> {
    cmd_status_drift_details_all(sd, json)
}

fn fleet_overview_report(sd: &Path, _machine: Option<&str>, json: bool) -> Result<(), String> {
    cmd_status_fleet_overview(sd, json)
}

fn executive_summary_report(sd: &Path, _machine: Option<&str>, json: bool) -> Result<(), String> {
    cmd_status_executive_summary(sd, json)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase58(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    resource_types_summary: bool,
    failed_resources: bool,
    drift_trend: bool,
    resource_inputs: bool,
    convergence_history: bool,
    config_hash: bool,
    last_apply_duration: bool,
    drift_details_all: bool,
    resource_size: bool,
    hash_verify: bool,
    lock_age: bool,
) -> Option<Result<(), String>> {
    first_enabled_report(
        sd,
        machine,
        json,
        &[
            (resource_types_summary, cmd_status_resource_types_summary),
            (failed_resources, cmd_status_failed_resources),
            (drift_trend, cmd_status_drift_trend),
            (resource_inputs, cmd_status_resource_inputs),
            (convergence_history, cmd_status_convergence_history),
            (config_hash, cmd_status_config_hash),
            (last_apply_duration, cmd_status_last_apply_duration),
            (drift_details_all, drift_details_all_report),
            (resource_size, cmd_status_resource_size),
            (hash_verify, cmd_status_hash_verify),
            (lock_age, cmd_status_lock_age),
        ],
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_analytics(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    change_frequency: bool,
    machine_summary: bool,
    recommendations: bool,
    uptime: bool,
    diagnostic: bool,
    resource_dependencies: bool,
    pipeline_status: bool,
    drift_forecast: bool,
    resource_cost: bool,
    security_posture: bool,
) -> Option<Result<(), String>> {
    first_enabled_report(
        sd,
        machine,
        json,
        &[
            (change_frequency, cmd_status_change_frequency),
            (machine_summary, cmd_status_machine_summary),
            (recommendations, cmd_status_recommendations),
            (uptime, cmd_status_uptime),
            (diagnostic, cmd_status_diagnostic),
            (resource_dependencies, cmd_status_resource_dependencies),
            (pipeline_status, cmd_status_pipeline_status),
            (drift_forecast, cmd_status_drift_forecast),
            (resource_cost, cmd_status_resource_cost),
            (security_posture, cmd_status_security_posture),
        ],
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_fleet(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    error_summary: bool,
    resource_timeline: bool,
    convergence_time: bool,
    config_drift: bool,
    machine_health: bool,
    fleet_overview: bool,
    drift_velocity: bool,
    resource_graph: bool,
    audit_trail: bool,
    executive_summary: bool,
) -> Option<Result<(), String>> {
    first_enabled_report(
        sd,
        machine,
        json,
        &[
            (error_summary, cmd_status_error_summary),
            (resource_timeline, cmd_status_resource_timeline),
            (convergence_time, cmd_status_convergence_time),
            (config_drift, cmd_status_config_drift),
            (machine_health, cmd_status_machine_health),
            (fleet_overview, fleet_overview_report),
            (drift_velocity, cmd_status_drift_velocity),
            (resource_graph, cmd_status_resource_graph),
            (audit_trail, cmd_status_audit_trail),
            (executive_summary, executive_summary_report),
        ],
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_reports(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    health_score: bool,
    staleness_report: &Option<String>,
    cost_estimate: bool,
    capacity: bool,
    prediction: bool,
    trend: Option<usize>,
    mttr: bool,
    compliance_report: &Option<String>,
    sla_report: bool,
    resource_age: bool,
    drift_summary: bool,
) -> Option<Result<(), String>> {
    // Candidates in declaration order; `or_else` keeps the first-match-wins
    // priority the `if` chain had, and evaluates nothing past that match.
    // Reports carrying a value cannot join the plain flag table, so this
    // dispatcher chains them instead.
    health_score
        .then(|| cmd_status_health_score(sd, machine, json))
        .or_else(|| {
            staleness_report
                .as_deref()
                .map(|w| cmd_status_staleness_report(sd, machine, w, json))
        })
        .or_else(|| cost_estimate.then(|| cmd_status_cost_estimate(sd, machine, json)))
        .or_else(|| capacity.then(|| cmd_status_capacity(sd, machine, json)))
        .or_else(|| prediction.then(|| cmd_status_prediction(sd, machine, json)))
        .or_else(|| trend.map(|n| cmd_status_trend(sd, machine, n, json)))
        .or_else(|| mttr.then(|| cmd_status_mttr(sd, machine, json)))
        .or_else(|| {
            compliance_report
                .as_deref()
                .map(|p| cmd_status_compliance_report(sd, machine, p, json))
        })
        .or_else(|| sla_report.then(|| cmd_status_sla_report(sd, machine, json)))
        .or_else(|| resource_age.then(|| cmd_status_resource_age(sd, machine, json)))
        .or_else(|| drift_summary.then(|| cmd_status_drift_summary(sd, machine, json)))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_queries_a(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    convergence_rate: bool,
    top_failures: bool,
    dependency_health: bool,
    histogram: bool,
    compliance: &Option<String>,
    diff_lock: &Option<std::path::PathBuf>,
    alerts: bool,
    compact: bool,
    export: &Option<std::path::PathBuf>,
    json_lines: bool,
) -> Option<Result<(), String>> {
    // Candidates in declaration order; `or_else` keeps first-match-wins and
    // evaluates nothing past the match. See `try_status_reports` for why the
    // value-carrying queries are chained rather than tabulated.
    convergence_rate
        .then(|| cmd_status_convergence_rate(sd, machine, json))
        .or_else(|| top_failures.then(|| cmd_status_top_failures(sd, machine, json)))
        .or_else(|| dependency_health.then(|| cmd_status_dependency_health(sd, machine, json)))
        .or_else(|| histogram.then(|| cmd_status_histogram(sd, machine, json)))
        .or_else(|| {
            compliance
                .as_deref()
                .map(|p| cmd_status_compliance(sd, machine, p, json))
        })
        .or_else(|| diff_lock.as_deref().map(|p| cmd_lock_diff(sd, p, json)))
        .or_else(|| alerts.then(|| cmd_status_alerts(sd, machine, json)))
        .or_else(|| compact.then(|| cmd_status_compact(sd, machine, json)))
        .or_else(|| {
            export
                .as_deref()
                .map(|p| cmd_status_export(sd, machine, p, json))
        })
        .or_else(|| json_lines.then(|| cmd_status_json_lines(sd, machine)))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_queries_b(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    since: &Option<String>,
    stale_resources: bool,
    health_threshold: Option<u32>,
    machines_only: bool,
    resources_by_type: bool,
    anomalies: bool,
    diff_from: &Option<String>,
    count: bool,
) -> Option<Result<(), String>> {
    if let Some(ref d) = since {
        return Some(cmd_status_since(sd, machine, d, json));
    }
    if stale_resources {
        return Some(cmd_status_stale_resources(sd, machine, json));
    }
    if let Some(t) = health_threshold {
        return Some(cmd_status_health_threshold(sd, machine, t, json));
    }
    if machines_only {
        return Some(cmd_status_machines_only(sd, machine, json));
    }
    if resources_by_type {
        return Some(cmd_status_resources_by_type(sd, machine, json));
    }
    if anomalies {
        return Some(cmd_status_anomalies(sd, machine, json));
    }
    if let Some(ref s) = diff_from {
        return Some(cmd_status_diff_from(sd, s, json));
    }
    if count {
        return Some(cmd_status_count(sd, machine, json));
    }
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_display(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    status_format: &Option<String>,
    prometheus: bool,
    expired: &Option<String>,
    changes_since: &Option<String>,
    summary_by: &Option<String>,
    timeline: bool,
    drift_details: bool,
    health: bool,
    stale: Option<u64>,
    failed_since: &Option<String>,
) -> Option<Result<(), String>> {
    // Candidates in declaration order; `or_else` keeps first-match-wins and
    // evaluates nothing past the match. See `try_status_reports` for why the
    // value-carrying displays are chained rather than tabulated.
    status_format
        .as_deref()
        .map(|f| cmd_status_format(sd, machine, f))
        .or_else(|| prometheus.then(|| cmd_status_prometheus(sd, machine)))
        .or_else(|| {
            expired
                .as_deref()
                .map(|d| cmd_status_expired(sd, machine, d, json))
        })
        .or_else(|| {
            changes_since
                .as_deref()
                .map(|c| cmd_status_changes_since(sd, c, json))
        })
        .or_else(|| {
            summary_by
                .as_deref()
                .map(|d| cmd_status_summary_by(sd, machine, d, json))
        })
        .or_else(|| timeline.then(|| cmd_status_timeline(sd, machine, json)))
        .or_else(|| drift_details.then(|| cmd_status_drift_details(sd, machine, json)))
        .or_else(|| health.then(|| cmd_status_health(sd, machine, json)))
        .or_else(|| stale.map(|days| cmd_status_stale(sd, machine, days, json)))
        .or_else(|| {
            failed_since
                .as_deref()
                .map(|s| cmd_status_failed_since(sd, machine, s, json))
        })
}
