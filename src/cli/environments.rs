//! FJ-3500: `forjar environments list|diff` CLI commands.

use super::commands::EnvironmentsCmd;
use super::helpers::parse_and_validate;

/// Dispatch environments subcommands.
pub(crate) fn dispatch_environments(cmd: EnvironmentsCmd) -> Result<(), String> {
    match cmd {
        EnvironmentsCmd::List { file, json } => cmd_environments_list(&file, json),
        EnvironmentsCmd::Diff {
            source,
            target,
            file,
            json,
        } => cmd_environments_diff(&file, &source, &target, json),
    }
}

/// List all environments defined in the config.
fn cmd_environments_list(file: &std::path::Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    if config.environments.is_empty() {
        if json {
            println!("[]");
        } else {
            println!("No environments defined in {}", file.display());
        }
        return Ok(());
    }

    if json {
        let envs: Vec<_> = config
            .environments
            .iter()
            .map(|(name, env)| {
                serde_json::json!({
                    "name": name,
                    "description": env.description,
                    "params": env.params.len(),
                    "machines": env.machines.len(),
                    "has_promotion": env.promotion.is_some(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&envs).unwrap_or_default()
        );
    } else {
        println!("Environments ({}):", config.environments.len());
        for (name, env) in &config.environments {
            let desc = env.description.as_deref().unwrap_or("(no description)");
            let promo = if let Some(ref p) = env.promotion {
                format!(" [promotes from: {}]", p.from)
            } else {
                String::new()
            };
            println!(
                "  {} — {} ({} params, {} machines){}",
                name,
                desc,
                env.params.len(),
                env.machines.len(),
                promo,
            );
        }
    }

    Ok(())
}

/// Diff two environments.
fn cmd_environments_diff(
    file: &std::path::Path,
    source: &str,
    target: &str,
    json: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let src_env = config
        .environments
        .get(source)
        .ok_or_else(|| format!("environment '{source}' not found"))?;
    let tgt_env = config
        .environments
        .get(target)
        .ok_or_else(|| format!("environment '{target}' not found"))?;

    let diff = crate::core::types::environment::diff_environments(
        source,
        src_env,
        target,
        tgt_env,
        &config.params,
        &config.machines,
    );

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&diff).unwrap_or_default()
        );
    } else if diff.is_identical() {
        println!("Environments '{source}' and '{target}' are identical.");
    } else {
        println!(
            "Diff: {} → {} ({} differences)",
            source,
            target,
            diff.total_diffs()
        );
        if !diff.param_diffs.is_empty() {
            println!("\n  Parameters:");
            for pd in &diff.param_diffs {
                let sv = pd.source_value.as_deref().unwrap_or("(unset)");
                let tv = pd.target_value.as_deref().unwrap_or("(unset)");
                println!("    {}: {} → {}", pd.key, sv, tv);
            }
        }
        if !diff.machine_diffs.is_empty() {
            println!("\n  Machines:");
            for md in &diff.machine_diffs {
                let sa = md.source_addr.as_deref().unwrap_or("(unset)");
                let ta = md.target_addr.as_deref().unwrap_or("(unset)");
                println!("    {}: {} → {}", md.machine, sa, ta);
            }
        }
    }

    Ok(())
}
