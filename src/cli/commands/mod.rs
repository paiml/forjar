mod apply_args;
mod graph_args;
mod lock_core_args;
mod lock_ops_args;
mod misc_analysis_args;
mod misc_args;
mod misc_ops_args;
mod ops_intel_args;
mod plan_args;
mod platform_args;
mod state_args;
mod status_args;
mod store_args;
mod subcmd_args;
mod validate_args;
pub use apply_args::*;
use clap::Subcommand;
pub use graph_args::*;
pub use lock_core_args::*;
pub use lock_ops_args::*;
pub use misc_analysis_args::*;
pub use misc_args::*;
pub use misc_ops_args::*;
pub use ops_intel_args::*;
pub use plan_args::*;
pub use platform_args::*;
pub use state_args::*;
pub use status_args::*;
pub use store_args::*;
pub use subcmd_args::*;
pub use validate_args::*;

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Commands {
    /// Initialize a new forjar project
    Init(InitArgs),

    /// Validate forjar.yaml without connecting to machines
    Validate(ValidateArgs),

    /// Show execution plan (diff desired vs current)
    Plan(PlanArgs),

    /// Converge infrastructure to desired state
    Apply(ApplyArgs),

    /// Detect unauthorized changes (tripwire)
    Drift(DriftArgs),

    /// Show current state from lock files
    Status(StatusArgs),

    /// Show apply history from event logs
    History(HistoryArgs),

    /// Remove all managed resources (reverse order)
    Destroy(DestroyArgs),

    /// Import existing infrastructure from a machine into forjar.yaml
    Import(ImportArgs),

    /// Show fully resolved config (recipes expanded, templates resolved)
    Show(ShowArgs),

    /// Show resource dependency graph
    Graph(GraphArgs),

    /// Run check scripts to verify pre-conditions without applying
    Check(CheckArgs),

    /// Compare two state snapshots (show what changed between applies)
    Diff(DiffArgs),

    /// FJ-1389: Unified stack diff — compare two configs (resources, machines, params)
    StackDiff(StackDiffArgs),

    /// Format (normalize) a forjar.yaml config file
    Fmt(FmtArgs),

    /// Lint config for best practices (beyond validation)
    Lint(LintArgs),

    /// Rollback to a previous config revision from git history
    Rollback(RollbackArgs),

    /// Detect anomalous resource behavior from event history
    Anomaly(AnomalyArgs),

    /// View trace provenance data from apply runs (FJ-050)
    Trace(TraceArgs),

    /// Migrate Docker resources to pepita kernel isolation (FJ-044)
    Migrate(MigrateArgs),

    /// Start MCP server (pforge integration, FJ-063)
    Mcp(McpArgs),

    /// Run performance benchmarks (spec §9 targets)
    Bench(BenchArgs),

    /// List all resources in state with type, status, hash prefix (FJ-214)
    #[command(name = "state-list")]
    StateList(StateListArgs),

    /// Rename a resource in state without re-applying (FJ-212)
    #[command(name = "state-mv")]
    StateMv(StateMvArgs),

    /// Remove a resource from state without destroying it on the machine (FJ-213)
    #[command(name = "state-rm")]
    StateRm(StateRmArgs),

    /// Show computed output values from forjar.yaml (FJ-215)
    Output(OutputArgs),

    /// FJ-220: Evaluate policy rules against config
    #[command(name = "policy")]
    Policy(PolicyArgs),

    /// FJ-210: Manage workspaces (isolated state directories)
    #[command(subcommand)]
    Workspace(WorkspaceCmd),

    /// FJ-200: Manage age-encrypted secrets
    #[command(subcommand)]
    Secrets(SecretsCmd),

    /// FJ-251: Pre-flight system checker
    Doctor(DoctorArgs),

    /// FJ-253: Generate shell completions
    Completion(CompletionArgs),

    /// FJ-264: Export JSON Schema for forjar.yaml
    Schema,

    /// FJ-267: Watch config for changes and auto-plan
    Watch(WatchArgs),

    /// FJ-271: Show full resolution chain for a resource
    Explain(ExplainArgs),

    /// FJ-277: Show resolved environment info
    Env(EnvArgs),

    /// FJ-273: Run check scripts for all resources, report pass/fail table
    Test(TestArgs),

    /// FJ-256: Generate lock file without applying
    Lock(LockArgs),

    /// FJ-260: Manage state snapshots
    #[command(subcommand)]
    Snapshot(SnapshotCmd),

    /// FJ-1386: Manage state generations (Nix-style numbered snapshots)
    #[command(subcommand)]
    Generation(GenerationCmd),

    /// FJ-326: List all machines with connection status
    Inventory(InventoryArgs),

    /// FJ-327: Re-run only previously failed resources
    RetryFailed(RetryFailedArgs),

    /// FJ-324: Rolling deployment — apply N machines at a time
    Rolling(RollingArgs),

    /// FJ-325: Canary deployment — apply to one machine first, confirm, then rest
    Canary(CanaryArgs),

    /// FJ-341: Show full audit trail — who applied what, when, from which config
    Audit(AuditArgs),

    /// FJ-344: One-line-per-resource plan output for large configs
    #[command(name = "plan-compact")]
    PlanCompact(PlanCompactArgs),

    /// FJ-351: Validate infrastructure against policy rules
    Compliance(ComplianceArgs),

    /// FJ-352: Export state to external formats (terraform, ansible, csv)
    Export(ExportArgs),

    /// FJ-361: Analyze config and suggest improvements
    Suggest(SuggestArgs),

    /// FJ-363: Compare two config files and show differences
    Compare(CompareArgs),

    /// FJ-366: Remove lock entries for resources no longer in config
    #[command(name = "lock-prune")]
    LockPrune(LockPruneArgs),

    /// FJ-367: Compare environments (workspaces) for drift
    #[command(name = "env-diff")]
    EnvDiff(EnvDiffArgs),

    /// FJ-371: Expand a recipe template to stdout without applying
    Template(TemplateArgs),

    /// FJ-384: Show lock file metadata
    #[command(name = "lock-info")]
    LockInfo(LockInfoArgs),

    /// FJ-395: Compact lock file — remove historical entries, keep latest per resource
    #[command(name = "lock-compact")]
    LockCompact(LockCompactArgs),

    /// FJ-425: Garbage collect orphaned lock entries with no matching config
    #[command(name = "lock-gc")]
    LockGc(LockGcArgs),

    /// FJ-415: Export lock file in alternative format
    #[command(name = "lock-export")]
    LockExport(LockExportArgs),

    /// FJ-405: Verify lock file integrity (BLAKE3 checksums)
    #[command(name = "lock-verify")]
    LockVerify(LockVerifyArgs),

    /// FJ-435: Compare two lock files and show resource-level differences
    LockDiff(LockDiffArgs),

    /// FJ-445: Merge two lock files (multi-team workflow)
    #[command(name = "lock-merge")]
    LockMerge(LockMergeArgs),

    /// FJ-455: Rebase lock file from one config version to another
    #[command(name = "lock-rebase")]
    LockRebase(LockRebaseArgs),

    /// FJ-465: Cryptographically sign lock file with BLAKE3
    #[command(name = "lock-sign")]
    LockSign(LockSignArgs),

    /// FJ-475: Verify lock file signature against signing key
    #[command(name = "lock-verify-sig")]
    LockVerifySig(LockVerifySigArgs),

    /// FJ-485: Compact all machine lock files in one operation
    #[command(name = "lock-compact-all")]
    LockCompactAll(LockCompactAllArgs),

    /// FJ-495: Show full audit trail of lock file changes with timestamps
    #[command(name = "lock-audit-trail")]
    LockAuditTrail(LockAuditTrailArgs),

    /// FJ-505: Rotate all lock file signing keys
    #[command(name = "lock-rotate-keys")]
    LockRotateKeys(LockRotateKeysArgs),

    /// FJ-515: Create timestamped backup of all lock files
    #[command(name = "lock-backup")]
    LockBackup(LockBackupArgs),

    /// FJ-535: Verify full chain of custody from lock signatures
    #[command(name = "lock-verify-chain")]
    LockVerifyChain(LockVerifyChainArgs),

    /// FJ-545: Show lock file statistics (sizes, ages, resource counts)
    #[command(name = "lock-stats")]
    LockStats(LockStatsArgs),

    /// FJ-555: Verify lock file integrity and show tampering evidence
    LockAudit(LockAuditArgs),

    /// FJ-565: Compress old lock files with zstd
    LockCompress(LockCompressArgs),

    /// FJ-575: Defragment lock files (reorder resources alphabetically)
    LockDefrag(LockDefragArgs),

    /// FJ-585: Normalize lock file format (consistent key ordering, whitespace)
    LockNormalize(LockNormalizeArgs),

    /// FJ-595: Validate lock file schema and cross-references
    LockValidate(LockValidateArgs),

    /// FJ-605: Verify lock file HMAC signatures
    #[command(name = "lock-verify-hmac")]
    LockVerifyHmac(LockVerifyHmacArgs),

    /// FJ-615: Archive old lock files to compressed storage
    #[command(name = "lock-archive")]
    LockArchive(LockArchiveArgs),

    /// FJ-625: Create point-in-time lock file snapshot with metadata
    #[command(name = "lock-snapshot")]
    LockSnapshot(LockSnapshotArgs),

    /// FJ-635: Attempt automatic repair of corrupted lock files
    #[command(name = "lock-repair")]
    LockRepair(LockRepairArgs),

    /// FJ-645: Show lock file change history with diffs
    #[command(name = "lock-history")]
    LockHistory(LockHistoryArgs),

    /// FJ-675: Check lock file structural integrity
    #[command(name = "lock-integrity")]
    LockIntegrity(LockIntegrityArgs),

    /// FJ-685: Rehash all lock file entries with current BLAKE3
    #[command(name = "lock-rehash")]
    LockRehash(LockRehashArgs),

    /// FJ-695: Restore lock state from a named snapshot
    #[command(name = "lock-restore")]
    LockRestore(LockRestoreArgs),

    /// FJ-705: Verify lock file schema version compatibility
    #[command(name = "lock-verify-schema")]
    LockVerifySchema(LockVerifySchemaArgs),

    /// FJ-715: Add metadata tags to lock files
    #[command(name = "lock-tag")]
    LockTag(LockTagArgs),

    /// FJ-725: Migrate lock file schema between versions
    #[command(name = "lock-migrate")]
    LockMigrate(LockMigrateArgs),

    /// Score a forjar config — multi-dimensional quality grade (A–F)
    Score(ScoreArgs),

    /// FJ-1311: Pin all inputs to current versions (lock file management)
    Pin(PinArgs),

    /// FJ-1323: Binary cache operations (list, push, pull, verify)
    #[command(subcommand)]
    Cache(CacheCmd),

    /// FJ-1327: Content-addressed store operations (gc, list, diff, sync)
    #[command(subcommand)]
    Store(StoreCmd),

    /// FJ-1346: Forjar Archive (FAR) operations (pack, unpack, inspect, verify)
    #[command(subcommand)]
    Archive(ArchiveCmd),

    /// FJ-1328: Convert recipe to reproducible format (version pin, store, lock)
    Convert(ConvertArgs),

    /// FJ-1333: Universal store import (apt, cargo, uv, nix, docker, tofu, terraform, apr)
    #[command(name = "store-import")]
    StoreImport(StoreImportArgs),

    /// FJ-1280: Reconstruct state at a point in time from event log
    #[command(name = "state-reconstruct")]
    StateReconstruct(StateReconstructArgs),

    /// FJ-1383: Merge two forjar config files into one
    #[command(name = "config-merge")]
    ConfigMerge(ConfigMergeArgs),

    /// FJ-1384: Extract resources matching tag/group/glob into sub-config
    Extract(ExtractArgs),

    /// FJ-1390: Static IaC security scanner
    #[command(name = "security-scan")]
    SecurityScan(SecurityScanArgs),

    /// FJ-1395: Generate SBOM (Software Bill of Materials) for managed infrastructure
    Sbom(SbomArgs),

    /// FJ-1400: Generate CBOM (Cryptographic Bill of Materials) for managed infrastructure
    Cbom(CbomArgs),

    /// FJ-1401: Prove convergence from current or arbitrary state
    Prove(ProveArgs),

    /// FJ-1403: Analyze minimum privileges required per resource
    #[command(name = "privilege-analysis")]
    PrivilegeAnalysis(PrivilegeAnalysisArgs),

    /// FJ-1404: Generate SLSA provenance attestation
    Provenance(ProvenanceArgs),

    /// FJ-1405: Show Merkle DAG configuration lineage
    Lineage(LineageArgs),

    /// FJ-1406: Package config + dependencies into self-contained bundle
    Bundle(BundleArgs),

    /// FJ-1407: Generate model card for ML resources
    #[command(name = "model-card")]
    ModelCard(ModelCardArgs),

    /// FJ-1408: Generate agent-specific SBOM
    #[command(name = "agent-sbom")]
    AgentSbom(AgentSbomArgs),

    /// FJ-1409: Generate training reproducibility certificate
    #[command(name = "repro-proof")]
    ReproProof(ReproProofArgs),

    /// FJ-1410: Data freshness monitoring — detect stale artifacts
    #[command(name = "data-freshness")]
    DataFreshness(DataFreshnessArgs),

    /// FJ-1411: Declarative data validation checks
    #[command(name = "data-validate")]
    DataValidate(DataValidateArgs),

    /// FJ-1412: Training checkpoint management
    Checkpoint(CheckpointArgs),

    /// FJ-1413: Dataset versioning and lineage tracking
    #[command(name = "dataset-lineage")]
    DatasetLineage(DatasetLineageArgs),

    /// FJ-1414: Data sovereignty tagging and compliance
    Sovereignty(SovereigntyArgs),

    /// FJ-1415: Cost estimation and resource budgeting
    #[command(name = "cost-estimate")]
    CostEstimate(CostEstimateArgs),

    /// FJ-1416: Model evaluation pipeline
    #[command(name = "model-eval")]
    ModelEval(ModelEvalArgs),

    /// FJ-1420: Fault injection testing
    #[command(name = "fault-inject")]
    FaultInject(FaultInjectArgs),

    /// FJ-1421: Runtime invariant monitors
    #[command(name = "invariants")]
    Invariants(InvariantsArgs),

    /// FJ-1422: ISO distribution export
    #[command(name = "iso-export")]
    IsoExport(IsoExportArgs),

    /// FJ-1423: Brownfield state import
    #[command(name = "import-brownfield")]
    ImportBrownfield(ImportBrownfieldArgs),

    /// FJ-1424: Cross-machine dependency analysis
    #[command(name = "cross-deps")]
    CrossDeps(CrossDepsArgs),

    /// FJ-1425: Remote state backend operations
    #[command(name = "state-backend")]
    StateBackend(StateBackendArgs),

    /// FJ-1426: Recipe registry listing
    #[command(name = "registry-list")]
    RegistryList(RegistryListArgs),

    /// FJ-1427: Service catalog listing
    #[command(name = "catalog-list")]
    CatalogList(CatalogListArgs),

    /// FJ-1428: Multi-config apply ordering
    #[command(name = "multi-apply")]
    MultiApply(MultiConfigArgs),

    /// FJ-1429: Stack dependency graph
    #[command(name = "stack-graph")]
    StackGraph(StackGraphArgs),

    /// FJ-1430+1431: Infrastructure query
    #[command(name = "query")]
    InfraQuery(InfraQueryArgs),

    /// FJ-1432+1433: Recipe signing
    #[command(name = "sign")]
    RecipeSign(RecipeSignArgs),

    /// FJ-1434: Preservation checking
    #[command(name = "preservation")]
    Preservation(PreservationArgs),

    /// FJ-1435: Parallel multi-stack apply
    #[command(name = "parallel-apply")]
    ParallelApply(ParallelStackArgs),

    /// FJ-1436: Saga-pattern multi-stack apply
    #[command(name = "saga")]
    Saga(SagaArgs),

    /// FJ-1437: Agent recipe registry
    #[command(name = "agent-registry")]
    AgentRegistry(AgentRegistryArgs),

    /// FJ-059+060: Pull agent / hybrid push-pull enforcement
    #[command(name = "agent")]
    PullAgent(PullAgentArgs),

    /// FJ-1450: Configuration complexity analysis
    Complexity(ComplexityArgs),
    /// FJ-1451: Dependency impact analysis
    Impact(ImpactArgs),
    /// FJ-1452: Configuration drift prediction
    #[command(name = "drift-predict")]
    DriftPredict(DriftPredictArgs),
}
