//! Misc command dispatch part C — import, scoring, and operations commands.

use super::commands::*;
use super::fleet_ops::*;
use super::fleet_reporting::*;
use super::import_cmd::*;
use super::score::*;
use super::show::*;

/// Import, export, operations, and scoring commands.
pub(super) fn dispatch_misc_ops(cmd: Commands, verbose: bool) -> Result<(), String> {
    match cmd {
        Commands::Import(ImportArgs {
            addr,
            user,
            name,
            output,
            scan,
            smart,
        }) => cmd_import(
            &addr,
            &user,
            name.as_deref(),
            &output,
            &scan,
            verbose,
            smart,
        ),
        Commands::Suggest(SuggestArgs { file, json }) => cmd_suggest(&file, json),
        Commands::Template(TemplateArgs { recipe, vars, json }) => {
            cmd_template(&recipe, &vars, json)
        }
        Commands::Score(ScoreArgs {
            file,
            status,
            idempotency,
            budget_ms,
            json,
            state_dir,
        }) => cmd_score(&file, &status, &idempotency, budget_ms, json, &state_dir),
        Commands::ConfigMerge(ConfigMergeArgs {
            file_a,
            file_b,
            output,
            allow_collisions,
        }) => super::config_merge::cmd_config_merge(
            &file_a,
            &file_b,
            output.as_deref(),
            allow_collisions,
        ),
        Commands::Extract(ExtractArgs {
            file,
            tags,
            group,
            glob,
            output,
            json,
        }) => super::extract::cmd_extract(
            &file,
            tags.as_deref(),
            group.as_deref(),
            glob.as_deref(),
            output.as_deref(),
            json,
        ),
        Commands::Inventory(InventoryArgs { file, json }) => cmd_inventory(&file, json),
        Commands::Output(OutputArgs { file, key, json }) => cmd_output(&file, key.as_deref(), json),
        Commands::Policy(PolicyArgs { file, json, sarif }) => cmd_policy(&file, json, sarif),
        Commands::PolicyCoverage(PolicyCoverageArgs { file, json }) => {
            super::policy_coverage::cmd_policy_coverage(&file, json)
        }
        Commands::PolicyInstall(a) => {
            super::policy_install::cmd_policy_install(&a.pack, &a.output_dir, a.json)
        }
        _ => unreachable!(),
    }
}
