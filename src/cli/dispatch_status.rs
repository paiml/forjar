//! Status command dispatch — routes status sub-flags to handlers.

#[allow(unused_imports)]
use crate::core::{state, types};
use std::path::Path;
use super::commands::*;
use super::status_resource_detail::*;
use super::status_counts::*;
use super::status_diagnostics::*;
use super::status_fleet_detail::*;
use super::status_operational::*;
use super::status_insights::*;
use super::status_predictive::*;
use super::status_recovery::*;
use super::status_intelligence::*;
use super::status_intelligence_ext::*;
use super::status_intelligence_ext2::*;
use super::dispatch_status_ext::*;

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

#[allow(clippy::too_many_arguments)]
fn try_status_phase73(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_drift_age: bool, fleet_failed_resources: bool, resource_dependency_health: bool,
    machine_resource_age_distribution: bool, fleet_convergence_velocity: bool, resource_failure_correlation: bool,
) -> Option<Result<(), String>> {
    if machine_drift_age { return Some(cmd_status_machine_drift_age(sd, machine, json)); }
    if fleet_failed_resources { return Some(cmd_status_fleet_failed_resources(sd, machine, json)); }
    if resource_dependency_health { return Some(cmd_status_resource_dependency_health(sd, machine, json)); }
    if machine_resource_age_distribution { return Some(cmd_status_machine_resource_age_distribution(sd, machine, json)); }
    if fleet_convergence_velocity { return Some(cmd_status_fleet_convergence_velocity(sd, machine, json)); }
    if resource_failure_correlation { return Some(cmd_status_resource_failure_correlation(sd, machine, json)); }
    None
}

#[allow(clippy::too_many_arguments)]
fn try_status_phase75(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_resource_churn_rate: bool, fleet_resource_staleness: bool, machine_convergence_trend: bool,
    machine_capacity_utilization: bool, fleet_configuration_entropy: bool, machine_resource_freshness: bool,
    machine_error_budget: bool, fleet_compliance_score: bool, machine_mean_time_to_recovery: bool,
    machine_resource_dependency_health: bool, fleet_resource_type_health: bool, machine_resource_convergence_rate: bool,
) -> Option<Result<(), String>> {
    if machine_resource_churn_rate { return Some(cmd_status_machine_resource_churn_rate(sd, machine, json)); }
    if fleet_resource_staleness { return Some(cmd_status_fleet_resource_staleness(sd, machine, json)); }
    if machine_convergence_trend { return Some(cmd_status_machine_convergence_trend(sd, machine, json)); }
    if machine_capacity_utilization { return Some(cmd_status_machine_capacity_utilization(sd, machine, json)); }
    if fleet_configuration_entropy { return Some(cmd_status_fleet_configuration_entropy(sd, machine, json)); }
    if machine_resource_freshness { return Some(cmd_status_machine_resource_freshness(sd, machine, json)); }
    if machine_error_budget { return Some(cmd_status_machine_error_budget(sd, machine, json)); }
    if fleet_compliance_score { return Some(cmd_status_fleet_compliance_score(sd, machine, json)); }
    if machine_mean_time_to_recovery { return Some(cmd_status_machine_mean_time_to_recovery(sd, machine, json)); }
    if machine_resource_dependency_health { return Some(cmd_status_machine_resource_dependency_health(sd, machine, json)); }
    if fleet_resource_type_health { return Some(cmd_status_fleet_resource_type_health(sd, machine, json)); }
    if machine_resource_convergence_rate { return Some(cmd_status_machine_resource_convergence_rate(sd, machine, json)); }
    None
}

