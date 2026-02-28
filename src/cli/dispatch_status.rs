//! Status command dispatch — routes status sub-flags to handlers.

#[allow(unused_imports)]
use crate::core::{state, types};
use std::path::Path;
use super::commands::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::status_core::*;
use super::status_queries::*;
use super::status_health::*;
use super::status_alerts::*;
use super::status_drift::*;
use super::status_convergence::*;
use super::status_trends::*;
use super::status_fleet::*;
use super::status_resources::*;
use super::status_resource_detail::*;
use super::status_counts::*;
use super::status_diagnostics::*;
use super::status_fleet_detail::*;
use super::status_compliance::*;
use super::status_cost::*;
use super::status_observability::*;
use super::status_failures::*;
use super::status_operational::*;
use super::status_insights::*;
use super::lock_ops::*;


/// Phase 58+ status flags (resource_types_summary through lock_age).
fn try_status_phase58(
    sd: &Path, machine: Option<&str>, json: bool,
    resource_types_summary: bool, failed_resources: bool,
    drift_trend: bool, resource_inputs: bool,
    convergence_history: bool, config_hash: bool,
    last_apply_duration: bool, drift_details_all: bool,
    resource_size: bool, hash_verify: bool,
    lock_age: bool,
) -> Option<Result<(), String>> {
    if resource_types_summary { return Some(cmd_status_resource_types_summary(sd, machine, json)); }
    if failed_resources { return Some(cmd_status_failed_resources(sd, machine, json)); }
    if drift_trend { return Some(cmd_status_drift_trend(sd, machine, json)); }
    if resource_inputs { return Some(cmd_status_resource_inputs(sd, machine, json)); }
    if convergence_history { return Some(cmd_status_convergence_history(sd, machine, json)); }
    if config_hash { return Some(cmd_status_config_hash(sd, machine, json)); }
    if last_apply_duration { return Some(cmd_status_last_apply_duration(sd, machine, json)); }
    if drift_details_all { return Some(cmd_status_drift_details_all(sd, json)); }
    if resource_size { return Some(cmd_status_resource_size(sd, machine, json)); }
    if hash_verify { return Some(cmd_status_hash_verify(sd, machine, json)); }
    if lock_age { return Some(cmd_status_lock_age(sd, machine, json)); }
    None
}

/// Status flags: change_frequency through security_posture.
fn try_status_analytics(
    sd: &Path, machine: Option<&str>, json: bool,
    change_frequency: bool, machine_summary: bool,
    recommendations: bool, uptime: bool,
    diagnostic: bool, resource_dependencies: bool,
    pipeline_status: bool, drift_forecast: bool,
    resource_cost: bool, security_posture: bool,
) -> Option<Result<(), String>> {
    if change_frequency { return Some(cmd_status_change_frequency(sd, machine, json)); }
    if machine_summary { return Some(cmd_status_machine_summary(sd, machine, json)); }
    if recommendations { return Some(cmd_status_recommendations(sd, machine, json)); }
    if uptime { return Some(cmd_status_uptime(sd, machine, json)); }
    if diagnostic { return Some(cmd_status_diagnostic(sd, machine, json)); }
    if resource_dependencies { return Some(cmd_status_resource_dependencies(sd, machine, json)); }
    if pipeline_status { return Some(cmd_status_pipeline_status(sd, machine, json)); }
    if drift_forecast { return Some(cmd_status_drift_forecast(sd, machine, json)); }
    if resource_cost { return Some(cmd_status_resource_cost(sd, machine, json)); }
    if security_posture { return Some(cmd_status_security_posture(sd, machine, json)); }
    None
}

/// Status flags: error_summary through executive_summary.
fn try_status_fleet(
    sd: &Path, machine: Option<&str>, json: bool,
    error_summary: bool, resource_timeline: bool,
    convergence_time: bool, config_drift: bool,
    machine_health: bool, fleet_overview: bool,
    drift_velocity: bool, resource_graph: bool,
    audit_trail: bool, executive_summary: bool,
) -> Option<Result<(), String>> {
    if error_summary { return Some(cmd_status_error_summary(sd, machine, json)); }
    if resource_timeline { return Some(cmd_status_resource_timeline(sd, machine, json)); }
    if convergence_time { return Some(cmd_status_convergence_time(sd, machine, json)); }
    if config_drift { return Some(cmd_status_config_drift(sd, machine, json)); }
    if machine_health { return Some(cmd_status_machine_health(sd, machine, json)); }
    if fleet_overview { return Some(cmd_status_fleet_overview(sd, json)); }
    if drift_velocity { return Some(cmd_status_drift_velocity(sd, machine, json)); }
    if resource_graph { return Some(cmd_status_resource_graph(sd, machine, json)); }
    if audit_trail { return Some(cmd_status_audit_trail(sd, machine, json)); }
    if executive_summary { return Some(cmd_status_executive_summary(sd, json)); }
    None
}

