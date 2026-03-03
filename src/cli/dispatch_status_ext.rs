//! Status dispatch extension — legacy phase routing.

use super::helpers::dim;
use super::status_core::*;
#[allow(unused_imports)]
use super::status_insights::*;
use super::status_operational::*;
#[allow(unused_imports)]
use super::status_predictive::*;
#[allow(unused_imports)]
use super::status_recovery::*;
#[allow(unused_imports)]
use crate::core::{state, types};
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_status_early(
    sd: &std::path::Path,
    m: Option<&str>,
    json: bool,
    file: Option<&Path>,
    summary: bool,
    watch: Option<u64>,
    machine_apply_count: bool,
    fleet_apply_history: bool,
    resource_hash_changes: bool,
    machine_uptime_estimate: bool,
    fleet_resource_type_breakdown: bool,
    resource_convergence_time: bool,
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
    since: &Option<String>,
    stale_resources: bool,
    health_threshold: Option<u32>,
    machines_only: bool,
    resources_by_type: bool,
    anomalies: bool,
    diff_from: &Option<String>,
    count: bool,
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
) -> Result<(), String> {
    if let Some(r) = try_status_phase71(
        sd,
        m,
        json,
        machine_apply_count,
        fleet_apply_history,
        resource_hash_changes,
        machine_uptime_estimate,
        fleet_resource_type_breakdown,
        resource_convergence_time,
    ) {
        return r;
    }
    if let Some(r) = try_status_phase58(
        sd,
        m,
        json,
        resource_types_summary,
        failed_resources,
        drift_trend,
        resource_inputs,
        convergence_history,
        config_hash,
        last_apply_duration,
        drift_details_all,
        resource_size,
        hash_verify,
        lock_age,
    ) {
        return r;
    }
    if let Some(r) = try_status_analytics(
        sd,
        m,
        json,
        change_frequency,
        machine_summary,
        recommendations,
        uptime,
        diagnostic,
        resource_dependencies,
        pipeline_status,
        drift_forecast,
        resource_cost,
        security_posture,
    ) {
        return r;
    }
    if let Some(r) = try_status_fleet(
        sd,
        m,
        json,
        error_summary,
        resource_timeline,
        convergence_time,
        config_drift,
        machine_health,
        fleet_overview,
        drift_velocity,
        resource_graph,
        audit_trail,
        executive_summary,
    ) {
        return r;
    }
    if let Some(r) = try_status_reports(
        sd,
        m,
        json,
        health_score,
        staleness_report,
        cost_estimate,
        capacity,
        prediction,
        trend,
        mttr,
        compliance_report,
        sla_report,
        resource_age,
        drift_summary,
    ) {
        return r;
    }
    if let Some(r) = try_status_queries_a(
        sd,
        m,
        json,
        convergence_rate,
        top_failures,
        dependency_health,
        histogram,
        compliance,
        diff_lock,
        alerts,
        compact,
        export,
        json_lines,
    ) {
        return r;
    }
    if let Some(r) = try_status_queries_b(
        sd,
        m,
        json,
        since,
        stale_resources,
        health_threshold,
        machines_only,
        resources_by_type,
        anomalies,
        diff_from,
        count,
    ) {
        return r;
    }
    if let Some(r) = try_status_display(
        sd,
        m,
        json,
        status_format,
        prometheus,
        expired,
        changes_since,
        summary_by,
        timeline,
        drift_details,
        health,
        stale,
        failed_since,
    ) {
        return r;
    }
    if let Some(interval) = watch {
        let interval = interval.max(1);
        loop {
            print!("\x1b[2J\x1b[H");
            cmd_status(sd, m, json, file, summary)?;
            println!(
                "\n{}",
                dim(&format!("Refreshing every {}s (Ctrl+C to stop)", interval))
            );
            std::thread::sleep(std::time::Duration::from_secs(interval));
        }
    } else {
        cmd_status(sd, m, json, file, summary)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_status_phase71(
    sd: &Path,
    machine: Option<&str>,
    json: bool,
    machine_apply_count: bool,
    fleet_apply_history: bool,
    resource_hash_changes: bool,
    machine_uptime_estimate: bool,
    fleet_resource_type_breakdown: bool,
    resource_convergence_time: bool,
) -> Option<Result<(), String>> {
    if machine_apply_count {
        return Some(cmd_status_machine_apply_count(sd, machine, json));
    }
    if fleet_apply_history {
        return Some(cmd_status_fleet_apply_history(sd, machine, json));
    }
    if resource_hash_changes {
        return Some(cmd_status_resource_hash_changes(sd, machine, json));
    }
    if machine_uptime_estimate {
        return Some(cmd_status_machine_uptime_estimate(sd, machine, json));
    }
    if fleet_resource_type_breakdown {
        return Some(cmd_status_fleet_resource_type_breakdown(sd, machine, json));
    }
    if resource_convergence_time {
        return Some(cmd_status_resource_convergence_time(sd, machine, json));
    }
    None
}

pub(super) use super::dispatch_status_ext_b::*;
