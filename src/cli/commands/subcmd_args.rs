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
