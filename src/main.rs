//! Forjar CLI — Rust-native Infrastructure as Code.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "forjar",
    version,
    about = "Rust-native Infrastructure as Code — bare-metal first, BLAKE3 state, provenance tracing"
)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: forjar::cli::Commands,
}

fn main() {
    let cli = Cli::parse();
    // --no-color is accepted for future colored output support.
    // Also honors NO_COLOR env per https://no-color.org/
    let no_color = cli.no_color || std::env::var("NO_COLOR").is_ok();
    if let Err(e) = forjar::cli::dispatch(cli.command, cli.verbose, no_color) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