/// Status flags: health_score through drift_summary.
fn try_status_reports(
    sd: &Path, machine: Option<&str>, json: bool,
    health_score: bool, staleness_report: &Option<String>,
    cost_estimate: bool, capacity: bool,
    prediction: bool, trend: Option<usize>,
    mttr: bool, compliance_report: &Option<String>,
    sla_report: bool, resource_age: bool,
    drift_summary: bool,
) -> Option<Result<(), String>> {
    if health_score { return Some(cmd_status_health_score(sd, machine, json)); }
    if let Some(ref w) = staleness_report { return Some(cmd_status_staleness_report(sd, machine, w, json)); }
    if cost_estimate { return Some(cmd_status_cost_estimate(sd, machine, json)); }
    if capacity { return Some(cmd_status_capacity(sd, machine, json)); }
    if prediction { return Some(cmd_status_prediction(sd, machine, json)); }
    if let Some(n) = trend { return Some(cmd_status_trend(sd, machine, n, json)); }
    if mttr { return Some(cmd_status_mttr(sd, machine, json)); }
    if let Some(ref p) = compliance_report { return Some(cmd_status_compliance_report(sd, machine, p, json)); }
    if sla_report { return Some(cmd_status_sla_report(sd, machine, json)); }
    if resource_age { return Some(cmd_status_resource_age(sd, machine, json)); }
    if drift_summary { return Some(cmd_status_drift_summary(sd, machine, json)); }
    None
}

/// Status flags: convergence_rate through json_lines.
fn try_status_queries_a(
    sd: &Path, machine: Option<&str>, json: bool,
    convergence_rate: bool, top_failures: bool,
    dependency_health: bool, histogram: bool,
    compliance: &Option<String>, diff_lock: &Option<std::path::PathBuf>,
    alerts: bool, compact: bool,
    export: &Option<std::path::PathBuf>, json_lines: bool,
) -> Option<Result<(), String>> {
    if convergence_rate { return Some(cmd_status_convergence_rate(sd, machine, json)); }
    if top_failures { return Some(cmd_status_top_failures(sd, machine, json)); }
    if dependency_health { return Some(cmd_status_dependency_health(sd, machine, json)); }
    if histogram { return Some(cmd_status_histogram(sd, machine, json)); }
    if let Some(ref p) = compliance { return Some(cmd_status_compliance(sd, machine, p, json)); }
    if let Some(ref p) = diff_lock { return Some(cmd_lock_diff(sd, p, json)); }
    if alerts { return Some(cmd_status_alerts(sd, machine, json)); }
    if compact { return Some(cmd_status_compact(sd, machine, json)); }
    if let Some(ref p) = export { return Some(cmd_status_export(sd, machine, p, json)); }
    if json_lines { return Some(cmd_status_json_lines(sd, machine)); }
    None
}

/// Status flags: since through count.
fn try_status_queries_b(
    sd: &Path, machine: Option<&str>, json: bool,
    since: &Option<String>, stale_resources: bool,
    health_threshold: Option<u32>,
    machines_only: bool, resources_by_type: bool,
    anomalies: bool, diff_from: &Option<String>,
    count: bool,
) -> Option<Result<(), String>> {
    if let Some(ref d) = since { return Some(cmd_status_since(sd, machine, d, json)); }
    if stale_resources { return Some(cmd_status_stale_resources(sd, machine, json)); }
    if let Some(t) = health_threshold { return Some(cmd_status_health_threshold(sd, machine, t, json)); }
    if machines_only { return Some(cmd_status_machines_only(sd, machine, json)); }
    if resources_by_type { return Some(cmd_status_resources_by_type(sd, machine, json)); }
    if anomalies { return Some(cmd_status_anomalies(sd, machine, json)); }
    if let Some(ref s) = diff_from { return Some(cmd_status_diff_from(sd, s, json)); }
    if count { return Some(cmd_status_count(sd, machine, json)); }
    None
}