#[allow(clippy::too_many_arguments)]
fn try_status_phase79(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_resource_failure_correlation: bool, fleet_resource_age_distribution: bool, machine_resource_rollback_readiness: bool,
    machine_resource_health_trend: bool, fleet_resource_drift_velocity: bool, machine_resource_apply_success_trend: bool,
    machine_resource_mttr_estimate: bool, fleet_resource_convergence_forecast: bool, machine_resource_error_budget_forecast: bool,
    machine_resource_dependency_lag: bool, fleet_resource_dependency_lag: bool, machine_resource_config_drift_rate: bool,
    machine_resource_convergence_lag: bool, fleet_resource_convergence_lag: bool, machine_resource_dependency_depth: bool,
    machine_resource_convergence_velocity: bool, fleet_resource_convergence_velocity: bool, machine_resource_failure_recurrence: bool,
    machine_resource_drift_frequency: bool, fleet_resource_drift_frequency: bool, machine_resource_apply_duration_trend: bool,
    machine_resource_convergence_streak: bool, fleet_resource_convergence_streak: bool, machine_resource_error_distribution: bool,
) -> Option<Result<(), String>> {
    if machine_resource_failure_correlation { return Some(cmd_status_machine_resource_failure_correlation(sd, machine, json)); }
    if fleet_resource_age_distribution { return Some(cmd_status_fleet_resource_age_distribution(sd, machine, json)); }
    if machine_resource_rollback_readiness { return Some(cmd_status_machine_resource_rollback_readiness(sd, machine, json)); }
    if machine_resource_health_trend { return Some(cmd_status_machine_resource_health_trend(sd, machine, json)); }
    if fleet_resource_drift_velocity { return Some(cmd_status_fleet_resource_drift_velocity(sd, machine, json)); }
    if machine_resource_apply_success_trend { return Some(cmd_status_machine_resource_apply_success_trend(sd, machine, json)); }
    if machine_resource_mttr_estimate { return Some(cmd_status_machine_resource_mttr_estimate(sd, machine, json)); }
    if fleet_resource_convergence_forecast { return Some(cmd_status_fleet_resource_convergence_forecast(sd, machine, json)); }
    if machine_resource_error_budget_forecast { return Some(cmd_status_machine_resource_error_budget_forecast(sd, machine, json)); }
    try_status_phase82(sd, machine, json,
        machine_resource_dependency_lag, fleet_resource_dependency_lag, machine_resource_config_drift_rate,
        machine_resource_convergence_lag, fleet_resource_convergence_lag, machine_resource_dependency_depth,
        machine_resource_convergence_velocity, fleet_resource_convergence_velocity, machine_resource_failure_recurrence,
        machine_resource_drift_frequency, fleet_resource_drift_frequency, machine_resource_apply_duration_trend,
        machine_resource_convergence_streak, fleet_resource_convergence_streak, machine_resource_error_distribution,
    )
}

#[allow(clippy::too_many_arguments)]
fn try_status_phase82(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_resource_dependency_lag: bool, fleet_resource_dependency_lag: bool, machine_resource_config_drift_rate: bool,
    machine_resource_convergence_lag: bool, fleet_resource_convergence_lag: bool, machine_resource_dependency_depth: bool,
    machine_resource_convergence_velocity: bool, fleet_resource_convergence_velocity: bool, machine_resource_failure_recurrence: bool,
    machine_resource_drift_frequency: bool, fleet_resource_drift_frequency: bool, machine_resource_apply_duration_trend: bool,
    machine_resource_convergence_streak: bool, fleet_resource_convergence_streak: bool, machine_resource_error_distribution: bool,
) -> Option<Result<(), String>> {
    if machine_resource_dependency_lag { return Some(cmd_status_machine_resource_dependency_lag(sd, machine, json)); }
    if fleet_resource_dependency_lag { return Some(cmd_status_fleet_resource_dependency_lag(sd, machine, json)); }
    if machine_resource_config_drift_rate { return Some(cmd_status_machine_resource_config_drift_rate(sd, machine, json)); }
    if machine_resource_convergence_lag { return Some(cmd_status_machine_resource_convergence_lag(sd, machine, json)); }
    if fleet_resource_convergence_lag { return Some(cmd_status_fleet_resource_convergence_lag(sd, machine, json)); }
    if machine_resource_dependency_depth { return Some(cmd_status_machine_resource_dependency_depth(sd, machine, json)); }
    if machine_resource_convergence_velocity { return Some(cmd_status_machine_resource_convergence_velocity(sd, machine, json)); }
    if fleet_resource_convergence_velocity { return Some(cmd_status_fleet_resource_convergence_velocity(sd, machine, json)); }
    if machine_resource_failure_recurrence { return Some(cmd_status_machine_resource_failure_recurrence(sd, machine, json)); }
    try_status_phase85(sd, machine, json,
        machine_resource_drift_frequency, fleet_resource_drift_frequency, machine_resource_apply_duration_trend,
        machine_resource_convergence_streak, fleet_resource_convergence_streak, machine_resource_error_distribution,
    )
}

#[allow(clippy::too_many_arguments)]
fn try_status_phase85(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_resource_drift_frequency: bool, fleet_resource_drift_frequency: bool, machine_resource_apply_duration_trend: bool,
    machine_resource_convergence_streak: bool, fleet_resource_convergence_streak: bool, machine_resource_error_distribution: bool,
) -> Option<Result<(), String>> {
    if machine_resource_drift_frequency { return Some(cmd_status_machine_resource_drift_frequency(sd, machine, json)); }
    if fleet_resource_drift_frequency { return Some(cmd_status_fleet_resource_drift_frequency(sd, machine, json)); }
    if machine_resource_apply_duration_trend { return Some(cmd_status_machine_resource_apply_duration_trend(sd, machine, json)); }
    if machine_resource_convergence_streak { return Some(cmd_status_machine_resource_convergence_streak(sd, machine, json)); }
    if fleet_resource_convergence_streak { return Some(cmd_status_fleet_resource_convergence_streak(sd, machine, json)); }
    if machine_resource_error_distribution { return Some(cmd_status_machine_resource_error_distribution(sd, machine, json)); }
    None
}

