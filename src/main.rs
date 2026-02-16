//! Forjar CLI — Rust-native Infrastructure as Code.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "forjar",
    version,
    about = "Rust-native Infrastructure as Code — bare-metal first, BLAKE3 state, provenance tracing"
)]
struct Cli {
    #[command(subcommand)]
    command: forjar::cli::Commands,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = forjar::cli::dispatch(cli.command) {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}