/// Status flags: format through watch/default.
fn try_status_display(
    sd: &Path, machine: Option<&str>, json: bool,
    status_format: &Option<String>, prometheus: bool,
    expired: &Option<String>, changes_since: &Option<String>,
    summary_by: &Option<String>, timeline: bool,
    drift_details: bool, health: bool,
    stale: Option<u64>, failed_since: &Option<String>,
) -> Option<Result<(), String>> {
    if let Some(ref f) = status_format { return Some(cmd_status_format(sd, machine, f)); }
    if prometheus { return Some(cmd_status_prometheus(sd, machine)); }
    if let Some(ref d) = expired { return Some(cmd_status_expired(sd, machine, d, json)); }
    if let Some(ref c) = changes_since { return Some(cmd_status_changes_since(sd, c, json)); }
    if let Some(ref d) = summary_by { return Some(cmd_status_summary_by(sd, machine, d, json)); }
    if timeline { return Some(cmd_status_timeline(sd, machine, json)); }
    if drift_details { return Some(cmd_status_drift_details(sd, machine, json)); }
    if health { return Some(cmd_status_health(sd, machine, json)); }
    if let Some(days) = stale { return Some(cmd_status_stale(sd, machine, days, json)); }
    if let Some(ref s) = failed_since { return Some(cmd_status_failed_since(sd, machine, s, json)); }
    None
}


/// Phase 59-61 status flags.
#[allow(clippy::too_many_arguments)]
fn try_status_phase59a(
    sd: &Path, machine: Option<&str>, json: bool,
    resource_health: bool, machine_health_summary: bool,
    last_apply_status: bool, resource_staleness: bool,
    convergence_percentage: bool, failed_count: bool, drift_count: bool,
    resource_duration: bool,
) -> Option<Result<(), String>> {
    if resource_health { return Some(cmd_status_resource_health(sd, machine, json)); }
    if machine_health_summary { return Some(cmd_status_machine_health_summary(sd, machine, json)); }
    if last_apply_status { return Some(cmd_status_last_apply_status(sd, machine, json)); }
    if resource_staleness { return Some(cmd_status_resource_staleness(sd, machine, json)); }
    if convergence_percentage { return Some(cmd_status_convergence_percentage(sd, machine, json)); }
    if failed_count { return Some(cmd_status_failed_count(sd, machine, json)); }
    if drift_count { return Some(cmd_status_drift_count(sd, machine, json)); }
    if resource_duration { return Some(cmd_status_resource_duration(sd, machine, json)); }
    None
}

/// Phase 62-64 status flags.
#[allow(clippy::too_many_arguments)]
fn try_status_phase62(
    sd: &Path, machine: Option<&str>, json: bool, file: Option<&Path>,
    machine_resource_map: bool, fleet_convergence: bool,
    resource_hash: bool, machine_drift_summary: bool,
    apply_history_count: bool, lock_file_count: bool,
    resource_type_distribution: bool,
) -> Option<Result<(), String>> {
    if machine_resource_map {
        let f = file.unwrap_or(std::path::Path::new("forjar.yaml"));
        return Some(cmd_status_machine_resource_map(f, json));
    }
    if fleet_convergence { return Some(cmd_status_fleet_convergence(sd, json)); }
    if resource_hash { return Some(cmd_status_resource_hash(sd, machine, json)); }
    if machine_drift_summary { return Some(cmd_status_machine_drift_summary(sd, machine, json)); }
    if apply_history_count { return Some(cmd_status_apply_history_count(sd, machine, json)); }
    if lock_file_count { return Some(cmd_status_lock_file_count(sd, json)); }
    if resource_type_distribution {
        let f = file.unwrap_or(std::path::Path::new("forjar.yaml"));
        return Some(cmd_status_resource_type_distribution(f, json));
    }
    None
}

/// Phase 65-67 status flags.
#[allow(clippy::too_many_arguments)]
fn try_status_phase65(
    sd: &Path, machine: Option<&str>, json: bool, file: Option<&Path>,
    resource_apply_age: bool, machine_uptime: bool, resource_churn: bool,
    last_drift_time: bool, machine_resource_count: bool, convergence_score: bool,
    apply_success_rate: bool, error_rate: bool, fleet_health_summary: bool,
) -> Option<Result<(), String>> {
    if resource_apply_age { return Some(cmd_status_resource_apply_age(sd, machine, json)); }
    if machine_uptime { return Some(cmd_status_machine_uptime(sd, machine, json)); }
    if resource_churn { return Some(cmd_status_resource_churn(sd, machine, json)); }
    if last_drift_time { return Some(cmd_status_last_drift_time(sd, machine, json)); }
    if machine_resource_count {
        let f = file.unwrap_or(std::path::Path::new("forjar.yaml"));
        return Some(cmd_status_machine_resource_count(f, json));
    }
    if convergence_score { return Some(cmd_status_convergence_score(sd, json)); }
    if apply_success_rate { return Some(cmd_status_apply_success_rate(sd, machine, json)); }
    if error_rate { return Some(cmd_status_error_rate(sd, machine, json)); }
    if fleet_health_summary { return Some(cmd_status_fleet_health_summary(sd, json)); }
    None
}


