    /// FJ-846: Age of oldest drift per machine
    #[arg(long)]
    pub machine_drift_age: bool,
    /// FJ-850: List all failed resources across fleet
    #[arg(long)]
    pub fleet_failed_resources: bool,
    /// FJ-852: Health of upstream dependencies per resource
    #[arg(long)]
    pub resource_dependency_health: bool,
    /// FJ-854: Age distribution of resources per machine
    #[arg(long)]
    pub machine_resource_age_distribution: bool,
    /// FJ-858: Rate of convergence across fleet
    #[arg(long)]
    pub fleet_convergence_velocity: bool,
    /// FJ-860: Correlate failures across resources
    #[arg(long)]
    pub resource_failure_correlation: bool,
    /// FJ-862: Resource change frequency per machine over time
    #[arg(long)]
    pub machine_resource_churn_rate: bool,
    /// FJ-866: Identify resources not applied in configurable window
    #[arg(long)]
    pub fleet_resource_staleness: bool,
    /// FJ-868: Convergence trend per machine over time
    #[arg(long)]
    pub machine_convergence_trend: bool,
    /// FJ-870: Resource density and capacity metrics per machine
    #[arg(long)]
    pub machine_capacity_utilization: bool,
    /// FJ-874: Measure configuration diversity across fleet
    #[arg(long)]
    pub fleet_configuration_entropy: bool,
    /// FJ-876: Time since last successful apply per resource
    #[arg(long)]
    pub machine_resource_freshness: bool,
    /// FJ-878: Track error budget consumption per machine
    #[arg(long)]
    pub machine_error_budget: bool,
    /// FJ-882: Aggregate compliance score across fleet
    #[arg(long)]
    pub fleet_compliance_score: bool,
    /// FJ-884: MTTR metrics per machine
    #[arg(long)]
    pub machine_mean_time_to_recovery: bool,
    /// FJ-886: Health of upstream dependencies per resource
    #[arg(long)]
    pub machine_resource_dependency_health: bool,
    /// FJ-890: Health breakdown by resource type across fleet
    #[arg(long)]
    pub fleet_resource_type_health: bool,
    /// FJ-892: Convergence rate per resource per machine
    #[arg(long)]
    pub machine_resource_convergence_rate: bool,
    /// FJ-894: Correlate resource failures across machines
    #[arg(long)]
    pub machine_resource_failure_correlation: bool,
    /// FJ-898: Age distribution of resources across fleet
    #[arg(long)]
    pub fleet_resource_age_distribution: bool,
    /// FJ-900: Readiness for rollback per machine
    #[arg(long)]
    pub machine_resource_rollback_readiness: bool,
    /// FJ-902: Health trend over time per machine
    #[arg(long)]
    pub machine_resource_health_trend: bool,
    /// FJ-906: Rate of drift accumulation across fleet
    #[arg(long)]
    pub fleet_resource_drift_velocity: bool,
    /// FJ-908: Apply success trend per machine over time
    #[arg(long)]
    pub machine_resource_apply_success_trend: bool,
    /// FJ-910: Estimated MTTR per resource based on history
    #[arg(long)]
    pub machine_resource_mttr_estimate: bool,
    /// FJ-914: Forecast time to full convergence
    #[arg(long)]
    pub fleet_resource_convergence_forecast: bool,
    /// FJ-916: Forecast error budget consumption rate
    #[arg(long)]
    pub machine_resource_error_budget_forecast: bool,
    /// FJ-918: Detect lag between dependent resource convergence
    #[arg(long)]
    pub machine_resource_dependency_lag: bool,
    /// FJ-922: Fleet-wide dependency convergence lag analysis
    #[arg(long)]
    pub fleet_resource_dependency_lag: bool,
    /// FJ-924: Rate of configuration drift per machine over time
    #[arg(long)]
    pub machine_resource_config_drift_rate: bool,
    /// FJ-926: Per-resource convergence lag within machine
    #[arg(long)]
    pub machine_resource_convergence_lag: bool,
    /// FJ-930: Fleet-wide per-resource convergence lag analysis
    #[arg(long)]
    pub fleet_resource_convergence_lag: bool,
    /// FJ-932: Dependency chain depth per resource per machine
    #[arg(long)]
    pub machine_resource_dependency_depth: bool,
    /// FJ-934: Rate of convergence improvement per machine
    #[arg(long)]
    pub machine_resource_convergence_velocity: bool,
    /// FJ-938: Fleet-wide convergence improvement rate
    #[arg(long)]
    pub fleet_resource_convergence_velocity: bool,
    /// FJ-940: Frequency of repeated failures per resource
    #[arg(long)]
    pub machine_resource_failure_recurrence: bool,
    /// FJ-942: How often resources drift per machine over time
    #[arg(long)]
    pub machine_resource_drift_frequency: bool,
    /// FJ-946: Fleet-wide drift frequency aggregation
    #[arg(long)]
    pub fleet_resource_drift_frequency: bool,
    /// FJ-948: Trend analysis of apply durations per machine
    #[arg(long)]
    pub machine_resource_apply_duration_trend: bool,
    /// FJ-950: Longest consecutive convergence streak per machine
    #[arg(long)]
    pub machine_resource_convergence_streak: bool,
    /// FJ-954: Fleet-wide convergence streak aggregation
    #[arg(long)]
    pub fleet_resource_convergence_streak: bool,
    /// FJ-956: Distribution of error types per machine
    #[arg(long)]
    pub machine_resource_error_distribution: bool,
    /// FJ-958: How long each resource has been in drifted state
    #[arg(long)]
    pub machine_resource_drift_age: bool,
    /// FJ-962: Fleet-wide drift age aggregation
    #[arg(long)]
    pub fleet_resource_drift_age: bool,
    /// FJ-964: Rate of recovery from failed/drifted states
    #[arg(long)]
    pub machine_resource_recovery_rate: bool,
    /// FJ-966: Rate of drift accumulation per machine over time
    #[arg(long)]
    pub machine_resource_drift_velocity: bool,
    /// FJ-970: Fleet-wide recovery rate aggregation
    #[arg(long)]
    pub fleet_resource_recovery_rate: bool,
    /// FJ-972: Ratio of converged resources to total apply time
    #[arg(long)]
    pub machine_resource_convergence_efficiency: bool,
    /// FJ-974: Track how often each machine's resources are applied
    #[arg(long)]
    pub machine_resource_apply_frequency: bool,
    /// FJ-978: Composite fleet health score
    #[arg(long)]
    pub fleet_resource_health_score: bool,
    /// FJ-980: Index of how stale each machine's state data is
    #[arg(long)]
    pub machine_resource_staleness_index: bool,
    /// FJ-982: Count how many times each resource has drifted
    #[arg(long)]
    pub machine_resource_drift_recurrence: bool,
    /// FJ-986: Heatmap of drift across fleet machines and resources
    #[arg(long)]
    pub fleet_resource_drift_heatmap: bool,
    /// FJ-988: Trend of convergence rate over recent applies
    #[arg(long)]
    pub machine_resource_convergence_trend_p90: bool,
    /// FJ-990: How long each drifted resource has been drifted
    #[arg(long)]
    pub machine_resource_drift_age_hours: bool,
    /// FJ-994: Convergence rate at various percentiles (p50, p90, p99)
    #[arg(long)]
    pub fleet_resource_convergence_percentile: bool,
    /// FJ-996: Error rate per machine across recent applies
    #[arg(long)]
    pub machine_resource_error_rate: bool,
    /// FJ-998: Gap between expected and actual convergence rate
    #[arg(long)]
    pub machine_resource_convergence_gap: bool,
    /// FJ-1002: Distribution of errors across fleet
    #[arg(long)]
    pub fleet_resource_error_distribution: bool,
    /// FJ-1004: Stability score based on convergence rate variance
    #[arg(long)]
    pub machine_resource_convergence_stability: bool,
    /// FJ-1013: Compute p95 apply latency per machine from state lock timestamps
    #[arg(long)]
    pub machine_resource_apply_latency_p95: bool,
    /// FJ-1017: Security posture score based on permissions, secrets, firewall
    #[arg(long)]
    pub fleet_resource_security_posture_score: bool,
    /// FJ-1021: Rolling apply success rate per machine over recent applies
    #[arg(long)]
    pub fleet_apply_success_rate_trend: bool,
    /// FJ-1024: Identify resources that repeatedly drift after apply (flapping)
    #[arg(long)]
    pub machine_resource_drift_flapping: bool,
    /// FJ-1027: Heatmap of drift frequency by resource type across fleet
    #[arg(long)]
    pub fleet_resource_type_drift_heatmap: bool,
    /// FJ-1029: SSH connection latency and health per machine
    #[arg(long)]
    pub machine_ssh_connection_health: bool,
    /// FJ-1032: Lock file age and staleness per machine
    #[arg(long)]
    pub lock_file_staleness_report: bool,
    /// FJ-1035: Transport methods (local vs SSH) across fleet
    #[arg(long)]
    pub fleet_transport_method_summary: bool,
    /// FJ-1037: State lock churn patterns and apply frequency volatility
    #[arg(long)]
    pub fleet_state_churn_analysis: bool,
    /// FJ-1040: Config maturity score (0-100) per machine
    #[arg(long)]
    pub config_maturity_score: bool,
    /// FJ-1043: Fleet capacity utilization from resource counts
    #[arg(long)]
    pub fleet_capacity_utilization: bool,
    /// FJ-1045: Fleet drift velocity trend over time
    #[arg(long)]
    pub fleet_drift_velocity_trend: bool,
    /// FJ-1048: Machine convergence window estimate
    #[arg(long)]
    pub machine_convergence_window: bool,
    /// FJ-1051: Fleet resource age histogram
    #[arg(long)]
    pub fleet_resource_age_histogram: bool,
    /// FJ-1053: Fleet-wide security posture summary
    #[arg(long)]
    pub fleet_security_posture_summary: bool,
    /// FJ-1056: Machine resource freshness index
    #[arg(long)]
    pub machine_resource_freshness_index: bool,
    /// FJ-1059: Fleet resource type coverage report
    #[arg(long)]
    pub fleet_resource_type_coverage: bool,
    /// FJ-1061: Fleet apply cadence (interval between applies)
    #[arg(long)]
    pub fleet_apply_cadence: bool,
    /// FJ-1064: Machine resource error classification
    #[arg(long)]
    pub machine_resource_error_classification: bool,
    /// FJ-1067: Fleet resource convergence summary
    #[arg(long)]
    pub fleet_resource_convergence_summary: bool,
    /// FJ-1069: Fleet resource staleness report
    #[arg(long)]
    pub fleet_resource_staleness_report: bool,
    /// FJ-1072: Machine resource type distribution
    #[arg(long)]
    pub machine_resource_type_distribution: bool,
    /// FJ-1075: Fleet machine health score
    #[arg(long)]
    pub fleet_machine_health_score: bool,
    /// FJ-1077: Fleet resource dependency lag report
    #[arg(long)]
    pub fleet_resource_dependency_lag_report: bool,
    /// FJ-1080: Machine resource convergence rate trend
    #[arg(long)]
    pub machine_resource_convergence_rate_trend: bool,
    /// FJ-1083: Fleet resource apply lag
    #[arg(long)]
    pub fleet_resource_apply_lag: bool,
    /// FJ-1085: Fleet resource error rate trend
    #[arg(long)]
    pub fleet_resource_error_rate_trend: bool,
    /// FJ-1088: Machine resource drift recovery time
    #[arg(long)]
    pub machine_resource_drift_recovery_time: bool,
    /// FJ-1091: Fleet resource config complexity score
    #[arg(long)]
    pub fleet_resource_config_complexity_score: bool,
    /// FJ-1093: Fleet resource maturity index
    #[arg(long)]
    pub fleet_resource_maturity_index: bool,
    /// FJ-1096: Machine resource convergence stability index
    #[arg(long)]
    pub machine_resource_convergence_stability_index: bool,
    /// FJ-1099: Fleet resource drift pattern analysis
    #[arg(long)]
    pub fleet_resource_drift_pattern_analysis: bool,
    /// FJ-1101: Fleet resource apply success trend
    #[arg(long)]
    pub fleet_resource_apply_success_trend: bool,
    /// FJ-1104: Machine resource drift age distribution
    #[arg(long)]
    pub machine_resource_drift_age_distribution_report: bool,
    /// FJ-1107: Fleet resource convergence gap analysis
    #[arg(long)]
    pub fleet_resource_convergence_gap_analysis: bool,
    /// FJ-1109: Fleet resource type drift correlation
    #[arg(long)]
    pub fleet_resource_type_drift_correlation: bool,
    /// FJ-1112: Machine resource apply cadence report
    #[arg(long)]
    pub machine_resource_apply_cadence_report: bool,
    /// FJ-1115: Fleet resource drift recovery trend
    #[arg(long)]
    pub fleet_resource_drift_recovery_trend: bool,
    /// FJ-1117: Fleet resource quality score
    #[arg(long)]
    pub fleet_resource_quality_score: bool,
    /// FJ-1120: Machine resource drift pattern classification
    #[arg(long)]
    pub machine_resource_drift_pattern_classification: bool,
    /// FJ-1123: Fleet resource convergence window analysis
    #[arg(long)]
    pub fleet_resource_convergence_window_analysis: bool,