#[allow(clippy::too_many_arguments)]
fn try_status_phase87(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_resource_drift_age: bool, fleet_resource_drift_age: bool, machine_resource_recovery_rate: bool,
) -> Option<Result<(), String>> {
    if machine_resource_drift_age { return Some(cmd_status_machine_resource_drift_age(sd, machine, json)); }
    if fleet_resource_drift_age { return Some(cmd_status_fleet_resource_drift_age(sd, machine, json)); }
    if machine_resource_recovery_rate { return Some(cmd_status_machine_resource_recovery_rate(sd, machine, json)); }
    None
}

#[allow(clippy::too_many_arguments)]
fn try_status_phase88(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_resource_drift_velocity: bool, fleet_resource_recovery_rate: bool, machine_resource_convergence_efficiency: bool,
) -> Option<Result<(), String>> {
    if machine_resource_drift_velocity { return Some(cmd_status_machine_resource_drift_velocity(sd, machine, json)); }
    if fleet_resource_recovery_rate { return Some(cmd_status_fleet_resource_recovery_rate(sd, machine, json)); }
    if machine_resource_convergence_efficiency { return Some(cmd_status_machine_resource_convergence_efficiency(sd, machine, json)); }
    None
}

fn try_status_phase89(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_resource_apply_frequency: bool, fleet_resource_health_score: bool, machine_resource_staleness_index: bool,
) -> Option<Result<(), String>> {
    if machine_resource_apply_frequency { return Some(cmd_status_machine_resource_apply_frequency(sd, machine, json)); }
    if fleet_resource_health_score { return Some(cmd_status_fleet_resource_health_score(sd, machine, json)); }
    if machine_resource_staleness_index { return Some(cmd_status_machine_resource_staleness_index(sd, machine, json)); }
    None
}
fn try_status_phase90(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_resource_drift_recurrence: bool, fleet_resource_drift_heatmap: bool, machine_resource_convergence_trend_p90: bool,
) -> Option<Result<(), String>> {
    if machine_resource_drift_recurrence { return Some(cmd_status_machine_resource_drift_recurrence(sd, machine, json)); }
    if fleet_resource_drift_heatmap { return Some(cmd_status_fleet_resource_drift_heatmap(sd, machine, json)); }
    if machine_resource_convergence_trend_p90 { return Some(cmd_status_machine_resource_convergence_trend_p90(sd, machine, json)); }
    None
}
#[allow(clippy::too_many_arguments)]
fn try_status_phase91(
    sd: &Path, machine: Option<&str>, json: bool,
    machine_resource_drift_age_hours: bool, fleet_resource_convergence_percentile: bool, machine_resource_error_rate: bool,
) -> Option<Result<(), String>> {
    if machine_resource_drift_age_hours { return Some(cmd_status_machine_resource_drift_age_hours(sd, machine, json)); }
    if fleet_resource_convergence_percentile { return Some(cmd_status_fleet_resource_convergence_percentile(sd, machine, json)); }
    if machine_resource_error_rate { return Some(cmd_status_machine_resource_error_rate(sd, machine, json)); }
    None
}
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
        machine_resource_age_distribution, fleet_convergence_velocity, resource_failure_correlation,
        machine_resource_churn_rate, fleet_resource_staleness, machine_convergence_trend,
        machine_capacity_utilization, fleet_configuration_entropy, machine_resource_freshness,
        machine_error_budget, fleet_compliance_score, machine_mean_time_to_recovery,
        machine_resource_dependency_health, fleet_resource_type_health, machine_resource_convergence_rate,
        machine_resource_failure_correlation, fleet_resource_age_distribution, machine_resource_rollback_readiness,
        machine_resource_health_trend, fleet_resource_drift_velocity, machine_resource_apply_success_trend,
        machine_resource_mttr_estimate, fleet_resource_convergence_forecast, machine_resource_error_budget_forecast,
        machine_resource_dependency_lag, fleet_resource_dependency_lag, machine_resource_config_drift_rate,
        machine_resource_convergence_lag, fleet_resource_convergence_lag, machine_resource_dependency_depth,
        machine_resource_convergence_velocity, fleet_resource_convergence_velocity, machine_resource_failure_recurrence,
        machine_resource_drift_frequency, fleet_resource_drift_frequency, machine_resource_apply_duration_trend,
        machine_resource_convergence_streak, fleet_resource_convergence_streak, machine_resource_error_distribution,
        machine_resource_drift_age, fleet_resource_drift_age, machine_resource_recovery_rate,
        machine_resource_drift_velocity, fleet_resource_recovery_rate, machine_resource_convergence_efficiency,
        machine_resource_apply_frequency, fleet_resource_health_score, machine_resource_staleness_index,
        machine_resource_drift_recurrence, fleet_resource_drift_heatmap, machine_resource_convergence_trend_p90,
        machine_resource_drift_age_hours, fleet_resource_convergence_percentile, machine_resource_error_rate,
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
    if let Some(r) = try_status_phase73(&state_dir, m, json, machine_drift_age, fleet_failed_resources, resource_dependency_health, machine_resource_age_distribution, fleet_convergence_velocity, resource_failure_correlation) {
        return r;
    }
    if let Some(r) = try_status_phase79(&state_dir, m, json, machine_resource_failure_correlation, fleet_resource_age_distribution, machine_resource_rollback_readiness, machine_resource_health_trend, fleet_resource_drift_velocity, machine_resource_apply_success_trend, machine_resource_mttr_estimate, fleet_resource_convergence_forecast, machine_resource_error_budget_forecast, machine_resource_dependency_lag, fleet_resource_dependency_lag, machine_resource_config_drift_rate, machine_resource_convergence_lag, fleet_resource_convergence_lag, machine_resource_dependency_depth, machine_resource_convergence_velocity, fleet_resource_convergence_velocity, machine_resource_failure_recurrence, machine_resource_drift_frequency, fleet_resource_drift_frequency, machine_resource_apply_duration_trend, machine_resource_convergence_streak, fleet_resource_convergence_streak, machine_resource_error_distribution) {
        return r;
    }
    if let Some(r) = try_status_phase87(&state_dir, m, json, machine_resource_drift_age, fleet_resource_drift_age, machine_resource_recovery_rate) {
        return r;
    }
    if let Some(r) = try_status_phase88(&state_dir, m, json, machine_resource_drift_velocity, fleet_resource_recovery_rate, machine_resource_convergence_efficiency) {
        return r;
    }
    if let Some(r) = try_status_phase89(&state_dir, m, json, machine_resource_apply_frequency, fleet_resource_health_score, machine_resource_staleness_index) {
        return r;
    }
    if let Some(r) = try_status_phase90(&state_dir, m, json, machine_resource_drift_recurrence, fleet_resource_drift_heatmap, machine_resource_convergence_trend_p90) {
        return r;
    }
    if let Some(r) = try_status_phase91(&state_dir, m, json, machine_resource_drift_age_hours, fleet_resource_convergence_percentile, machine_resource_error_rate) {
        return r;
    }
    if let Some(r) = try_status_phase75(&state_dir, m, json, machine_resource_churn_rate, fleet_resource_staleness, machine_convergence_trend, machine_capacity_utilization, fleet_configuration_entropy, machine_resource_freshness, machine_error_budget, fleet_compliance_score, machine_mean_time_to_recovery, machine_resource_dependency_health, fleet_resource_type_health, machine_resource_convergence_rate) {
        return r;
    }
    dispatch_status_early(&state_dir, m, json, file.as_deref(), summary, watch,
        machine_apply_count, fleet_apply_history, resource_hash_changes, machine_uptime_estimate, fleet_resource_type_breakdown, resource_convergence_time,
        resource_types_summary, failed_resources, drift_trend, resource_inputs, convergence_history, config_hash, last_apply_duration, drift_details_all, resource_size, hash_verify, lock_age,
        change_frequency, machine_summary, recommendations, uptime, diagnostic, resource_dependencies, pipeline_status, drift_forecast, resource_cost, security_posture,
        error_summary, resource_timeline, convergence_time, config_drift, machine_health, fleet_overview, drift_velocity, resource_graph, audit_trail, executive_summary,
        health_score, &staleness_report, cost_estimate, capacity, prediction, trend, mttr, &compliance_report, sla_report, resource_age, drift_summary,
        convergence_rate, top_failures, dependency_health, histogram, &compliance, &diff_lock, alerts, compact, &export, json_lines,
        &since, stale_resources, health_threshold, machines_only, resources_by_type, anomalies, &diff_from, count,
        &status_format, prometheus, &expired, &changes_since, &summary_by, timeline, drift_details, health, stale, &failed_since,
    )
}
