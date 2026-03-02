//! CLI Args structs for store-related commands (pin, cache, store, archive).

use clap::Subcommand;
use std::path::PathBuf;

/// Pin command: pin all inputs to current versions.
#[derive(clap::Args, Debug)]
pub struct PinArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// State directory
    #[arg(long, default_value = "state")]
    pub state_dir: PathBuf,

    /// Update a specific pin (or all if no name given)
    #[arg(long)]
    pub update: Option<Option<String>>,

    /// CI gate mode — fail if lock file is stale or incomplete
    #[arg(long)]
    pub check: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Cache subcommands: list, push, pull, verify.
#[derive(Subcommand, Debug)]
pub enum CacheCmd {
    /// List local store entries
    List {
        /// Store directory
        #[arg(long, default_value = "/var/lib/forjar/store")]
        store_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Push local store entries to an SSH remote cache
    Push {
        /// Remote cache identifier (user@host:path)
        remote: String,

        /// Store directory
        #[arg(long, default_value = "/var/lib/forjar/store")]
        store_dir: PathBuf,

        /// Only push specific hash
        #[arg(long)]
        hash: Option<String>,
    },

    /// Pull a specific store entry from a remote cache
    Pull {
        /// BLAKE3 hash of the store entry
        hash: String,

        /// Store directory
        #[arg(long, default_value = "/var/lib/forjar/store")]
        store_dir: PathBuf,
    },

    /// Verify all local store entries by re-hashing
    Verify {
        /// Store directory
        #[arg(long, default_value = "/var/lib/forjar/store")]
        store_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Store subcommands: gc, list, diff, sync.
#[derive(Subcommand, Debug)]
pub enum StoreCmd {
    /// Delete unreachable store entries (garbage collection)
    Gc {
        /// Store directory
        #[arg(long, default_value = "/var/lib/forjar/store")]
        store_dir: PathBuf,

        /// State directory (for lock file pins)
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Show what would be deleted without actually deleting
        #[arg(long)]
        dry_run: bool,

        /// Only delete entries older than N days
        #[arg(long)]
        older_than: Option<u64>,

        /// Keep last N profile generations (default: 5)
        #[arg(long, default_value = "5")]
        keep_generations: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List store entries with provenance info
    List {
        /// Store directory
        #[arg(long, default_value = "/var/lib/forjar/store")]
        store_dir: PathBuf,

        /// Show source provider for each entry
        #[arg(long)]
        show_provider: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Diff a store entry against its upstream origin
    Diff {
        /// BLAKE3 hash of the store entry to diff
        hash: String,

        /// Store directory
        #[arg(long, default_value = "/var/lib/forjar/store")]
        store_dir: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Re-import upstream and replay derivation chain
    Sync {
        /// BLAKE3 hash of the store entry to sync
        hash: String,

        /// Store directory
        #[arg(long, default_value = "/var/lib/forjar/store")]
        store_dir: PathBuf,

        /// Actually apply the sync (default: dry-run)
        #[arg(long)]
        apply: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Archive subcommands: pack, unpack, inspect, verify.
#[derive(Subcommand, Debug)]
pub enum ArchiveCmd {
    /// Pack a store entry into a .far archive
    Pack {
        /// BLAKE3 hash of the store entry to pack
        hash: String,

        /// Store directory
        #[arg(long, default_value = "/var/lib/forjar/store")]
        store_dir: PathBuf,

        /// Output file path (default: <hash>.far)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Unpack a .far archive into the store
    Unpack {
        /// Path to the .far file
        file: PathBuf,

        /// Store directory
        #[arg(long, default_value = "/var/lib/forjar/store")]
        store_dir: PathBuf,
    },

    /// Print the manifest of a .far archive without unpacking
    Inspect {
        /// Path to the .far file
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Verify chunk hashes and signature of a .far archive
    Verify {
        /// Path to the .far file
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Store import args: `forjar store-import <provider> <ref>`.
#[derive(clap::Args, Debug)]
pub struct StoreImportArgs {
    /// Provider name (apt, cargo, uv, nix, docker, tofu, terraform, apr)
    pub provider: String,

    /// Package/image/reference to import
    pub reference: String,

    /// Version pin
    #[arg(long)]
    pub version: Option<String>,

    /// Store directory
    #[arg(long, default_value = "/var/lib/forjar/store")]
    pub store_dir: std::path::PathBuf,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// List supported providers instead of importing
    #[arg(long)]
    pub list_providers: bool,
}

/// Convert subcommand args: --reproducible conversion.
#[derive(clap::Args, Debug)]
pub struct ConvertArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Enable reproducible conversion (steps 1-3)
    #[arg(long)]
    pub reproducible: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
