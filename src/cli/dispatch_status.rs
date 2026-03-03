use super::status_counts::*;
use super::status_diagnostics::*;
use super::status_fleet_detail::*;
use super::status_insights::*;
use super::status_intelligence::*;
use super::status_intelligence_ext::*;
use super::status_operational::*;
use super::status_predictive::*;
use super::status_recovery::*;
use super::status_resource_detail::*;
#[allow(unused_imports)]
use crate::core::{state, types};
use std::path::Path;
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase59a(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    resource_health: bool,
    machine_health_summary: bool,
    last_apply_status: bool,
    resource_staleness: bool,
    convergence_percentage: bool,
    failed_count: bool,
    drift_count: bool,
    resource_duration: bool,
) -> Option<Result<(), String>> {
    if resource_health {
        return Some(cmd_status_resource_health(sd, machine, json));
    }
    if machine_health_summary {
        return Some(cmd_status_machine_health_summary(sd, machine, json));
    }
    if last_apply_status {
        return Some(cmd_status_last_apply_status(sd, machine, json));
    }
    if resource_staleness {
        return Some(cmd_status_resource_staleness(sd, machine, json));
    }
    if convergence_percentage {
        return Some(cmd_status_convergence_percentage(sd, machine, json));
    }
    if failed_count {
        return Some(cmd_status_failed_count(sd, machine, json));
    }
    if drift_count {
        return Some(cmd_status_drift_count(sd, machine, json));
    }
    if resource_duration {
        return Some(cmd_status_resource_duration(sd, machine, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase62(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    file: Option<&Path>,
    machine_resource_map: bool,
    fleet_convergence: bool,
    resource_hash: bool,
    machine_drift_summary: bool,
    apply_history_count: bool,
    lock_file_count: bool,
    resource_type_distribution: bool,
) -> Option<Result<(), String>> {
    if machine_resource_map {
        let f = file.unwrap_or(std::path::Path::new("forjar.yaml"));
        return Some(cmd_status_machine_resource_map(f, json));
    }
    if fleet_convergence {
        return Some(cmd_status_fleet_convergence(sd, json));
    }
    if resource_hash {
        return Some(cmd_status_resource_hash(sd, machine, json));
    }
    if machine_drift_summary {
        return Some(cmd_status_machine_drift_summary(sd, machine, json));
    }
    if apply_history_count {
        return Some(cmd_status_apply_history_count(sd, machine, json));
    }
    if lock_file_count {
        return Some(cmd_status_lock_file_count(sd, json));
    }
    if resource_type_distribution {
        let f = file.unwrap_or(std::path::Path::new("forjar.yaml"));
        return Some(cmd_status_resource_type_distribution(f, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase65(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    file: Option<&Path>,
    resource_apply_age: bool,
    machine_uptime: bool,
    resource_churn: bool,
    last_drift_time: bool,
    machine_resource_count: bool,
    convergence_score: bool,
    apply_success_rate: bool,
    error_rate: bool,
    fleet_health_summary: bool,
) -> Option<Result<(), String>> {
    if resource_apply_age {
        return Some(cmd_status_resource_apply_age(sd, machine, json));
    }
    if machine_uptime {
        return Some(cmd_status_machine_uptime(sd, machine, json));
    }
    if resource_churn {
        return Some(cmd_status_resource_churn(sd, machine, json));
    }
    if last_drift_time {
        return Some(cmd_status_last_drift_time(sd, machine, json));
    }
    if machine_resource_count {
        let f = file.unwrap_or(std::path::Path::new("forjar.yaml"));
        return Some(cmd_status_machine_resource_count(f, json));
    }
    if convergence_score {
        return Some(cmd_status_convergence_score(sd, json));
    }
    if apply_success_rate {
        return Some(cmd_status_apply_success_rate(sd, machine, json));
    }
    if error_rate {
        return Some(cmd_status_error_rate(sd, machine, json));
    }
    if fleet_health_summary {
        return Some(cmd_status_fleet_health_summary(sd, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase68(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_convergence_history: bool,
    drift_history: bool,
    resource_failure_rate: bool,
    machine_last_apply: bool,
    fleet_drift_summary: bool,
    resource_apply_duration: bool,
    machine_resource_health: bool,
    fleet_convergence_trend: bool,
    resource_state_distribution: bool,
) -> Option<Result<(), String>> {
    if machine_convergence_history {
        return Some(cmd_status_machine_convergence_history(sd, machine, json));
    }
    if drift_history {
        return Some(cmd_status_drift_history(sd, machine, json));
    }
    if resource_failure_rate {
        return Some(cmd_status_resource_failure_rate(sd, machine, json));
    }
    if machine_last_apply {
        return Some(cmd_status_machine_last_apply(sd, machine, json));
    }
    if fleet_drift_summary {
        return Some(cmd_status_fleet_drift_summary(sd, machine, json));
    }
    if resource_apply_duration {
        return Some(cmd_status_resource_apply_duration(sd, machine, json));
    }
    if machine_resource_health {
        return Some(cmd_status_machine_resource_health(sd, machine, json));
    }
    if fleet_convergence_trend {
        return Some(cmd_status_fleet_convergence_trend(sd, machine, json));
    }
    if resource_state_distribution {
        return Some(cmd_status_resource_state_distribution(sd, machine, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase73(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_drift_age: bool,
    fleet_failed_resources: bool,
    resource_dependency_health: bool,
    machine_resource_age_distribution: bool,
    fleet_convergence_velocity: bool,
    resource_failure_correlation: bool,
) -> Option<Result<(), String>> {
    if machine_drift_age {
        return Some(cmd_status_machine_drift_age(sd, machine, json));
    }
    if fleet_failed_resources {
        return Some(cmd_status_fleet_failed_resources(sd, machine, json));
    }
    if resource_dependency_health {
        return Some(cmd_status_resource_dependency_health(sd, machine, json));
    }
    if machine_resource_age_distribution {
        return Some(cmd_status_machine_resource_age_distribution(
            sd, machine, json,
        ));
    }
    if fleet_convergence_velocity {
        return Some(cmd_status_fleet_convergence_velocity(sd, machine, json));
    }
    if resource_failure_correlation {
        return Some(cmd_status_resource_failure_correlation(sd, machine, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase75(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_resource_churn_rate: bool,
    fleet_resource_staleness: bool,
    machine_convergence_trend: bool,
    machine_capacity_utilization: bool,
    fleet_configuration_entropy: bool,
    machine_resource_freshness: bool,
    machine_error_budget: bool,
    fleet_compliance_score: bool,
    machine_mean_time_to_recovery: bool,
    machine_resource_dependency_health: bool,
    fleet_resource_type_health: bool,
    machine_resource_convergence_rate: bool,
) -> Option<Result<(), String>> {
    if machine_resource_churn_rate {
        return Some(cmd_status_machine_resource_churn_rate(sd, machine, json));
    }
    if fleet_resource_staleness {
        return Some(cmd_status_fleet_resource_staleness(sd, machine, json));
    }
    if machine_convergence_trend {
        return Some(cmd_status_machine_convergence_trend(sd, machine, json));
    }
    if machine_capacity_utilization {
        return Some(cmd_status_machine_capacity_utilization(sd, machine, json));
    }
    if fleet_configuration_entropy {
        return Some(cmd_status_fleet_configuration_entropy(sd, machine, json));
    }
    if machine_resource_freshness {
        return Some(cmd_status_machine_resource_freshness(sd, machine, json));
    }
    if machine_error_budget {
        return Some(cmd_status_machine_error_budget(sd, machine, json));
    }
    if fleet_compliance_score {
        return Some(cmd_status_fleet_compliance_score(sd, machine, json));
    }
    if machine_mean_time_to_recovery {
        return Some(cmd_status_machine_mean_time_to_recovery(sd, machine, json));
    }
    if machine_resource_dependency_health {
        return Some(cmd_status_machine_resource_dependency_health(
            sd, machine, json,
        ));
    }
    if fleet_resource_type_health {
        return Some(cmd_status_fleet_resource_type_health(sd, machine, json));
    }
    if machine_resource_convergence_rate {
        return Some(cmd_status_machine_resource_convergence_rate(
            sd, machine, json,
        ));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase79(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_resource_failure_correlation: bool,
    fleet_resource_age_distribution: bool,
    machine_resource_rollback_readiness: bool,
    machine_resource_health_trend: bool,
    fleet_resource_drift_velocity: bool,
    machine_resource_apply_success_trend: bool,
    machine_resource_mttr_estimate: bool,
    fleet_resource_convergence_forecast: bool,
    machine_resource_error_budget_forecast: bool,
    machine_resource_dependency_lag: bool,
    fleet_resource_dependency_lag: bool,
    machine_resource_config_drift_rate: bool,
    machine_resource_convergence_lag: bool,
    fleet_resource_convergence_lag: bool,
    machine_resource_dependency_depth: bool,
    machine_resource_convergence_velocity: bool,
    fleet_resource_convergence_velocity: bool,
    machine_resource_failure_recurrence: bool,
    machine_resource_drift_frequency: bool,
    fleet_resource_drift_frequency: bool,
    machine_resource_apply_duration_trend: bool,
    machine_resource_convergence_streak: bool,
    fleet_resource_convergence_streak: bool,
    machine_resource_error_distribution: bool,
) -> Option<Result<(), String>> {
    if machine_resource_failure_correlation {
        return Some(cmd_status_machine_resource_failure_correlation(
            sd, machine, json,
        ));
    }
    if fleet_resource_age_distribution {
        return Some(cmd_status_fleet_resource_age_distribution(
            sd, machine, json,
        ));
    }
    if machine_resource_rollback_readiness {
        return Some(cmd_status_machine_resource_rollback_readiness(
            sd, machine, json,
        ));
    }
    if machine_resource_health_trend {
        return Some(cmd_status_machine_resource_health_trend(sd, machine, json));
    }
    if fleet_resource_drift_velocity {
        return Some(cmd_status_fleet_resource_drift_velocity(sd, machine, json));
    }
    if machine_resource_apply_success_trend {
        return Some(cmd_status_machine_resource_apply_success_trend(
            sd, machine, json,
        ));
    }
    if machine_resource_mttr_estimate {
        return Some(cmd_status_machine_resource_mttr_estimate(sd, machine, json));
    }
    if fleet_resource_convergence_forecast {
        return Some(cmd_status_fleet_resource_convergence_forecast(
            sd, machine, json,
        ));
    }
    if machine_resource_error_budget_forecast {
        return Some(cmd_status_machine_resource_error_budget_forecast(
            sd, machine, json,
        ));
    }
    try_status_phase82(
        sd,
        machine,
        json,
        machine_resource_dependency_lag,
        fleet_resource_dependency_lag,
        machine_resource_config_drift_rate,
        machine_resource_convergence_lag,
        fleet_resource_convergence_lag,
        machine_resource_dependency_depth,
        machine_resource_convergence_velocity,
        fleet_resource_convergence_velocity,
        machine_resource_failure_recurrence,
        machine_resource_drift_frequency,
        fleet_resource_drift_frequency,
        machine_resource_apply_duration_trend,
        machine_resource_convergence_streak,
        fleet_resource_convergence_streak,
        machine_resource_error_distribution,
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase82(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_resource_dependency_lag: bool,
    fleet_resource_dependency_lag: bool,
    machine_resource_config_drift_rate: bool,
    machine_resource_convergence_lag: bool,
    fleet_resource_convergence_lag: bool,
    machine_resource_dependency_depth: bool,
    machine_resource_convergence_velocity: bool,
    fleet_resource_convergence_velocity: bool,
    machine_resource_failure_recurrence: bool,
    machine_resource_drift_frequency: bool,
    fleet_resource_drift_frequency: bool,
    machine_resource_apply_duration_trend: bool,
    machine_resource_convergence_streak: bool,
    fleet_resource_convergence_streak: bool,
    machine_resource_error_distribution: bool,
) -> Option<Result<(), String>> {
    if machine_resource_dependency_lag {
        return Some(cmd_status_machine_resource_dependency_lag(
            sd, machine, json,
        ));
    }
    if fleet_resource_dependency_lag {
        return Some(cmd_status_fleet_resource_dependency_lag(sd, machine, json));
    }
    if machine_resource_config_drift_rate {
        return Some(cmd_status_machine_resource_config_drift_rate(
            sd, machine, json,
        ));
    }
    if machine_resource_convergence_lag {
        return Some(cmd_status_machine_resource_convergence_lag(
            sd, machine, json,
        ));
    }
    if fleet_resource_convergence_lag {
        return Some(cmd_status_fleet_resource_convergence_lag(sd, machine, json));
    }
    if machine_resource_dependency_depth {
        return Some(cmd_status_machine_resource_dependency_depth(
            sd, machine, json,
        ));
    }
    if machine_resource_convergence_velocity {
        return Some(cmd_status_machine_resource_convergence_velocity(
            sd, machine, json,
        ));
    }
    if fleet_resource_convergence_velocity {
        return Some(cmd_status_fleet_resource_convergence_velocity(
            sd, machine, json,
        ));
    }
    if machine_resource_failure_recurrence {
        return Some(cmd_status_machine_resource_failure_recurrence(
            sd, machine, json,
        ));
    }
    try_status_phase85(
        sd,
        machine,
        json,
        machine_resource_drift_frequency,
        fleet_resource_drift_frequency,
        machine_resource_apply_duration_trend,
        machine_resource_convergence_streak,
        fleet_resource_convergence_streak,
        machine_resource_error_distribution,
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase85(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_resource_drift_frequency: bool,
    fleet_resource_drift_frequency: bool,
    machine_resource_apply_duration_trend: bool,
    machine_resource_convergence_streak: bool,
    fleet_resource_convergence_streak: bool,
    machine_resource_error_distribution: bool,
) -> Option<Result<(), String>> {
    if machine_resource_drift_frequency {
        return Some(cmd_status_machine_resource_drift_frequency(
            sd, machine, json,
        ));
    }
    if fleet_resource_drift_frequency {
        return Some(cmd_status_fleet_resource_drift_frequency(sd, machine, json));
    }
    if machine_resource_apply_duration_trend {
        return Some(cmd_status_machine_resource_apply_duration_trend(
            sd, machine, json,
        ));
    }
    if machine_resource_convergence_streak {
        return Some(cmd_status_machine_resource_convergence_streak(
            sd, machine, json,
        ));
    }
    if fleet_resource_convergence_streak {
        return Some(cmd_status_fleet_resource_convergence_streak(
            sd, machine, json,
        ));
    }
    if machine_resource_error_distribution {
        return Some(cmd_status_machine_resource_error_distribution(
            sd, machine, json,
        ));
    }
    None
}

pub(super) use super::dispatch_status_b::*;
pub(super) use super::dispatch_status_c::*;
