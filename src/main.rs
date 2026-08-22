//! Forjar CLI — Rust-native Infrastructure as Code.

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "forjar",
    version,
    about = "Rust-native Infrastructure as Code — bare-metal first, BLAKE3 state, provenance tracing"
)]
struct Cli {
    /// Increase verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: forjar::cli::Commands,
}

/// FJ-2301: Structured exit codes.
///  0 — Success (all resources converged)
///  1 — General error (parse, validation, usage)
///  2 — Partial failure (some resources failed)
///  3 — Configuration error (invalid YAML, missing fields)
///  4 — Connection error (SSH, container transport)
/// 10 — Drift detected (non-zero diff in `forjar drift`)
///
/// The code comes from the error's CLASS — a variant of
/// `forjar::core::error::ErrorClass` — not from matching its text. `main` used
/// to substring-match the message, which sent every error whose prose merely
/// mentioned a transport out as 4 (retryable), including the deterministic I8
/// bashrs rejections that a retry can never fix. See `forjar::core::error`.
fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let cli = Cli::parse();
    let no_color = cli.no_color || std::env::var("NO_COLOR").is_ok();
    if let Err(e) = forjar::cli::dispatch_classified(cli.command, cli.verbose, no_color) {
        eprintln!("error: {e}");
        std::process::exit(e.exit_code());
    }
}
