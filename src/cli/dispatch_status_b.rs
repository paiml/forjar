use super::status_analytics::*;
use super::status_drift_intel::*;
use super::status_drift_intel2::*;
use super::status_fleet_insight::*;
use super::status_maturity::*;
use super::status_operational_ext::*;
use super::status_operational_ext2::*;
use super::status_quality::*;
use super::status_resilience::*;
use super::status_resource_intel::*;
use super::status_security::*;
use super::status_transport::*;
use super::{status_intelligence_ext::*, status_intelligence_ext2::*};
use std::path::Path;

/// Shape shared by every `status` sub-report reached from a phase dispatcher.
/// Because it is uniform, a dispatcher can be written as a table of
/// (flag, report) pairs rather than a chain of `if` statements.
pub(super) type StatusReport = fn(&Path, Option<&str>, bool) -> Result<(), String>;

/// Runs the first report whose flag is set, in table order, and returns `None`
/// when no flag in the table is set. Reports past the first match are never
/// called, so this keeps the short-circuiting the `if` chains had.
pub(super) fn first_enabled_report(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    table: &[(bool, StatusReport)],
) -> Option<Result<(), String>> {
    table
        .iter()
        .find(|(enabled, _)| *enabled)
        .map(|(_, report)| report(sd, machine, json))
}

