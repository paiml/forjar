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
        EnvironmentsCmd::Rollback {
            env,
            state_dir,
            generations,
            yes,
            json,
        } => cmd_environments_rollback(&env, &state_dir, generations, yes, json),
        EnvironmentsCmd::History {
            env,
            state_dir,
            limit,
            json,
        } => cmd_environments_history(&env, &state_dir, limit, json),
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

/// FJ-3508: Rollback an environment by logging a rollback event.
fn cmd_environments_rollback(
    env: &str,
    state_dir: &std::path::Path,
    generations: u32,
    yes: bool,
    json: bool,
) -> Result<(), String> {
    // GH-91: Warn that --yes auto-confirmation is not yet implemented
    if yes {
        eprintln!("Warning: --yes is not yet implemented for environment rollback. Flag ignored.");
    }

    let env_dir = state_dir.join(env);
    if !env_dir.exists() {
        return Err(format!(
            "no state directory for environment '{}' at {}",
            env,
            env_dir.display()
        ));
    }
    crate::core::promotion_events::log_rollback(
        state_dir,
        env,
        generations as usize,
        "manual rollback",
    )
    .map_err(|e| format!("log rollback: {e}"))?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "environment": env,
                "generations": generations,
                "action": "rollback",
            })
        );
    } else {
        println!(
            "Rollback logged for environment '{}' ({} generation(s))",
            env, generations
        );
    }
    Ok(())
}

/// FJ-3509: Show promotion/rollback history for an environment.
fn cmd_environments_history(
    env: &str,
    state_dir: &std::path::Path,
    limit: usize,
    json: bool,
) -> Result<(), String> {
    let events_path = state_dir.join(env).join("events.jsonl");
    if !events_path.exists() {
        if json {
            println!("[]");
        } else {
            println!("No history for environment '{}'", env);
        }
        return Ok(());
    }
    let content = std::fs::read_to_string(&events_path).map_err(|e| format!("read events: {e}"))?;
    let events: Vec<serde_json::Value> = content
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    let shown: Vec<_> = events.iter().rev().take(limit).collect();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&shown).unwrap_or_default()
        );
    } else {
        println!("History for '{}' (last {} events):", env, shown.len());
        for ev in &shown {
            let ts = ev.get("timestamp").and_then(|v| v.as_str()).unwrap_or("?");
            let et = ev.get("event_type").and_then(|v| v.as_str()).unwrap_or("?");
            let detail = ev
                .get("reason")
                .or_else(|| ev.get("source"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!("  {} | {:<25} | {}", ts, et, detail);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_logs_event() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("prod")).unwrap();
        assert!(cmd_environments_rollback("prod", dir.path(), 1, true, false).is_ok());
        let ev = std::fs::read_to_string(dir.path().join("prod/events.jsonl")).unwrap();
        assert!(ev.contains("rollback_triggered"));
    }

    #[test]
    fn rollback_missing_env() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_environments_rollback("missing", dir.path(), 1, true, false).is_err());
    }

    #[test]
    fn history_no_events() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_environments_history("dev", dir.path(), 10, false).is_ok());
    }

    #[test]
    fn history_with_events() {
        let dir = tempfile::tempdir().unwrap();
        let ed = dir.path().join("prod");
        std::fs::create_dir_all(&ed).unwrap();
        std::fs::write(
            ed.join("events.jsonl"),
            "{\"event_type\":\"promotion_completed\",\"timestamp\":\"T1\"}\n",
        )
        .unwrap();
        assert!(cmd_environments_history("prod", dir.path(), 10, false).is_ok());
    }

    #[test]
    fn history_json() {
        let dir = tempfile::tempdir().unwrap();
        let ed = dir.path().join("dev");
        std::fs::create_dir_all(&ed).unwrap();
        std::fs::write(ed.join("events.jsonl"), "{\"event_type\":\"e\"}\n").unwrap();
        assert!(cmd_environments_history("dev", dir.path(), 5, true).is_ok());
    }
}
