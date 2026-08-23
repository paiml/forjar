use super::commands::*;
use super::dispatch_status::*;
use super::dispatch_status_ext::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_status_cmd_tail(args: StatusArgs) -> Result<(), String> {
    // GH-211: FJ-742 was hidden behind a `..` rest pattern — accepted, never
    // read, and invisible to rustc. Refuse rather than print a normal status
    // report that silently omits what was asked for.
    //
    // The underscore binding is KEPT rather than reading the field through the
    // `args` binding: `tests_flag_consumer_guard` classifies a field reached
    // that way as CONSUMED, so it would make this refused flag look
    // implemented while `--help` says [UNIMPLEMENTED].
    let StatusArgs {
        dependency_count: _dependency_count,
        ..
    } = &args;
    super::inert_flags::reject_inert_flag("--dependency-count", *_dependency_count)?;
    let m = args.machine.as_deref();
    // FJ-2300/E19: Active machine connectivity probing
    if args.connectivity {
        if let Some(ref f) = args.file {
            return super::status_connectivity::cmd_status_connectivity(f, args.json);
        }
        return Err("--connectivity requires -f <config file>".to_string());
    }
    if let Some(r) = try_status_phases_94_96(
        &args.state_dir,
        m,
        args.json,
        args.machine_resource_apply_latency_p95,
        args.fleet_resource_security_posture_score,
        args.fleet_apply_success_rate_trend,
        args.machine_resource_drift_flapping,
        args.fleet_resource_type_drift_heatmap,
        args.machine_ssh_connection_health,
        args.lock_file_staleness_report,
        args.fleet_transport_method_summary,
    ) {
        return r;
    }
    if let Some(r) = try_status_phases_97_99(
        &args.state_dir,
        m,
        args.json,
        args.fleet_state_churn_analysis,
        args.config_maturity_score,
        args.fleet_capacity_utilization,
        args.fleet_drift_velocity_trend,
        args.machine_convergence_window,
        args.fleet_resource_age_histogram,
        args.fleet_security_posture_summary,
        args.machine_resource_freshness_index,
        args.fleet_resource_type_coverage,
    ) {
        return r;
    }
    if let Some(r) = try_status_phases_100_103(
        &args.state_dir,
        m,
        args.json,
        args.fleet_apply_cadence,
        args.machine_resource_error_classification,
        args.fleet_resource_convergence_summary,
        args.fleet_resource_staleness_report,
        args.machine_resource_type_distribution,
        args.fleet_machine_health_score,
        args.fleet_resource_dependency_lag_report,
        args.machine_resource_convergence_rate_trend,
        args.fleet_resource_apply_lag,
        args.fleet_resource_error_rate_trend,
        args.machine_resource_drift_recovery_time,
        args.fleet_resource_config_complexity_score,
    )
    .or_else(|| {
        try_status_phases_104_107(
            &args.state_dir,
            m,
            args.json,
            args.fleet_resource_maturity_index,
            args.machine_resource_convergence_stability_index,
            args.fleet_resource_drift_pattern_analysis,
            args.fleet_resource_apply_success_trend,
            args.machine_resource_drift_age_distribution_report,
            args.fleet_resource_convergence_gap_analysis,
            args.fleet_resource_type_drift_correlation,
            args.machine_resource_apply_cadence_report,
            args.fleet_resource_drift_recovery_trend,
            args.fleet_resource_quality_score,
            args.machine_resource_drift_pattern_classification,
            args.fleet_resource_convergence_window_analysis,
        )
    }) {
        return r;
    }
    if let Some(r) = try_status_phase75(
        &args.state_dir,
        m,
        args.json,
        args.machine_resource_churn_rate,
        args.fleet_resource_staleness,
        args.machine_convergence_trend,
        args.machine_capacity_utilization,
        args.fleet_configuration_entropy,
        args.machine_resource_freshness,
        args.machine_error_budget,
        args.fleet_compliance_score,
        args.machine_mean_time_to_recovery,
        args.machine_resource_dependency_health,
        args.fleet_resource_type_health,
        args.machine_resource_convergence_rate,
    ) {
        return r;
    }
    status_tail_to_early(&args, m)
}

/// Forward the remaining `status` flags to the legacy early dispatcher.
///
/// Split out of `dispatch_status_cmd_tail` verbatim: the argument list is
/// unchanged, in the same order, and it is still the last thing the tail
/// dispatcher does when no earlier phase claimed the run.
#[allow(clippy::too_many_arguments)]
fn status_tail_to_early(args: &StatusArgs, m: Option<&str>) -> Result<(), String> {
    dispatch_status_early(
        &args.state_dir,
        m,
        args.json,
        args.file.as_deref(),
        args.summary,
        args.watch,
        args.machine_apply_count,
        args.fleet_apply_history,
        args.resource_hash_changes,
        args.machine_uptime_estimate,
        args.fleet_resource_type_breakdown,
        args.resource_convergence_time,
        args.resource_types_summary,
        args.failed_resources,
        args.drift_trend,
        args.resource_inputs,
        args.convergence_history,
        args.config_hash,
        args.last_apply_duration,
        args.drift_details_all,
        args.resource_size,
        args.hash_verify,
        args.lock_age,
        args.change_frequency,
        args.machine_summary,
        args.recommendations,
        args.uptime,
        args.diagnostic,
        args.resource_dependencies,
        args.pipeline_status,
        args.drift_forecast,
        args.resource_cost,
        args.security_posture,
        args.error_summary,
        args.resource_timeline,
        args.convergence_time,
        args.config_drift,
        args.machine_health,
        args.fleet_overview,
        args.drift_velocity,
        args.resource_graph,
        args.audit_trail,
        args.executive_summary,
        args.health_score,
        &args.staleness_report,
        args.cost_estimate,
        args.capacity,
        args.prediction,
        args.trend,
        args.mttr,
        &args.compliance_report,
        args.sla_report,
        args.resource_age,
        args.drift_summary,
        args.convergence_rate,
        args.top_failures,
        args.dependency_health,
        args.histogram,
        &args.compliance,
        &args.diff_lock,
        args.alerts,
        args.compact,
        &args.export,
        args.json_lines,
        &args.since,
        args.stale_resources,
        args.health_threshold,
        args.machines_only,
        args.resources_by_type,
        args.anomalies,
        &args.diff_from,
        args.count,
        &args.format,
        args.prometheus,
        &args.expired,
        &args.changes_since,
        &args.summary_by,
        args.timeline,
        args.drift_details,
        args.health,
        args.stale,
        &args.failed_since,
    )
}
