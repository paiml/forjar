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
/// 10 — Reserved (`ErrorClass::Drift`): nothing emits it. A `forjar drift`
///      verdict exits 1 on every run (PMAT-562); an unmeasured run exits 4.
///
/// The code comes from the error's CLASS — a variant of
/// `forjar::core::error::ErrorClass` — not from matching its text. `main` used
/// to substring-match the message, which sent every error whose prose merely
/// mentioned a transport out as 4 (retryable), including the deterministic I8
/// bashrs rejections that a retry can never fix. See `forjar::core::error`.
fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let cli = Cli::try_parse().unwrap_or_else(exit_redacted);
    let no_color = cli.no_color || std::env::var("NO_COLOR").is_ok();
    if let Err(e) = forjar::cli::dispatch_classified(cli.command, cli.verbose, no_color) {
        eprintln!("error: {e}");
        std::process::exit(e.exit_code());
    }
}

/// Exit on a usage error. clap quotes the offending argument in its message
/// and "tip" line; for `secrets encrypt` that argument can be the secret
/// itself, so this subcommand gets a fixed message that echoes nothing.
fn exit_redacted(e: clap::Error) -> Cli {
    // args_os + lossy: `args()` panics on a non-UTF-8 value and prints it.
    let argv: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    if e.use_stderr() && forjar::cli::argv_is_secrets_encrypt(&argv) {
        eprintln!(
            "error: invalid arguments for `forjar secrets encrypt` (not shown: it may hold a \
             secret). Usage: forjar secrets encrypt -r <RECIPIENT> [--from-file PATH]  \
             (plaintext is read from stdin)"
        );
        std::process::exit(2);
    }
    e.exit()
}
