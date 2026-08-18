//! The root clap parser — the single definition of forjar's command surface.
//!
//! # Why this lives in the library
//!
//! Before the Unified Verb Surface (GH-267) this struct was private to
//! `src/main.rs`. That made it invisible to anything that wanted to *describe*
//! the CLI rather than *be* the CLI, so every other surface — the pforge MCP
//! server, the docs, the parity tests — re-stated the command list by hand and
//! drifted from it.
//!
//! [`crate::verb`] derives the whole verb registry from `Cli::command()`. That
//! is the same [`clap::Command`] value `main` parses with, so a verb cannot
//! exist on one surface and not the other: there is one tree, and both the
//! parser and the registry read it.
//!
//! Keep this struct free of behaviour. It declares the surface; dispatch lives
//! in [`crate::cli::dispatch`].

use clap::Parser;

/// forjar's root command-line interface.
///
/// Global flags declared here apply to every subcommand and are visible to the
/// registry as globals rather than per-verb parameters.
///
/// `long_about = None` is load-bearing. clap's derive promotes a struct's doc
/// comment to `long_about`, which `--help` prefers over `about`. Without it,
/// moving this struct out of `main.rs` — where it had no doc comment — replaced
/// the product tagline in `forjar --help` with these three paragraphs and
/// switched every global flag to long-form layout. Caught by the 354-capture
/// output baseline, which is why that baseline exists.
#[derive(Parser, Debug)]
#[command(
    name = "forjar",
    version,
    long_about = None,
    about = "Rust-native Infrastructure as Code — bare-metal first, BLAKE3 state, provenance tracing"
)]
pub struct Cli {
    /// Increase verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: crate::cli::Commands,
}

#[cfg(test)]
mod tests {
    use crate::verb::derive::{check_argv, cli_command};

    // These tests go through `verb::derive`'s helpers rather than calling
    // `Cli::command()` directly. Building this tree needs more than the 2 MiB a
    // libtest thread gets, and under `panic = "abort"` the overflow is a
    // SIGABRT that takes down the whole test binary rather than one test.
    // `cli_command` and `check_argv` do the work on a thread sized for it.

    #[test]
    fn root_command_is_named_forjar() {
        assert_eq!(cli_command().get_name(), "forjar");
    }

    #[test]
    fn root_exposes_the_subcommand_tree() {
        // The registry depends on this being non-empty; if `command` ever stops
        // being a `#[command(subcommand)]` the derivation silently yields zero
        // verbs, and every downstream "all verbs agree" test passes vacuously.
        let cmd = cli_command();
        assert!(
            cmd.get_subcommands().count() > 100,
            "expected the full verb tree, got {}",
            cmd.get_subcommands().count()
        );
    }

    #[test]
    fn global_flags_are_marked_global() {
        let cmd = cli_command();
        for id in ["verbose", "no_color"] {
            let arg = cmd
                .get_arguments()
                .find(|a| a.get_id() == id)
                .unwrap_or_else(|| panic!("root arg {id} missing"));
            assert!(arg.is_global_set(), "{id} must be global");
        }
    }

    #[test]
    fn parses_a_subcommand_through_the_library_root() {
        check_argv(&[
            "validate".to_string(),
            "--file".to_string(),
            "x.yaml".to_string(),
        ])
        .expect("validate must parse through the library root");
    }

    #[test]
    fn the_long_about_override_keeps_the_product_tagline_in_help() {
        // Regression: this struct's doc comment became clap's `long_about` when
        // it moved out of main.rs, replacing the tagline in `forjar --help` and
        // switching every global flag to long-form layout. `long_about = None`
        // is what prevents that; without it `about` is not what --help shows.
        let cmd = cli_command();
        let about = cmd.get_about().map(|a| a.to_string()).unwrap_or_default();
        assert!(
            about.starts_with("Rust-native Infrastructure as Code"),
            "about must be the product tagline, got {about:?}"
        );
        assert!(
            cmd.get_long_about().is_none(),
            "long_about must stay unset or --help stops showing `about`"
        );
    }
}
