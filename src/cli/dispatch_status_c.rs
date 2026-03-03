use super::commands::*;
use super::dispatch_status::*;
use super::dispatch_status_d::*;

pub(crate) fn dispatch_status_cmd(cmd: Commands) -> Result<(), String> {
    let Commands::Status(args) = cmd else {
        unreachable!()
    };
    let m = args.machine.as_deref();
    if let Some(r) = try_status_phase59a(
        &args.state_dir,
        m,
        args.json,
        args.resource_health,
        args.machine_health_summary,
        args.last_apply_status,
        args.resource_staleness,
        args.convergence_percentage,
        args.failed_count,
        args.drift_count,
        args.resource_duration,
    ) {
        return r;
    }
    if let Some(r) = try_status_phase62(
        &args.state_dir,
        m,
        args.json,
        args.file.as_deref(),
        args.machine_resource_map,
        args.fleet_convergence,
        args.resource_hash,
        args.machine_drift_summary,
        args.apply_history_count,
        args.lock_file_count,
        args.resource_type_distribution,
    ) {
        return r;
    }
    if let Some(r) = try_status_phase65(
        &args.state_dir,
        m,
        args.json,
        args.file.as_deref(),
        args.resource_apply_age,
        args.machine_uptime,
        args.resource_churn,
        args.last_drift_time,
        args.machine_resource_count,
        args.convergence_score,
        args.apply_success_rate,
        args.error_rate,
        args.fleet_health_summary,
    ) {
        return r;
    }
    if let Some(r) = try_status_phase68(
        &args.state_dir,
        m,
        args.json,
        args.machine_convergence_history,
        args.drift_history,
        args.resource_failure_rate,
        args.machine_last_apply,
        args.fleet_drift_summary,
        args.resource_apply_duration,
        args.machine_resource_health,
        args.fleet_convergence_trend,
        args.resource_state_distribution,
    ) {
        return r;
    }
    if let Some(r) = try_status_phase73(
        &args.state_dir,
        m,
        args.json,
        args.machine_drift_age,
        args.fleet_failed_resources,
        args.resource_dependency_health,
        args.machine_resource_age_distribution,
        args.fleet_convergence_velocity,
        args.resource_failure_correlation,
    ) {
        return r;
    }
    if let Some(r) = try_status_phase79(
        &args.state_dir,
        m,
        args.json,
        args.machine_resource_failure_correlation,
        args.fleet_resource_age_distribution,
        args.machine_resource_rollback_readiness,
        args.machine_resource_health_trend,
        args.fleet_resource_drift_velocity,
        args.machine_resource_apply_success_trend,
        args.machine_resource_mttr_estimate,
        args.fleet_resource_convergence_forecast,
        args.machine_resource_error_budget_forecast,
        args.machine_resource_dependency_lag,
        args.fleet_resource_dependency_lag,
        args.machine_resource_config_drift_rate,
        args.machine_resource_convergence_lag,
        args.fleet_resource_convergence_lag,
        args.machine_resource_dependency_depth,
        args.machine_resource_convergence_velocity,
        args.fleet_resource_convergence_velocity,
        args.machine_resource_failure_recurrence,
        args.machine_resource_drift_frequency,
        args.fleet_resource_drift_frequency,
        args.machine_resource_apply_duration_trend,
        args.machine_resource_convergence_streak,
        args.fleet_resource_convergence_streak,
        args.machine_resource_error_distribution,
    ) {
        return r;
    }
    if let Some(r) = try_status_phases_87_92(
        &args.state_dir,
        m,
        args.json,
        args.machine_resource_drift_age,
        args.fleet_resource_drift_age,
        args.machine_resource_recovery_rate,
        args.machine_resource_drift_velocity,
        args.fleet_resource_recovery_rate,
        args.machine_resource_convergence_efficiency,
        args.machine_resource_apply_frequency,
        args.fleet_resource_health_score,
        args.machine_resource_staleness_index,
        args.machine_resource_drift_recurrence,
        args.fleet_resource_drift_heatmap,
        args.machine_resource_convergence_trend_p90,
        args.machine_resource_drift_age_hours,
        args.fleet_resource_convergence_percentile,
        args.machine_resource_error_rate,
        args.machine_resource_convergence_gap,
        args.fleet_resource_error_distribution,
        args.machine_resource_convergence_stability,
    ) {
        return r;
    }
    dispatch_status_cmd_tail(args)
}
