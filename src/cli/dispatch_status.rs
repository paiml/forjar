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

/// One `status --*` report. Almost every flag-selected report has the same
/// shape: it reads the state directory, optionally narrowed to a single
/// machine, and prints its findings honouring `--json`.
type StatusReport = fn(&Path, Option<&str>, bool) -> Result<(), String>;

/// Run the first report whose flag is set, in table order, and return its
/// result; `None` when none of the flags in the table is set. Table order is
/// the precedence order, and only the selected report is called.
fn first_enabled_report(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    reports: &[(bool, StatusReport)],
) -> Option<Result<(), String>> {
    reports
        .iter()
        .find(|(enabled, _)| *enabled)
        .map(|(_, report)| report(sd, machine, json))
}

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
    if let Some(result) = first_enabled_report(
        sd,
        machine,
        json,
        &[
            (resource_apply_age, cmd_status_resource_apply_age),
            (machine_uptime, cmd_status_machine_uptime),
            (resource_churn, cmd_status_resource_churn),
            (last_drift_time, cmd_status_last_drift_time),
        ],
    ) {
        return Some(result);
    }
    // Counting a machine's resources is a config-file question, not a state one.
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
    first_enabled_report(
        sd,
        machine,
        json,
        &[
            (
                machine_convergence_history,
                cmd_status_machine_convergence_history,
            ),
            (drift_history, cmd_status_drift_history),
            (resource_failure_rate, cmd_status_resource_failure_rate),
            (machine_last_apply, cmd_status_machine_last_apply),
            (fleet_drift_summary, cmd_status_fleet_drift_summary),
            (resource_apply_duration, cmd_status_resource_apply_duration),
            (machine_resource_health, cmd_status_machine_resource_health),
            (fleet_convergence_trend, cmd_status_fleet_convergence_trend),
            (
                resource_state_distribution,
                cmd_status_resource_state_distribution,
            ),
        ],
    )
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
    first_enabled_report(
        sd,
        machine,
        json,
        &[
            (
                machine_resource_churn_rate,
                cmd_status_machine_resource_churn_rate,
            ),
            (
                fleet_resource_staleness,
                cmd_status_fleet_resource_staleness,
            ),
            (
                machine_convergence_trend,
                cmd_status_machine_convergence_trend,
            ),
            (
                machine_capacity_utilization,
                cmd_status_machine_capacity_utilization,
            ),
            (
                fleet_configuration_entropy,
                cmd_status_fleet_configuration_entropy,
            ),
            (
                machine_resource_freshness,
                cmd_status_machine_resource_freshness,
            ),
            (machine_error_budget, cmd_status_machine_error_budget),
            (fleet_compliance_score, cmd_status_fleet_compliance_score),
            (
                machine_mean_time_to_recovery,
                cmd_status_machine_mean_time_to_recovery,
            ),
            (
                machine_resource_dependency_health,
                cmd_status_machine_resource_dependency_health,
            ),
            (
                fleet_resource_type_health,
                cmd_status_fleet_resource_type_health,
            ),
            (
                machine_resource_convergence_rate,
                cmd_status_machine_resource_convergence_rate,
            ),
        ],
    )
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
    let phase79 = first_enabled_report(
        sd,
        machine,
        json,
        &[
            (
                machine_resource_failure_correlation,
                cmd_status_machine_resource_failure_correlation,
            ),
            (
                fleet_resource_age_distribution,
                cmd_status_fleet_resource_age_distribution,
            ),
            (
                machine_resource_rollback_readiness,
                cmd_status_machine_resource_rollback_readiness,
            ),
            (
                machine_resource_health_trend,
                cmd_status_machine_resource_health_trend,
            ),
            (
                fleet_resource_drift_velocity,
                cmd_status_fleet_resource_drift_velocity,
            ),
            (
                machine_resource_apply_success_trend,
                cmd_status_machine_resource_apply_success_trend,
            ),
            (
                machine_resource_mttr_estimate,
                cmd_status_machine_resource_mttr_estimate,
            ),
            (
                fleet_resource_convergence_forecast,
                cmd_status_fleet_resource_convergence_forecast,
            ),
            (
                machine_resource_error_budget_forecast,
                cmd_status_machine_resource_error_budget_forecast,
            ),
        ],
    );
    if phase79.is_some() {
        return phase79;
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
    let phase82 = first_enabled_report(
        sd,
        machine,
        json,
        &[
            (
                machine_resource_dependency_lag,
                cmd_status_machine_resource_dependency_lag,
            ),
            (
                fleet_resource_dependency_lag,
                cmd_status_fleet_resource_dependency_lag,
            ),
            (
                machine_resource_config_drift_rate,
                cmd_status_machine_resource_config_drift_rate,
            ),
            (
                machine_resource_convergence_lag,
                cmd_status_machine_resource_convergence_lag,
            ),
            (
                fleet_resource_convergence_lag,
                cmd_status_fleet_resource_convergence_lag,
            ),
            (
                machine_resource_dependency_depth,
                cmd_status_machine_resource_dependency_depth,
            ),
            (
                machine_resource_convergence_velocity,
                cmd_status_machine_resource_convergence_velocity,
            ),
            (
                fleet_resource_convergence_velocity,
                cmd_status_fleet_resource_convergence_velocity,
            ),
            (
                machine_resource_failure_recurrence,
                cmd_status_machine_resource_failure_recurrence,
            ),
        ],
    );
    if phase82.is_some() {
        return phase82;
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