fn try_status_phase87(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_resource_drift_age: bool,
    fleet_resource_drift_age: bool,
    machine_resource_recovery_rate: bool,
) -> Option<Result<(), String>> {
    if machine_resource_drift_age {
        return Some(cmd_status_machine_resource_drift_age(sd, machine, json));
    }
    if fleet_resource_drift_age {
        return Some(cmd_status_fleet_resource_drift_age(sd, machine, json));
    }
    if machine_resource_recovery_rate {
        return Some(cmd_status_machine_resource_recovery_rate(sd, machine, json));
    }
    None
}
fn try_status_phase88(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_resource_drift_velocity: bool,
    fleet_resource_recovery_rate: bool,
    machine_resource_convergence_efficiency: bool,
) -> Option<Result<(), String>> {
    if machine_resource_drift_velocity {
        return Some(cmd_status_machine_resource_drift_velocity(
            sd, machine, json,
        ));
    }
    if fleet_resource_recovery_rate {
        return Some(cmd_status_fleet_resource_recovery_rate(sd, machine, json));
    }
    if machine_resource_convergence_efficiency {
        return Some(cmd_status_machine_resource_convergence_efficiency(
            sd, machine, json,
        ));
    }
    None
}
fn try_status_phase89(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_resource_apply_frequency: bool,
    fleet_resource_health_score: bool,
    machine_resource_staleness_index: bool,
) -> Option<Result<(), String>> {
    if machine_resource_apply_frequency {
        return Some(cmd_status_machine_resource_apply_frequency(
            sd, machine, json,
        ));
    }
    if fleet_resource_health_score {
        return Some(cmd_status_fleet_resource_health_score(sd, machine, json));
    }
    if machine_resource_staleness_index {
        return Some(cmd_status_machine_resource_staleness_index(
            sd, machine, json,
        ));
    }
    None
}
fn try_status_phase90(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_resource_drift_recurrence: bool,
    fleet_resource_drift_heatmap: bool,
    machine_resource_convergence_trend_p90: bool,
) -> Option<Result<(), String>> {
    if machine_resource_drift_recurrence {
        return Some(cmd_status_machine_resource_drift_recurrence(
            sd, machine, json,
        ));
    }
    if fleet_resource_drift_heatmap {
        return Some(cmd_status_fleet_resource_drift_heatmap(sd, machine, json));
    }
    if machine_resource_convergence_trend_p90 {
        return Some(cmd_status_machine_resource_convergence_trend_p90(
            sd, machine, json,
        ));
    }
    None
}
fn try_status_phase91(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_resource_drift_age_hours: bool,
    fleet_resource_convergence_percentile: bool,
    machine_resource_error_rate: bool,
) -> Option<Result<(), String>> {
    if machine_resource_drift_age_hours {
        return Some(cmd_status_machine_resource_drift_age_hours(
            sd, machine, json,
        ));
    }
    if fleet_resource_convergence_percentile {
        return Some(cmd_status_fleet_resource_convergence_percentile(
            sd, machine, json,
        ));
    }
    if machine_resource_error_rate {
        return Some(cmd_status_machine_resource_error_rate(sd, machine, json));
    }
    None
}
fn try_status_phase92(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_resource_convergence_gap: bool,
    fleet_resource_error_distribution: bool,
    machine_resource_convergence_stability: bool,
) -> Option<Result<(), String>> {
    if machine_resource_convergence_gap {
        return Some(cmd_status_machine_resource_convergence_gap(
            sd, machine, json,
        ));
    }
    if fleet_resource_error_distribution {
        return Some(cmd_status_fleet_resource_error_distribution(
            sd, machine, json,
        ));
    }
    if machine_resource_convergence_stability {
        return Some(cmd_status_machine_resource_convergence_stability(
            sd, machine, json,
        ));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phases_94_96(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    a1: bool,
    a2: bool,
    b1: bool,
    b2: bool,
    b3: bool,
    c1: bool,
    c2: bool,
    c3: bool,
) -> Option<Result<(), String>> {
    if a1 {
        return Some(cmd_status_machine_resource_apply_latency_p95(
            sd, machine, json,
        ));
    }
    if a2 {
        return Some(cmd_status_fleet_resource_security_posture_score(
            sd, machine, json,
        ));
    }
    if b1 {
        return Some(cmd_status_fleet_apply_success_rate_trend(sd, machine, json));
    }
    if b2 {
        return Some(cmd_status_machine_resource_drift_flapping(
            sd, machine, json,
        ));
    }
    if b3 {
        return Some(cmd_status_fleet_resource_type_drift_heatmap(
            sd, machine, json,
        ));
    }
    if c1 {
        return Some(cmd_status_machine_ssh_connection_health(sd, machine, json));
    }
    if c2 {
        return Some(cmd_status_lock_file_staleness_report(sd, machine, json));
    }
    if c3 {
        return Some(cmd_status_fleet_transport_method_summary(sd, machine, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phases_97_99(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    d1: bool,
    d2: bool,
    d3: bool,
    e1: bool,
    e2: bool,
    e3: bool,
    f1: bool,
    f2: bool,
    f3: bool,
) -> Option<Result<(), String>> {
    first_enabled_report(
        sd,
        machine,
        json,
        &[
            (d1, cmd_status_fleet_state_churn_analysis),
            (d2, cmd_status_config_maturity_score),
            (d3, cmd_status_fleet_capacity_utilization),
            (e1, cmd_status_fleet_drift_velocity_trend),
            (e2, cmd_status_machine_convergence_window),
            (e3, cmd_status_fleet_resource_age_histogram),
            (f1, cmd_status_fleet_security_posture_summary),
            (f2, cmd_status_machine_resource_freshness_index),
            (f3, cmd_status_fleet_resource_type_coverage),
        ],
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phases_100_103(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    fleet_apply_cadence: bool,
    machine_resource_error_classification: bool,
    fleet_resource_convergence_summary: bool,
    fleet_resource_staleness_report: bool,
    machine_resource_type_distribution: bool,
    fleet_machine_health_score: bool,
    fleet_resource_dependency_lag_report: bool,
    machine_resource_convergence_rate_trend: bool,
    fleet_resource_apply_lag: bool,
    fleet_resource_error_rate_trend: bool,
    machine_resource_drift_recovery_time: bool,
    fleet_resource_config_complexity_score: bool,
) -> Option<Result<(), String>> {
    first_enabled_report(
        sd,
        machine,
        json,
        &[
            (fleet_apply_cadence, cmd_status_fleet_apply_cadence),
            (
                machine_resource_error_classification,
                cmd_status_machine_resource_error_classification,
            ),
            (
                fleet_resource_convergence_summary,
                cmd_status_fleet_resource_convergence_summary,
            ),
            (
                fleet_resource_staleness_report,
                cmd_status_fleet_resource_staleness_report,
            ),
            (
                machine_resource_type_distribution,
                cmd_status_machine_resource_type_distribution,
            ),
            (
                fleet_machine_health_score,
                cmd_status_fleet_machine_health_score,
            ),
            (
                fleet_resource_dependency_lag_report,
                cmd_status_fleet_resource_dependency_lag_report,
            ),
            (
                machine_resource_convergence_rate_trend,
                cmd_status_machine_resource_convergence_rate_trend,
            ),
            (
                fleet_resource_apply_lag,
                cmd_status_fleet_resource_apply_lag,
            ),
            (
                fleet_resource_error_rate_trend,
                cmd_status_fleet_resource_error_rate_trend,
            ),
            (
                machine_resource_drift_recovery_time,
                cmd_status_machine_resource_drift_recovery_time,
            ),
            (
                fleet_resource_config_complexity_score,
                cmd_status_fleet_resource_config_complexity_score,
            ),
        ],
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phases_104_107(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    g1: bool,
    g2: bool,
    g3: bool,
    h1: bool,
    h2: bool,
    h3: bool,
    i1: bool,
    i2: bool,
    i3: bool,
    j1: bool,
    j2: bool,
    j3: bool,
) -> Option<Result<(), String>> {
    first_enabled_report(
        sd,
        machine,
        json,
        &[
            (g1, cmd_status_fleet_resource_maturity_index),
            (g2, cmd_status_machine_resource_convergence_stability_index),
            (g3, cmd_status_fleet_resource_drift_pattern_analysis),
            (h1, cmd_status_fleet_resource_apply_success_trend),
            (h2, cmd_status_machine_resource_drift_age_distribution),
            (h3, cmd_status_fleet_resource_convergence_gap_analysis),
            (i1, cmd_status_fleet_resource_type_drift_correlation),
            (i2, cmd_status_machine_resource_apply_cadence_report),
            (i3, cmd_status_fleet_resource_drift_recovery_trend),
            (j1, cmd_status_fleet_resource_quality_score),
            (j2, cmd_status_machine_resource_drift_pattern_classification),
            (j3, cmd_status_fleet_resource_convergence_window_analysis),
        ],
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phases_87_92(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    a1: bool,
    a2: bool,
    a3: bool,
    b1: bool,
    b2: bool,
    b3: bool,
    c1: bool,
    c2: bool,
    c3: bool,
    d1: bool,
    d2: bool,
    d3: bool,
    e1: bool,
    e2: bool,
    e3: bool,
    f1: bool,
    f2: bool,
    f3: bool,
) -> Option<Result<(), String>> {
    if let Some(r) = try_status_phase87(sd, machine, json, a1, a2, a3) {
        return Some(r);
    }
    if let Some(r) = try_status_phase88(sd, machine, json, b1, b2, b3) {
        return Some(r);
    }
    if let Some(r) = try_status_phase89(sd, machine, json, c1, c2, c3) {
        return Some(r);
    }
    if let Some(r) = try_status_phase90(sd, machine, json, d1, d2, d3) {
        return Some(r);
    }
    if let Some(r) = try_status_phase91(sd, machine, json, e1, e2, e3) {
        return Some(r);
    }
    if let Some(r) = try_status_phase92(sd, machine, json, f1, f2, f3) {
        return Some(r);
    }
    None
}
