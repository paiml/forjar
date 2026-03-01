//! FJ-017: CLI subcommands.

pub mod commands;
pub mod dispatch;
mod dispatch_apply;
mod dispatch_status;
mod dispatch_status_ext;
mod dispatch_notify;
mod dispatch_notify_custom;
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
mod validate_governance;
mod validate_ownership;
mod validate_ordering;
mod validate_ordering_ext;
mod validate_resilience;
mod validate_transport;
mod validate_analytics;
mod validate_compliance_ext;
mod validate_security;
mod validate_security_ext;
mod validate_governance_ext;
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
mod graph_intelligence;
mod graph_intelligence_ext;
mod graph_intelligence_ext2;
mod status_intelligence;
mod status_intelligence_ext;
mod status_intelligence_ext2;
mod graph_paths;
mod graph_scoring;
mod graph_visualization;
mod graph_impact;
mod graph_resilience;
mod graph_transport;
mod graph_analytics;
mod graph_analytics_ext;
mod graph_compliance;
mod graph_lifecycle;
mod graph_health;
mod graph_quality;
mod graph_governance;
mod graph_resilience_ext;
mod graph_topology_ext;
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
mod status_operational_ext;
mod status_operational_ext2;
mod status_transport;
mod status_analytics;
mod status_insights;
mod status_predictive;
mod status_recovery;
mod status_drift_intel;
mod status_security;
mod status_fleet_insight;
mod status_resource_intel;
mod validate_topology;
mod validate_config_quality;
mod status_maturity;
mod status_resilience;
mod validate_maturity;
mod validate_hygiene;
#[cfg(test)] mod test_fixtures;
#[cfg(test)] mod tests_helpers;
#[cfg(test)] mod tests_helpers_state;
#[cfg(test)] mod tests_helpers_time;
#[cfg(test)] mod tests_print_helpers;
#[cfg(test)] mod tests_apply_helpers;
#[cfg(test)] mod tests_validate_core;
#[cfg(test)] mod tests_validate_core_1;
#[cfg(test)] mod tests_validate_core_2;
#[cfg(test)] mod tests_validate_core_3;
#[cfg(test)] mod tests_apply;
#[cfg(test)] mod tests_apply_1;
#[cfg(test)] mod tests_apply_2;
#[cfg(test)] mod tests_apply_3;
#[cfg(test)] mod tests_apply_4;
#[cfg(test)] mod tests_apply_5;
#[cfg(test)] mod tests_apply_6;
#[cfg(test)] mod tests_apply_7;
#[cfg(test)] mod tests_apply_8;
#[cfg(test)] mod tests_apply_9;
#[cfg(test)] mod tests_apply_10;
#[cfg(test)] mod tests_apply_11;
#[cfg(test)] mod tests_apply_12;
#[cfg(test)] mod tests_apply_13;
#[cfg(test)] mod tests_apply_14;
#[cfg(test)] mod tests_apply_15;
#[cfg(test)] mod tests_apply_16;
#[cfg(test)] mod tests_apply_17;
#[cfg(test)] mod tests_apply_18;
#[cfg(test)] mod tests_apply_19;
#[cfg(test)] mod tests_apply_20;
#[cfg(test)] mod tests_apply_21;
#[cfg(test)] mod tests_apply_22;
#[cfg(test)] mod tests_apply_23;
#[cfg(test)] mod tests_plan;
#[cfg(test)] mod tests_plan_1;
#[cfg(test)] mod tests_status_core;
#[cfg(test)] mod tests_status_core_1;
#[cfg(test)] mod tests_status_core_2;
#[cfg(test)] mod tests_status_core_3;
#[cfg(test)] mod tests_status_core_4;
#[cfg(test)] mod tests_status_core_5;
#[cfg(test)] mod tests_status_core_6;
#[cfg(test)] mod tests_status_core_7;
#[cfg(test)] mod tests_status_core_8;
#[cfg(test)] mod tests_status_core_9;
#[cfg(test)] mod tests_status_core_10;
#[cfg(test)] mod tests_status_queries;
#[cfg(test)] mod tests_graph_core;
#[cfg(test)] mod tests_graph_core_1;
#[cfg(test)] mod tests_graph_core_2;
#[cfg(test)] mod tests_graph_core_3;
#[cfg(test)] mod tests_graph_core_4;
#[cfg(test)] mod tests_graph_core_5;
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
#[cfg(test)]
mod tests_phase70;
#[cfg(test)]
mod tests_phase71;
#[cfg(test)]
mod tests_phase72;
#[cfg(test)]
mod tests_phase73;
#[cfg(test)]
mod tests_phase74;
#[cfg(test)]
mod tests_phase75;
#[cfg(test)]
mod tests_phase76;
#[cfg(test)]
mod tests_phase77;
#[cfg(test)]
mod tests_phase78;
#[cfg(test)]
mod tests_phase79;
#[cfg(test)]
mod tests_phase80;
#[cfg(test)]
mod tests_phase81;
#[cfg(test)]
mod tests_phase82;
#[cfg(test)]
mod tests_phase83;
#[cfg(test)]
mod tests_phase84;
#[cfg(test)]
mod tests_phase85;
#[cfg(test)]
mod tests_phase86;
#[cfg(test)]
mod tests_phase87;
#[cfg(test)]
mod tests_phase88;
#[cfg(test)]
mod tests_phase89;
#[cfg(test)]
mod tests_phase90;
#[cfg(test)]
mod tests_phase91;
#[cfg(test)]
mod tests_phase92;
#[cfg(test)]
mod tests_phase94;
#[cfg(test)]
mod tests_phase98;
#[cfg(test)]
mod tests_phase99;
#[cfg(test)]
mod tests_phase100;
#[cfg(test)]
mod tests_cov_graph;
#[cfg(test)]
mod tests_cov_validate;
#[cfg(test)]
mod tests_cov_status_1;
#[cfg(test)]
mod tests_cov_status_2;
#[cfg(test)]
mod tests_cov_status_3;
#[cfg(test)]
mod tests_cov_lock;
#[cfg(test)]
mod tests_cov_graph2;
#[cfg(test)]
mod tests_cov_misc2;
#[cfg(test)]
mod tests_cov_apply;
#[cfg(test)]
mod tests_cov_fleet;
#[cfg(test)]
mod tests_cov_validate2;
#[cfg(test)]
mod tests_cov_dispatch;
#[cfg(test)]
mod tests_cov_args;
#[cfg(test)]
mod tests_cov_args_extra;
#[cfg(test)]
mod tests_cov_transport;
#[cfg(test)]
mod tests_cov_graph3;
#[cfg(test)]
mod tests_cov_status_5;
#[cfg(test)]
mod tests_cov_validate3;
#[cfg(test)]
mod tests_cov_status_4;
#[cfg(test)]
mod tests_cov_notify;
#[cfg(test)]
mod tests_cov_remaining;
#[cfg(test)]
mod tests_cov_graph3_b;
#[cfg(test)]
mod tests_cov_status_4_b;
#[cfg(test)]
mod tests_cov_status_4_c;
#[cfg(test)]
mod tests_cov_fleet_b;
#[cfg(test)]
mod tests_cov_validate2_b;
#[cfg(test)]
mod tests_cov_validate2_c;
#[cfg(test)]
mod tests_cov_validate3_b;
#[cfg(test)]
mod tests_cov_validate3_c;
#[cfg(test)]
mod tests_cov_validate3_d;
#[cfg(test)]
mod tests_cov_apply_b;
#[cfg(test)]
mod tests_cov_misc2_b;
#[cfg(test)]
mod tests_cov_args_extra_b;
#[cfg(test)]
mod tests_cov_args_2;
#[cfg(test)]
mod tests_cov_args_3;
#[cfg(test)]
mod tests_cov_args_4;
#[cfg(test)]
mod tests_cov_dispatch_2;
#[cfg(test)]
mod tests_cov_dispatch_3;
#[cfg(test)]
mod tests_cov_dispatch_4;
#[cfg(test)]
mod tests_cov_status_6;
#[cfg(test)]
mod tests_cov_status_7;
#[cfg(test)]
mod tests_cov_transport_2;
#[cfg(test)]
mod tests_cov_transport_3;
#[cfg(test)]
mod tests_cov_notify_2;
#[cfg(test)]
mod tests_cov_notify_3;
#[cfg(test)]
mod tests_cov_notify_4;
#[cfg(test)]
mod tests_cov_remaining_2;
#[cfg(test)]
mod tests_cov_remaining_3;
#[cfg(test)]
mod tests_cov_remaining_4;
#[cfg(test)]
mod tests_cov_remaining_5;
#[cfg(test)]
mod tests_cov_remaining_6;
#[cfg(test)]
mod tests_cov_remaining_7;
#[cfg(test)]
mod tests_cov_remaining_8;
#[cfg(test)]
mod tests_cov_remaining_9;
#[cfg(test)]
mod tests_phase95;
#[cfg(test)]
mod tests_validate_transport;
#[cfg(test)]
mod tests_phase96;
#[cfg(test)]
mod tests_phase97;
#[cfg(test)]
mod tests_phase101;
#[cfg(test)]
mod tests_phase102;
#[cfg(test)]
mod tests_phase103;
#[cfg(test)]
mod tests_phase104;
#[cfg(test)]
mod tests_phase105;

pub use commands::Commands;
pub use dispatch::dispatch;
