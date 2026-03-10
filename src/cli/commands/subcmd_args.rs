//! Subcommand enums extracted from commands/mod.rs to keep it under 500 lines.

use clap::Subcommand;
use std::path::PathBuf;

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

/// FJ-1386: Generation subcommands — Nix-style generational state snapshots.
#[derive(Subcommand, Debug)]
pub enum GenerationCmd {
    /// List all state generations
    List {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Garbage-collect old generations
    Gc {
        /// Number of generations to keep
        #[arg(long, default_value = "5")]
        keep: u32,
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },
    /// FJ-2003: Diff two generations
    Diff {
        /// Source generation number
        from: u32,
        /// Target generation number
        to: u32,
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Shell types for completion generation.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum CompletionShell {
    /// Generate bash completions.
    Bash,
    /// Generate zsh completions.
    Zsh,
    /// Generate fish completions.
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
        /// Workspace name
        name: String,
    },
    /// Delete a workspace and its state
    Delete {
        /// Workspace name
        name: String,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
    /// Show current active workspace
    Current,
}

/// FJ-3500: Environment management subcommands.
#[derive(Subcommand, Debug)]
pub enum EnvironmentsCmd {
    /// List all defined environments
    List {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Diff two environments
    Diff {
        /// Source environment name
        source: String,
        /// Target environment name
        target: String,
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Rollback an environment to a previous generation
    Rollback {
        /// Environment name to rollback
        env: String,
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
        /// Number of generations to rollback
        #[arg(long, default_value = "1")]
        generations: u32,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Show promotion/rollback history for an environment
    History {
        /// Environment name
        env: String,
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
        /// Maximum events to show
        #[arg(long, default_value = "20")]
        limit: usize,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
}

/// FJ-3506: Promote between environments.
#[derive(clap::Args, Debug)]
pub struct PromoteArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,
    /// Target environment to promote to
    #[arg(short, long)]
    pub target: String,
    /// Skip approval prompt (use auto_approve from config)
    #[arg(long)]
    pub yes: bool,
    /// Dry-run: evaluate gates without applying
    #[arg(long)]
    pub dry_run: bool,
    /// JSON output
    #[arg(long)]
    pub json: bool,
}

/// FJ-3108: Rulebook management subcommands.
#[derive(Subcommand, Debug)]
pub enum RulesCmd {
    /// Validate rulebook YAML syntax and semantics
    Validate {
        /// Path to rulebook YAML file
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Show event type coverage across rulebooks
    Coverage {
        /// Path to rulebook YAML file
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
}

/// FJ-3403: Plugin management subcommands.
#[derive(Subcommand, Debug)]
pub enum PluginCmd {
    /// List installed plugins
    List {
        /// Plugin directory
        #[arg(long, default_value = "plugins")]
        plugin_dir: PathBuf,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Verify a plugin manifest and WASM binary
    Verify {
        /// Path to plugin manifest YAML
        manifest: PathBuf,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// FJ-3407: Scaffold a new plugin project
    Init {
        /// Plugin name
        name: String,
        /// Output directory (default: plugins/<name>)
        #[arg(long)]
        output: Option<PathBuf>,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Install a plugin from a local path or registry
    Install {
        /// Plugin source (directory path or name)
        source: String,
        /// Plugin directory to install into
        #[arg(long, default_value = "plugins")]
        plugin_dir: PathBuf,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Build a WASM plugin from a Rust source directory
    Build {
        /// Path to plugin source directory (must contain Cargo.toml)
        #[arg(long)]
        path: PathBuf,
        /// Output directory for built plugin
        #[arg(long, default_value = "plugins")]
        output: Option<PathBuf>,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
    /// Remove an installed plugin
    Remove {
        /// Plugin name to remove
        name: String,
        /// Plugin directory
        #[arg(long, default_value = "plugins")]
        plugin_dir: PathBuf,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
        /// JSON output
        #[arg(long)]
        json: bool,
    },
}

/// FJ-3304: State encryption arguments.
#[derive(clap::Args, Debug)]
pub struct StateEncryptArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,
    /// Passphrase for encryption (reads from stdin if not provided)
    #[arg(long)]
    pub passphrase: Option<String>,
    /// JSON output
    #[arg(long)]
    pub json: bool,
}

/// FJ-3304: State decryption arguments.
#[derive(clap::Args, Debug)]
pub struct StateDecryptArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,
    /// Passphrase for decryption
    #[arg(long)]
    pub passphrase: Option<String>,
    /// JSON output
    #[arg(long)]
    pub json: bool,
}

/// FJ-3309: State rekey arguments — re-encrypt with new passphrase.
#[derive(clap::Args, Debug)]
pub struct StateRekeyArgs {
    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,
    /// Current passphrase
    #[arg(long)]
    pub old_passphrase: Option<String>,
    /// New passphrase
    #[arg(long)]
    pub new_passphrase: Option<String>,
    /// JSON output
    #[arg(long)]
    pub json: bool,
}

/// FJ-200: Secrets subcommands — age-encrypted secret management.
#[derive(Subcommand, Debug)]
pub enum SecretsCmd {
    /// Encrypt a value with age recipients
    Encrypt {
        /// Plaintext value to encrypt
        value: String,
        /// Age recipient public key(s)
        #[arg(short, long, required = true)]
        recipient: Vec<String>,
    },
    /// Decrypt an ENC[age,...] marker
    Decrypt {
        /// Encrypted marker to decrypt
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
        /// Path to age identity file
        #[arg(short, long)]
        identity: Option<PathBuf>,
        /// Age recipient public key(s)
        #[arg(short, long, required = true)]
        recipient: Vec<String>,
    },
    /// FJ-201: Rotate all secrets — decrypt and re-encrypt with new keys
    Rotate {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,
        /// Path to age identity file
        #[arg(short, long)]
        identity: Option<PathBuf>,
        /// Age recipient public key(s)
        #[arg(short, long, required = true)]
        recipient: Vec<String>,
        /// Re-encrypt after rotation
        #[arg(long)]
        re_encrypt: bool,
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },
}