/// Phase 68-70 operational status flags.
#[allow(clippy::too_many_arguments)]
fn try_status_phase68(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_convergence_history: bool, drift_history: bool, resource_failure_rate: bool,
    machine_last_apply: bool, fleet_drift_summary: bool, resource_apply_duration: bool,
    machine_resource_health: bool, fleet_convergence_trend: bool, resource_state_distribution: bool,
) -> Option<Result<(), String>> {
    if machine_convergence_history { return Some(cmd_status_machine_convergence_history(sd, machine, json)); }
    if drift_history { return Some(cmd_status_drift_history(sd, machine, json)); }
    if resource_failure_rate { return Some(cmd_status_resource_failure_rate(sd, machine, json)); }
    if machine_last_apply { return Some(cmd_status_machine_last_apply(sd, machine, json)); }
    if fleet_drift_summary { return Some(cmd_status_fleet_drift_summary(sd, machine, json)); }
    if resource_apply_duration { return Some(cmd_status_resource_apply_duration(sd, machine, json)); }
    if machine_resource_health { return Some(cmd_status_machine_resource_health(sd, machine, json)); }
    if fleet_convergence_trend { return Some(cmd_status_fleet_convergence_trend(sd, machine, json)); }
    if resource_state_distribution { return Some(cmd_status_resource_state_distribution(sd, machine, json)); }
    None
}

/// Phase 71-72 operational status flags.
#[allow(clippy::too_many_arguments)]
fn try_status_phase71(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_apply_count: bool, fleet_apply_history: bool, resource_hash_changes: bool,
    machine_uptime_estimate: bool, fleet_resource_type_breakdown: bool, resource_convergence_time: bool,
) -> Option<Result<(), String>> {
    if machine_apply_count { return Some(cmd_status_machine_apply_count(sd, machine, json)); }
    if fleet_apply_history { return Some(cmd_status_fleet_apply_history(sd, machine, json)); }
    if resource_hash_changes { return Some(cmd_status_resource_hash_changes(sd, machine, json)); }
    if machine_uptime_estimate { return Some(cmd_status_machine_uptime_estimate(sd, machine, json)); }
    if fleet_resource_type_breakdown { return Some(cmd_status_fleet_resource_type_breakdown(sd, machine, json)); }
    if resource_convergence_time { return Some(cmd_status_resource_convergence_time(sd, machine, json)); }
    None
}

