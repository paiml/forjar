//! Tool, lint, and check command dispatch.
//!
//! Split out of `dispatch_misc` when that file crossed the repo's 500-line
//! ceiling. The two functions below are that file's `tools` group moved
//! verbatim — same bodies, same grouping, same `_b` continuation — so this is
//! a relocation, not a refactor.

use super::check::*;
use super::commands::*;
use super::doctor::*;
use super::infra::*;
use super::init::*;
use super::lint::*;
use super::observe::*;

/// Tool, lint, and check commands.
pub(super) fn dispatch_misc_tools(cmd: Commands, verbose: bool) -> Result<(), String> {
    match cmd {
        Commands::Check(CheckArgs {
            file,
            machine,
            resource,
            tag,
            state_dir,
            json,
        }) => cmd_check(
            &file,
            machine.as_deref(),
            resource.as_deref(),
            tag.as_deref(),
            &state_dir,
            json,
            verbose,
        ),
        Commands::Verify(args) => super::verify::cmd_verify(&args, verbose),
        Commands::Fmt(FmtArgs { file, check }) => cmd_fmt(&file, check),
        Commands::Lint(LintArgs {
            file,
            json,
            strict,
            fix,
            rules: _rules,
            bashrs_version,
        }) => {
            // GH-211: FJ-374 was destructured to `_rules` and dropped — rustc
            // silenced, the operator not. A custom rule file that is never
            // loaded means lint reports clean against rules it never ran. The
            // underscore binding is KEPT so the guard test still classifies
            // `rules` as unconsumed; the refusal below is its only reader.
            super::inert_flags::reject_inert_flag("--rules", _rules.is_some())?;
            if bashrs_version {
                // Version extracted from Cargo.toml dependency
                const BASHRS_VERSION: &str = "6.64.0";
                println!("bashrs {BASHRS_VERSION}");
                return Ok(());
            }
            cmd_lint(&file, json, strict, fix)
        }
        other => dispatch_misc_tools_b(other),
    }
}

/// Tool, lint, and check commands — second half of the same group.
pub(super) fn dispatch_misc_tools_b(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::Doctor(DoctorArgs {
            file,
            json,
            fix,
            network,
        }) => {
            if network {
                return cmd_doctor_network(file.as_deref(), json);
            }
            cmd_doctor(file.as_deref(), json, fix)
        }
        Commands::Mcp(McpArgs { schema }) => {
            if schema {
                cmd_mcp_schema()
            } else {
                cmd_mcp()
            }
        }
        Commands::Bench(BenchArgs {
            iterations,
            json,
            compare,
        }) => cmd_bench(iterations as usize, json, compare),
        Commands::Watch(WatchArgs {
            file,
            state_dir,
            interval,
            apply,
            yes,
        }) => cmd_watch(&file, &state_dir, interval, apply, yes),
        _ => unreachable!(),
    }
}
