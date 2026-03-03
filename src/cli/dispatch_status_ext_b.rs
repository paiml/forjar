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
    if resource_types_summary {
        return Some(cmd_status_resource_types_summary(sd, machine, json));
    }
    if failed_resources {
        return Some(cmd_status_failed_resources(sd, machine, json));
    }
    if drift_trend {
        return Some(cmd_status_drift_trend(sd, machine, json));
    }
    if resource_inputs {
        return Some(cmd_status_resource_inputs(sd, machine, json));
    }
    if convergence_history {
        return Some(cmd_status_convergence_history(sd, machine, json));
    }
    if config_hash {
        return Some(cmd_status_config_hash(sd, machine, json));
    }
    if last_apply_duration {
        return Some(cmd_status_last_apply_duration(sd, machine, json));
    }
    if drift_details_all {
        return Some(cmd_status_drift_details_all(sd, json));
    }
    if resource_size {
        return Some(cmd_status_resource_size(sd, machine, json));
    }
    if hash_verify {
        return Some(cmd_status_hash_verify(sd, machine, json));
    }
    if lock_age {
        return Some(cmd_status_lock_age(sd, machine, json));
    }
    None
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
    if change_frequency {
        return Some(cmd_status_change_frequency(sd, machine, json));
    }
    if machine_summary {
        return Some(cmd_status_machine_summary(sd, machine, json));
    }
    if recommendations {
        return Some(cmd_status_recommendations(sd, machine, json));
    }
    if uptime {
        return Some(cmd_status_uptime(sd, machine, json));
    }
    if diagnostic {
        return Some(cmd_status_diagnostic(sd, machine, json));
    }
    if resource_dependencies {
        return Some(cmd_status_resource_dependencies(sd, machine, json));
    }
    if pipeline_status {
        return Some(cmd_status_pipeline_status(sd, machine, json));
    }
    if drift_forecast {
        return Some(cmd_status_drift_forecast(sd, machine, json));
    }
    if resource_cost {
        return Some(cmd_status_resource_cost(sd, machine, json));
    }
    if security_posture {
        return Some(cmd_status_security_posture(sd, machine, json));
    }
    None
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
    if error_summary {
        return Some(cmd_status_error_summary(sd, machine, json));
    }
    if resource_timeline {
        return Some(cmd_status_resource_timeline(sd, machine, json));
    }
    if convergence_time {
        return Some(cmd_status_convergence_time(sd, machine, json));
    }
    if config_drift {
        return Some(cmd_status_config_drift(sd, machine, json));
    }
    if machine_health {
        return Some(cmd_status_machine_health(sd, machine, json));
    }
    if fleet_overview {
        return Some(cmd_status_fleet_overview(sd, json));
    }
    if drift_velocity {
        return Some(cmd_status_drift_velocity(sd, machine, json));
    }
    if resource_graph {
        return Some(cmd_status_resource_graph(sd, machine, json));
    }
    if audit_trail {
        return Some(cmd_status_audit_trail(sd, machine, json));
    }
    if executive_summary {
        return Some(cmd_status_executive_summary(sd, json));
    }
    None
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
    if health_score {
        return Some(cmd_status_health_score(sd, machine, json));
    }
    if let Some(ref w) = staleness_report {
        return Some(cmd_status_staleness_report(sd, machine, w, json));
    }
    if cost_estimate {
        return Some(cmd_status_cost_estimate(sd, machine, json));
    }
    if capacity {
        return Some(cmd_status_capacity(sd, machine, json));
    }
    if prediction {
        return Some(cmd_status_prediction(sd, machine, json));
    }
    if let Some(n) = trend {
        return Some(cmd_status_trend(sd, machine, n, json));
    }
    if mttr {
        return Some(cmd_status_mttr(sd, machine, json));
    }
    if let Some(ref p) = compliance_report {
        return Some(cmd_status_compliance_report(sd, machine, p, json));
    }
    if sla_report {
        return Some(cmd_status_sla_report(sd, machine, json));
    }
    if resource_age {
        return Some(cmd_status_resource_age(sd, machine, json));
    }
    if drift_summary {
        return Some(cmd_status_drift_summary(sd, machine, json));
    }
    None
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
    if convergence_rate {
        return Some(cmd_status_convergence_rate(sd, machine, json));
    }
    if top_failures {
        return Some(cmd_status_top_failures(sd, machine, json));
    }
    if dependency_health {
        return Some(cmd_status_dependency_health(sd, machine, json));
    }
    if histogram {
        return Some(cmd_status_histogram(sd, machine, json));
    }
    if let Some(ref p) = compliance {
        return Some(cmd_status_compliance(sd, machine, p, json));
    }
    if let Some(ref p) = diff_lock {
        return Some(cmd_lock_diff(sd, p, json));
    }
    if alerts {
        return Some(cmd_status_alerts(sd, machine, json));
    }
    if compact {
        return Some(cmd_status_compact(sd, machine, json));
    }
    if let Some(ref p) = export {
        return Some(cmd_status_export(sd, machine, p, json));
    }
    if json_lines {
        return Some(cmd_status_json_lines(sd, machine));
    }
    None
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
    if let Some(ref f) = status_format {
        return Some(cmd_status_format(sd, machine, f));
    }
    if prometheus {
        return Some(cmd_status_prometheus(sd, machine));
    }
    if let Some(ref d) = expired {
        return Some(cmd_status_expired(sd, machine, d, json));
    }
    if let Some(ref c) = changes_since {
        return Some(cmd_status_changes_since(sd, c, json));
    }
    if let Some(ref d) = summary_by {
        return Some(cmd_status_summary_by(sd, machine, d, json));
    }
    if timeline {
        return Some(cmd_status_timeline(sd, machine, json));
    }
    if drift_details {
        return Some(cmd_status_drift_details(sd, machine, json));
    }
    if health {
        return Some(cmd_status_health(sd, machine, json));
    }
    if let Some(days) = stale {
        return Some(cmd_status_stale(sd, machine, days, json));
    }
    if let Some(ref s) = failed_since {
        return Some(cmd_status_failed_since(sd, machine, s, json));
    }
    None
}