/// Dispatch the Status command variant.
pub(crate) fn dispatch_status_cmd(cmd: Commands) -> Result<(), String> {
    let Commands::Status(StatusArgs {
        state_dir, machine, json, file, summary, watch,
        stale, health, drift_details, timeline, changes_since,
        summary_by, prometheus, expired, count,
        format: status_format, anomalies, diff_from,
        resources_by_type, machines_only, stale_resources,
        health_threshold, json_lines, since, export, compact,
        alerts, diff_lock, compliance, histogram, dependency_health,
        top_failures, convergence_rate, drift_summary, resource_age,
        sla_report, compliance_report, mttr, trend, prediction,
        capacity, cost_estimate, staleness_report, health_score,
        executive_summary, audit_trail, resource_graph, drift_velocity,
        fleet_overview, machine_health, config_drift, convergence_time,
        resource_timeline, error_summary, security_posture, resource_cost,
        drift_forecast, pipeline_status, resource_dependencies, diagnostic,
        uptime, recommendations, machine_summary, change_frequency,
        lock_age, failed_since, hash_verify, resource_size,
        drift_details_all, last_apply_duration, config_hash,
        convergence_history, resource_inputs, drift_trend,
        failed_resources, resource_types_summary,
        resource_health, machine_health_summary,
        dependency_count: _dependency_count, last_apply_status, resource_staleness,
        convergence_percentage, failed_count, drift_count,
        resource_duration, machine_resource_map,
        fleet_convergence, resource_hash, machine_drift_summary,
        apply_history_count, lock_file_count, resource_type_distribution,
        resource_apply_age, machine_uptime, resource_churn,
        last_drift_time, machine_resource_count, convergence_score,
        apply_success_rate, error_rate, fleet_health_summary,
        machine_convergence_history, drift_history, resource_failure_rate,
        machine_last_apply, fleet_drift_summary, resource_apply_duration,
        machine_resource_health, fleet_convergence_trend, resource_state_distribution,
        machine_apply_count, fleet_apply_history, resource_hash_changes,
        machine_uptime_estimate, fleet_resource_type_breakdown, resource_convergence_time,
        machine_drift_age, fleet_failed_resources, resource_dependency_health,
    }) = cmd
    else {
        unreachable!()
    };

    let m = machine.as_deref();

    if let Some(r) = try_status_phase59a(&state_dir, m, json, resource_health, machine_health_summary, last_apply_status, resource_staleness, convergence_percentage, failed_count, drift_count, resource_duration) {
        return r;
    }
    if let Some(r) = try_status_phase62(&state_dir, m, json, file.as_deref(), machine_resource_map, fleet_convergence, resource_hash, machine_drift_summary, apply_history_count, lock_file_count, resource_type_distribution) {
        return r;
    }
    if let Some(r) = try_status_phase65(&state_dir, m, json, file.as_deref(), resource_apply_age, machine_uptime, resource_churn, last_drift_time, machine_resource_count, convergence_score, apply_success_rate, error_rate, fleet_health_summary) {
        return r;
    }
    if let Some(r) = try_status_phase68(&state_dir, m, json, machine_convergence_history, drift_history, resource_failure_rate, machine_last_apply, fleet_drift_summary, resource_apply_duration, machine_resource_health, fleet_convergence_trend, resource_state_distribution) {
        return r;
    }
    if machine_drift_age { return cmd_status_machine_drift_age(&state_dir, m, json); }
    if fleet_failed_resources { return cmd_status_fleet_failed_resources(&state_dir, m, json); }
    if resource_dependency_health { return cmd_status_resource_dependency_health(&state_dir, m, json); }
    if let Some(r) = try_status_phase71(&state_dir, m, json, machine_apply_count, fleet_apply_history, resource_hash_changes, machine_uptime_estimate, fleet_resource_type_breakdown, resource_convergence_time) {
        return r;
    }
    if let Some(r) = try_status_phase58(&state_dir, m, json, resource_types_summary, failed_resources, drift_trend, resource_inputs, convergence_history, config_hash, last_apply_duration, drift_details_all, resource_size, hash_verify, lock_age) {
        return r;
    }
    if let Some(r) = try_status_analytics(&state_dir, m, json, change_frequency, machine_summary, recommendations, uptime, diagnostic, resource_dependencies, pipeline_status, drift_forecast, resource_cost, security_posture) {
        return r;
    }
    if let Some(r) = try_status_fleet(&state_dir, m, json, error_summary, resource_timeline, convergence_time, config_drift, machine_health, fleet_overview, drift_velocity, resource_graph, audit_trail, executive_summary) {
        return r;
    }
    if let Some(r) = try_status_reports(&state_dir, m, json, health_score, &staleness_report, cost_estimate, capacity, prediction, trend, mttr, &compliance_report, sla_report, resource_age, drift_summary) {
        return r;
    }
    if let Some(r) = try_status_queries_a(&state_dir, m, json, convergence_rate, top_failures, dependency_health, histogram, &compliance, &diff_lock, alerts, compact, &export, json_lines) {
        return r;
    }
    if let Some(r) = try_status_queries_b(&state_dir, m, json, &since, stale_resources, health_threshold, machines_only, resources_by_type, anomalies, &diff_from, count) {
        return r;
    }
    if let Some(r) = try_status_display(&state_dir, m, json, &status_format, prometheus, &expired, &changes_since, &summary_by, timeline, drift_details, health, stale, &failed_since) {
        return r;
    }

    if let Some(interval) = watch {
        let interval = interval.max(1);
        loop {
            print!("\x1b[2J\x1b[H");
            cmd_status(&state_dir, m, json, file.as_deref(), summary)?;
            println!("\n{}", dim(&format!("Refreshing every {}s (Ctrl+C to stop)", interval)));
            std::thread::sleep(std::time::Duration::from_secs(interval));
        }
    } else {
        cmd_status(&state_dir, m, json, file.as_deref(), summary)
    }
}
