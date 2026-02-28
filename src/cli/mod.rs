//! FJ-017: CLI subcommands.

pub mod commands;
pub mod dispatch;
mod dispatch_apply;
mod dispatch_status;
mod dispatch_notify;
mod dispatch_graph;
mod dispatch_validate;
mod dispatch_lock;
mod dispatch_misc;
pub mod helpers;
pub mod helpers_state;
pub mod helpers_time;
mod print_helpers;
mod apply_helpers;
mod validate_core;
mod validate_policy;
mod validate_structural;
mod validate_paths;
mod validate_quality;
mod validate_compliance;
mod validate_resources;
mod validate_safety;
mod validate_advanced;
mod apply;
mod apply_output;
mod apply_variants;
mod plan;
mod status_core;
mod status_queries;
mod status_health;
mod status_alerts;
mod status_drift;
mod status_convergence;
mod status_trends;
mod status_fleet;
mod status_resources;
mod status_resource_detail;
mod status_counts;
mod status_diagnostics;
mod status_fleet_detail;
mod status_compliance;
mod status_cost;
mod status_observability;
mod status_failures;
mod graph_core;
mod graph_topology;
mod graph_analysis;
mod graph_cross;
mod graph_extended;
mod graph_export;
mod graph_advanced;
mod graph_visualization;
mod graph_impact;
mod lock_core;
mod lock_repair;
mod lock_lifecycle;
mod lock_audit;
mod lock_ops;
mod lock_merge;
mod lock_security;
mod drift;
mod history;
mod doctor;
mod import_cmd;
mod lint;
mod workspace;
mod secrets;
mod snapshot;
mod init;
mod show;
mod check;
mod diff_cmd;
mod destroy;
mod observe;
mod fleet_ops;
mod fleet_reporting;
mod infra;
mod status_operational;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod tests_helpers;
#[cfg(test)]
mod tests_helpers_state;
#[cfg(test)]
mod tests_helpers_time;
#[cfg(test)]
mod tests_print_helpers;
#[cfg(test)]
mod tests_apply_helpers;
#[cfg(test)]
mod tests_validate_core;
#[cfg(test)]
mod tests_validate_core_1;
#[cfg(test)]
mod tests_validate_core_2;
#[cfg(test)]
mod tests_validate_core_3;
#[cfg(test)]
mod tests_apply;
#[cfg(test)]
mod tests_apply_1;
#[cfg(test)]
mod tests_apply_2;
#[cfg(test)]
mod tests_apply_3;
#[cfg(test)]
mod tests_apply_4;
#[cfg(test)]
mod tests_apply_5;
#[cfg(test)]
mod tests_apply_6;
#[cfg(test)]
mod tests_apply_7;
#[cfg(test)]
mod tests_apply_8;
#[cfg(test)]
mod tests_apply_9;
#[cfg(test)]
mod tests_apply_10;
#[cfg(test)]
mod tests_apply_11;
#[cfg(test)]
mod tests_apply_12;
#[cfg(test)]
mod tests_apply_13;
#[cfg(test)]
mod tests_apply_14;
#[cfg(test)]
mod tests_apply_15;
#[cfg(test)]
mod tests_apply_16;
#[cfg(test)]
mod tests_apply_17;
#[cfg(test)]
mod tests_apply_18;
#[cfg(test)]
mod tests_apply_19;
#[cfg(test)]
mod tests_apply_20;
#[cfg(test)]
mod tests_apply_21;
#[cfg(test)]
mod tests_apply_22;
#[cfg(test)]
mod tests_apply_23;
#[cfg(test)]
mod tests_plan;
#[cfg(test)]
mod tests_plan_1;
#[cfg(test)]
mod tests_status_core;
#[cfg(test)]
mod tests_status_core_1;
#[cfg(test)]
mod tests_status_core_2;
#[cfg(test)]
mod tests_status_core_3;
#[cfg(test)]
mod tests_status_core_4;
#[cfg(test)]
mod tests_status_core_5;
#[cfg(test)]
mod tests_status_core_6;
#[cfg(test)]
mod tests_status_core_7;
#[cfg(test)]
mod tests_status_core_8;
#[cfg(test)]
mod tests_status_core_9;
#[cfg(test)]
mod tests_status_core_10;
#[cfg(test)]
mod tests_status_queries;
#[cfg(test)]
mod tests_graph_core;
#[cfg(test)]
mod tests_graph_core_1;
#[cfg(test)]
mod tests_graph_core_2;
#[cfg(test)]
mod tests_graph_core_3;
#[cfg(test)]
mod tests_graph_core_4;
#[cfg(test)]
mod tests_graph_core_5;
#[cfg(test)]
mod tests_phase59;
#[cfg(test)]
mod tests_phase60;
#[cfg(test)]
mod tests_phase61;
#[cfg(test)]
mod tests_phase62;
#[cfg(test)]
mod tests_phase63;
#[cfg(test)]
mod tests_phase64;
#[cfg(test)]
mod tests_lock_core;
#[cfg(test)]
mod tests_lock_core_1;
#[cfg(test)]
mod tests_drift;
#[cfg(test)]
mod tests_history;
#[cfg(test)]
mod tests_doctor;
#[cfg(test)]
mod tests_import_cmd;
#[cfg(test)]
mod tests_lint;
#[cfg(test)]
mod tests_workspace;
#[cfg(test)]
mod tests_snapshot;
#[cfg(test)]
mod tests_init;
#[cfg(test)]
mod tests_show;
#[cfg(test)]
mod tests_show_1;
#[cfg(test)]
mod tests_show_2;
#[cfg(test)]
mod tests_show_3;
#[cfg(test)]
mod tests_check;
#[cfg(test)]
mod tests_check_1;
#[cfg(test)]
mod tests_check_2;
#[cfg(test)]
mod tests_diff_cmd;
#[cfg(test)]
mod tests_destroy;
#[cfg(test)]
mod tests_destroy_1;
#[cfg(test)]
mod tests_observe;
#[cfg(test)]
mod tests_fleet_ops;
#[cfg(test)]
mod tests_fleet_ops_1;
#[cfg(test)]
mod tests_fleet_reporting;
#[cfg(test)]
mod tests_infra;
#[cfg(test)]
mod tests_misc;
#[cfg(test)]
mod tests_misc_1;
#[cfg(test)]
mod tests_misc_2;
#[cfg(test)]
mod tests_misc_3;
#[cfg(test)]
mod tests_misc_4;
#[cfg(test)]
mod tests_misc_5;
#[cfg(test)]
mod tests_misc_6;
#[cfg(test)]
mod tests_misc_7;
#[cfg(test)]
mod tests_misc_8;
#[cfg(test)]
mod tests_misc_9;
#[cfg(test)]
mod tests_graph_core_6;
#[cfg(test)]
mod tests_phase65;
#[cfg(test)]
mod tests_phase66;
#[cfg(test)]
mod tests_phase67;
#[cfg(test)]
mod tests_misc_10;
#[cfg(test)]
mod tests_phase68;
#[cfg(test)]
mod tests_phase69;

pub use commands::Commands;
pub use dispatch::dispatch;
