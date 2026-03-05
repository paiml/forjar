//! Forjar CLI — Rust-native Infrastructure as Code.

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
fn main() {
    let cli = Cli::parse();
    let no_color = cli.no_color || std::env::var("NO_COLOR").is_ok();
    if let Err(e) = forjar::cli::dispatch(cli.command, cli.verbose, no_color) {
        let code = classify_exit_code(&e);
        eprintln!("error: {e}");
        std::process::exit(code);
    }
}

fn classify_exit_code(error: &str) -> i32 {
    if error.contains("validation error") || error.contains("YAML parse error") {
        3
    } else if error.contains("SSH") || error.contains("connection") || error.contains("transport") {
        4
    } else if error.contains("partial") || error.contains("some resources failed") {
        2
    } else if error.contains("drift detected") {
        10
    } else {
        1
    }
}
