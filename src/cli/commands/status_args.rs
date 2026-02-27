//! CLI Args structs for status-related commands.

use std::path::PathBuf;


#[derive(clap::Args, Debug)]
pub struct StatusArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Target specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Output status as JSON
    #[arg(long)]
    pub json: bool,

    /// Config file — enriches JSON with resource_group, tags, depends_on
    #[arg(short, long)]
    pub file: Option<PathBuf>,

    /// One-line summary for dashboards
    #[arg(long)]
    pub summary: bool,

    /// FJ-314: Watch mode — refresh every N seconds
    #[arg(long)]
    pub watch: Option<u64>,

    /// FJ-336: Show only resources not updated in N days
    #[arg(long)]
    pub stale: Option<u64>,

    /// FJ-346: Show aggregate health score (0-100)
    #[arg(long)]
    pub health: bool,

    /// FJ-355: Show detailed drift report with field-level diffs
    #[arg(long)]
    pub drift_details: bool,

    /// FJ-364: Show convergence timeline with timestamps
    #[arg(long)]
    pub timeline: bool,

    /// FJ-372: Show resources changed since a git commit
    #[arg(long)]
    pub changes_since: Option<String>,

    /// FJ-376: Group output by dimension (machine, type, or status)
    #[arg(long)]
    pub summary_by: Option<String>,

    /// FJ-382: Expose metrics in Prometheus exposition format
    #[arg(long)]
    pub prometheus: bool,

    /// FJ-387: Show resources whose lock is older than duration (e.g., 7d, 24h)
    #[arg(long)]
    pub expired: Option<String>,

    /// FJ-392: Show resource count by status (converged/failed/drifted)
    #[arg(long)]
    pub count: bool,

    /// FJ-397: Output format: table (default), json, csv
    #[arg(long)]
    pub format: Option<String>,

    /// FJ-402: Detect anomalous resource states from historical patterns
    #[arg(long)]
    pub anomalies: bool,

    /// FJ-407: Diff current state against a named snapshot
    #[arg(long)]
    pub diff_from: Option<String>,

    /// FJ-412: Group status output by resource type
    #[arg(long)]
    pub resources_by_type: bool,

    /// FJ-417: Show only machine-level summary (no resource details)
    #[arg(long)]
    pub machines_only: bool,

    /// FJ-422: Show resources not updated in any recent apply
    #[arg(long)]
    pub stale_resources: bool,

    /// FJ-427: Custom health score threshold (default: 80)
    #[arg(long)]
    pub health_threshold: Option<u32>,

    /// FJ-432: Output status as newline-delimited JSON (NDJSON)
    #[arg(long)]
    pub json_lines: bool,

    /// FJ-437: Show only resources changed within duration (e.g., 1h, 7d)
    #[arg(long)]
    pub since: Option<String>,

    /// FJ-442: Export status report to file (JSON/CSV/YAML)
    #[arg(long)]
    pub export: Option<PathBuf>,

    /// FJ-452: Minimal one-line-per-machine output for large fleets
    #[arg(long)]
    pub compact: bool,

    /// FJ-457: Show resources in alert state (failed, drifted, or stale)
    #[arg(long)]
    pub alerts: bool,

    /// FJ-462: Diff current lock against a saved lock snapshot
    #[arg(long)]
    pub diff_lock: Option<PathBuf>,

    /// FJ-467: Check compliance against named policy
    #[arg(long)]
    pub compliance: Option<String>,

    /// FJ-472: Show resource status distribution as ASCII histogram
    #[arg(long)]
    pub histogram: bool,

    /// FJ-477: Show health score weighted by dependency position
    #[arg(long)]
    pub dependency_health: bool,

    /// FJ-482: Show most frequently failing resources
    #[arg(long)]
    pub top_failures: bool,

    /// FJ-487: Show convergence percentage over time
    #[arg(long)]
    pub convergence_rate: bool,

    /// FJ-492: One-line per-machine drift count and percentage
    #[arg(long)]
    pub drift_summary: bool,

    /// FJ-497: Show age of each resource since last successful apply
    #[arg(long)]
    pub resource_age: bool,

    /// FJ-502: Show SLA compliance based on convergence timing
    #[arg(long)]
    pub sla_report: bool,

    /// FJ-507: Generate full compliance report for named policy
    #[arg(long)]
    pub compliance_report: Option<String>,

    /// FJ-512: Show mean time to recovery per resource
    #[arg(long)]
    pub mttr: bool,

    /// FJ-517: Show status trend over last N applies
    #[arg(long)]
    pub trend: Option<usize>,

    /// FJ-522: Predict next failure based on historical patterns
    #[arg(long)]
    pub prediction: bool,

    /// FJ-527: Show resource utilization vs limits per machine
    #[arg(long)]
    pub capacity: bool,

    /// FJ-532: Estimate resource cost based on type counts
    #[arg(long)]
    pub cost_estimate: bool,

    /// FJ-537: Show resources not applied within configurable window
    #[arg(long)]
    pub staleness_report: Option<String>,

    /// FJ-542: Composite health score (0-100) across all machines
    #[arg(long)]
    pub health_score: bool,

    /// FJ-547: One-line per machine summary for dashboards
    #[arg(long)]
    pub executive_summary: bool,

    /// FJ-552: Full audit trail with who/what/when for each change
    #[arg(long)]
    pub audit_trail: bool,

    /// FJ-562: Show resource dependency graph from live state
    #[arg(long)]
    pub resource_graph: bool,

    /// FJ-567: Show drift rate over time (changes per day/week)
    #[arg(long)]
    pub drift_velocity: bool,

    /// FJ-572: Aggregated fleet summary across all machines
    #[arg(long)]
    pub fleet_overview: bool,

    /// FJ-577: Per-machine health details with resource breakdown
    #[arg(long)]
    pub machine_health: bool,

    /// FJ-582: Compare running config against declared config
    #[arg(long)]
    pub config_drift: bool,

    /// FJ-587: Show average time to convergence per resource
    #[arg(long)]
    pub convergence_time: bool,

    /// FJ-592: Show per-resource status changes over time
    #[arg(long)]
    pub resource_timeline: bool,

    /// FJ-597: Aggregated error summary across all machines
    #[arg(long)]
    pub error_summary: bool,

    /// FJ-602: Show security-relevant resource states
    #[arg(long)]
    pub security_posture: bool,

    /// FJ-612: Estimate resource cost based on type and count
    #[arg(long)]
    pub resource_cost: bool,

    /// FJ-617: Predict likely drift based on historical patterns
    #[arg(long)]
    pub drift_forecast: bool,

    /// FJ-622: Show CI/CD pipeline integration status
    #[arg(long)]
    pub pipeline_status: bool,

    /// FJ-627: Show runtime dependency graph from lock files
    #[arg(long)]
    pub resource_dependencies: bool,

    /// FJ-632: Comprehensive diagnostic report with recommendations
    #[arg(long)]
    pub diagnostic: bool,

    /// FJ-642: Show resource uptime based on convergence history
    #[arg(long)]
    pub uptime: bool,

    /// FJ-647: AI-powered recommendations based on state analysis
    #[arg(long)]
    pub recommendations: bool,

    /// FJ-657: Per-machine resource count and health summary
    #[arg(long)]
    pub machine_summary: bool,

    /// FJ-662: Show how often each resource changes
    #[arg(long)]
    pub change_frequency: bool,

    /// FJ-667: Show age of each lock file entry
    #[arg(long)]
    pub lock_age: bool,

    /// FJ-672: Show resources failed since a given timestamp
    #[arg(long)]
    pub failed_since: Option<String>,

    /// FJ-677: Verify BLAKE3 hashes in lock match computed hashes
    #[arg(long)]
    pub hash_verify: bool,

    /// FJ-682: Show estimated resource sizes
    #[arg(long)]
    pub resource_size: bool,

    /// FJ-687: Show drift details for all machines at once
    #[arg(long)]
    pub drift_details_all: bool,

    /// FJ-692: Show duration of last apply per resource
    #[arg(long)]
    pub last_apply_duration: bool,

    /// FJ-697: Show hash of current config for change detection
    #[arg(long)]
    pub config_hash: bool,

    /// FJ-707: Show convergence trend over time
    #[arg(long)]
    pub convergence_history: bool,

    /// FJ-712: Show resource input fields per resource
    #[arg(long)]
    pub resource_inputs: bool,

    /// FJ-717: Show drift trend over time
    #[arg(long)]
    pub drift_trend: bool,

    /// FJ-722: Show only failed resources across machines
    #[arg(long)]
    pub failed_resources: bool,

    /// FJ-727: Show count per resource type
    #[arg(long)]
    pub resource_types_summary: bool,

    /// FJ-732: Show health status per resource (converged/failed/drifted)
    #[arg(long)]
    pub resource_health: bool,

    /// FJ-737: Show overall health per machine
    #[arg(long)]
    pub machine_health_summary: bool,

    /// FJ-742: Show inbound/outbound dependency count per resource
    #[arg(long)]
    pub dependency_count: bool,

    /// FJ-746: Show last apply success/failure per machine
    #[arg(long)]
    pub last_apply_status: bool,

    /// FJ-748: Show time since last successful apply per resource
    #[arg(long)]
    pub resource_staleness: bool,

    /// FJ-750: Show % of resources converged per machine
    #[arg(long)]
    pub convergence_percentage: bool,

    /// FJ-754: Show count of failed resources per machine
    #[arg(long)]
    pub failed_count: bool,

    /// FJ-756: Show count of drifted resources per machine
    #[arg(long)]
    pub drift_count: bool,

    /// FJ-762: Show last apply duration per resource
    #[arg(long)]
    pub resource_duration: bool,

    /// FJ-764: Show which resources target each machine
    #[arg(long)]
    pub machine_resource_map: bool,

    /// FJ-766: Aggregate convergence across all machines
    #[arg(long)]
    pub fleet_convergence: bool,

    /// FJ-770: Show BLAKE3 hash per resource from lock file
    #[arg(long)]
    pub resource_hash: bool,

    /// FJ-772: Show drift percentage per machine
    #[arg(long)]
    pub machine_drift_summary: bool,

    /// FJ-774: Show total apply count per machine from event log
    #[arg(long)]
    pub apply_history_count: bool,

    /// FJ-778: Show number of lock files per machine
    #[arg(long)]
    pub lock_file_count: bool,

    /// FJ-780: Show resource type breakdown across fleet
    #[arg(long)]
    pub resource_type_distribution: bool,

    /// FJ-782: Show time since last apply per resource
    #[arg(long)]
    pub resource_apply_age: bool,

    /// FJ-786: Show time since first apply per machine
    #[arg(long)]
    pub machine_uptime: bool,

    /// FJ-788: Show apply frequency per resource over time
    #[arg(long)]
    pub resource_churn: bool,
}

