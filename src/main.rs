//! Forjar CLI — Rust-native Infrastructure as Code.

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use clap::Parser;
use forjar::cli::Cli;

/// FJ-2301: Structured exit codes.
///  0 — Success (all resources converged)
///  1 — General error (parse, validation, usage)
///  2 — Partial failure (some resources failed)
///  3 — Configuration error (invalid YAML, missing fields)
///  4 — Connection error (SSH, container transport)
/// 10 — Drift detected (non-zero diff in `forjar drift`)
fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

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
