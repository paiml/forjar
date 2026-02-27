//! CLI Commands enum and sub-command enums.

mod apply_args;
mod validate_args;
mod plan_args;
mod status_args;
mod graph_args;
mod lock_core_args;
mod lock_ops_args;
mod state_args;
mod misc_args;
mod misc_ops_args;

pub use apply_args::*;
pub use validate_args::*;
pub use plan_args::*;
pub use status_args::*;
pub use graph_args::*;
pub use lock_core_args::*;
pub use lock_ops_args::*;
pub use state_args::*;
pub use misc_args::*;
pub use misc_ops_args::*;

use clap::Subcommand;
use std::path::PathBuf;


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

}


/// FJ-260: Snapshot subcommands — named state checkpoints.
#[derive(Subcommand, Debug)]
pub enum SnapshotCmd {
    /// Save current state as a named snapshot
    Save {
        /// Snapshot name
        name: String,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },

    /// List available snapshots
    List {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Restore state from a named snapshot
    Restore {
        /// Snapshot name to restore
        name: String,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },

    /// Delete a named snapshot
    Delete {
        /// Snapshot name to delete
        name: String,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },
}


/// Shell types for completion generation.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
}


/// FJ-210: Workspace subcommands.
#[derive(Subcommand, Debug)]
pub enum WorkspaceCmd {
    /// Create a new workspace
    New {
        /// Workspace name
        name: String,
    },

    /// List all workspaces
    List,

    /// Select (activate) a workspace
    Select {
        /// Workspace name to activate
        name: String,
    },

    /// Delete a workspace and its state
    Delete {
        /// Workspace name to delete
        name: String,

        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },

    /// Show current active workspace
    Current,
}


/// FJ-200: Secrets subcommands — age-encrypted secret management.
#[derive(Subcommand, Debug)]
pub enum SecretsCmd {
    /// Encrypt a value with age recipients
    Encrypt {
        /// Plaintext value to encrypt
        value: String,

        /// Age recipient public key (age1...)
        #[arg(short, long, required = true)]
        recipient: Vec<String>,
    },

    /// Decrypt an ENC[age,...] marker
    Decrypt {
        /// Encrypted marker (ENC[age,...])
        value: String,

        /// Path to age identity file
        #[arg(short, long)]
        identity: Option<PathBuf>,
    },

    /// Generate a new age identity (keypair)
    Keygen,

    /// Decrypt and display all secrets in a forjar.yaml
    View {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Path to age identity file
        #[arg(short, long)]
        identity: Option<PathBuf>,
    },

    /// Re-encrypt all ENC[age,...] markers with new recipients
    Rekey {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Path to current age identity file (for decryption)
        #[arg(short, long)]
        identity: Option<PathBuf>,

        /// New recipient public keys (age1...)
        #[arg(short, long, required = true)]
        recipient: Vec<String>,
    },

    /// FJ-201: Rotate all secrets — decrypt and re-encrypt with new keys
    Rotate {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Path to current age identity file (for decryption)
        #[arg(short, long)]
        identity: Option<PathBuf>,

        /// New recipient public keys (age1...)
        #[arg(short, long, required = true)]
        recipient: Vec<String>,

        /// Re-encrypt in place (required flag to prevent accidents)
        #[arg(long)]
        re_encrypt: bool,

        /// State directory for audit logging
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },
}

