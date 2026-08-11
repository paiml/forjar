//! CLI argument structs for artifact generation:
//! distribution artifacts (`dist`, FJ-3600) and
//! autoinstall images (`image`, FJ-52).

use std::path::PathBuf;

/// FJ-3600: CLI arguments for `dist` (distribution artifact generation).
#[derive(clap::Args, Debug)]
pub struct DistArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Generate shell installer script
    #[arg(long)]
    pub installer: bool,

    /// Generate Homebrew formula
    #[arg(long)]
    pub homebrew: bool,

    /// Generate cargo-binstall metadata
    #[arg(long)]
    pub binstall: bool,

    /// Generate Nix flake
    #[arg(long)]
    pub nix: bool,

    /// Generate GitHub Actions setup action
    #[arg(long)]
    pub github_action: bool,

    /// Generate Debian package spec
    #[arg(long)]
    pub deb: bool,

    /// Generate RPM spec file
    #[arg(long)]
    pub rpm: bool,

    /// Generate all distribution artifacts
    #[arg(long)]
    pub all: bool,

    /// PMAT-082/FJ-3607 Tier 1: generate artifacts to a temp dir and
    /// statically verify the installer (sh -n, bashrs lint, required
    /// snippets, download URL structure) instead of writing them
    #[arg(long)]
    pub verify: bool,

    /// FJ-3607 Tier 2: actually RUN the generated installer in ubuntu
    /// (gnu) and alpine (musl) containers against a locally-staged
    /// tarball, asserting the binary lands in install_dir and
    /// version_cmd succeeds. Requires Docker/Podman; degrades to a clean
    /// skip when no container runtime is available. Implies --verify.
    #[arg(long)]
    pub verify_containers: bool,

    /// Release tag to pin (e.g., v1.4.3) — required for artifacts that
    /// embed real checksums (--homebrew, --nix)
    #[arg(long, value_name = "TAG")]
    pub version: Option<String>,

    /// Local SHA256SUMS-format file to resolve checksums offline
    /// (instead of fetching from the GitHub release)
    #[arg(long, value_name = "PATH")]
    pub checksums_file: Option<PathBuf>,

    /// Output file when exactly one artifact is selected; output directory
    /// when several are (an alias for --output-dir). Mutually exclusive
    /// with --output-dir.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Output directory for every generated artifact (default: dist/)
    #[arg(long)]
    pub output_dir: Option<PathBuf>,

    /// JSON output (manifest of generated artifacts)
    #[arg(long)]
    pub json: bool,
}

/// FJ-52: CLI arguments for `image` (autoinstall ISO generation).
#[derive(clap::Args, Debug)]
pub struct ImageArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Target machine name from machines: section
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Generate user-data only (for manual ISO build or PXE)
    #[arg(long)]
    pub user_data: bool,

    /// Generate Android Magisk module ZIP (experimental)
    #[arg(long)]
    pub android: bool,

    /// Path to base Ubuntu ISO (enables ISO generation)
    #[arg(long)]
    pub base: Option<PathBuf>,

    /// Output path (ISO file or user-data YAML)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Disk layout: auto-lvm, auto-zfs, or device path (default: auto-lvm)
    #[arg(long, default_value = "auto-lvm")]
    pub disk: String,

    /// Locale (default: en_US.UTF-8)
    #[arg(long, default_value = "en_US.UTF-8")]
    pub locale: String,

    /// Timezone (default: UTC)
    #[arg(long, default_value = "UTC")]
    pub timezone: String,

    /// JSON output
    #[arg(long)]
    pub json: bool,
}
