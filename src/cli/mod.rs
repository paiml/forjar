//! FJ-017: CLI subcommands — plan, apply, drift, status, init, validate.

use crate::core::{executor, parser, planner, resolver, state};
use crate::tripwire::drift;
use clap::Subcommand;
use std::path::{Path, PathBuf};

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new forjar project
    Init {
        /// Directory to initialize (default: current)
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Validate forjar.yaml without connecting to machines
    Validate {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,
    },

    /// Show execution plan (diff desired vs current)
    Plan {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Target specific resource
        #[arg(short, long)]
        resource: Option<String>,
    },

    /// Converge infrastructure to desired state
    Apply {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Target specific resource
        #[arg(short, long)]
        resource: Option<String>,

        /// Force re-apply all resources
        #[arg(long)]
        force: bool,

        /// Show what would be executed without running
        #[arg(long)]
        dry_run: bool,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },

    /// Detect unauthorized changes (tripwire)
    Drift {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Exit non-zero on any drift (for CI/cron)
        #[arg(long)]
        tripwire: bool,
    },

    /// Show current state from lock files
    Status {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,
    },
}

/// Dispatch a CLI command.
pub fn dispatch(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::Init { path } => cmd_init(&path),
        Commands::Validate { file } => cmd_validate(&file),
        Commands::Plan {
            file,
            machine,
            resource,
        } => cmd_plan(&file, machine.as_deref(), resource.as_deref()),
        Commands::Apply {
            file,
            machine,
            resource,
            force,
            dry_run,
            state_dir,
        } => cmd_apply(
            &file,
            &state_dir,
            machine.as_deref(),
            resource.as_deref(),
            force,
            dry_run,
        ),
        Commands::Drift {
            file: _,
            machine,
            state_dir,
            tripwire,
        } => cmd_drift(&state_dir, machine.as_deref(), tripwire),
        Commands::Status {
            state_dir,
            machine,
        } => cmd_status(&state_dir, machine.as_deref()),
    }
}

fn cmd_init(path: &Path) -> Result<(), String> {
    let config_path = path.join("forjar.yaml");
    if config_path.exists() {
        return Err(format!("{} already exists", config_path.display()));
    }

    let state_dir = path.join("state");
    std::fs::create_dir_all(&state_dir)
        .map_err(|e| format!("cannot create state dir: {}", e))?;

    let template = r#"version: "1.0"
name: my-infrastructure
description: "Managed by forjar"

params: {}

machines: {}

resources: {}

policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#;
    std::fs::write(&config_path, template)
        .map_err(|e| format!("cannot write {}: {}", config_path.display(), e))?;

    println!("Initialized forjar project at {}", path.display());
    println!("  Created: {}", config_path.display());
    println!("  Created: {}/", state_dir.display());
    Ok(())
}

fn cmd_validate(file: &Path) -> Result<(), String> {
    let config = parser::parse_config_file(file)?;
    let errors = parser::validate_config(&config);

    if errors.is_empty() {
        println!(
            "OK: {} ({} machines, {} resources)",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
        Ok(())
    } else {
        for e in &errors {
            eprintln!("  ERROR: {}", e);
        }
        Err(format!("{} validation error(s)", errors.len()))
    }
}

fn cmd_plan(
    file: &Path,
    machine_filter: Option<&str>,
    _resource_filter: Option<&str>,
) -> Result<(), String> {
    let config = parser::parse_config_file(file)?;
    let errors = parser::validate_config(&config);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("  ERROR: {}", e);
        }
        return Err("validation failed".to_string());
    }

    let execution_order = resolver::build_execution_order(&config)?;
    let locks = std::collections::HashMap::new(); // TODO: load from state dir
    let plan = planner::plan(&config, &execution_order, &locks);

    println!("Planning: {} ({} resources)", plan.name, plan.changes.len());
    println!();

    let mut current_machine = String::new();
    for change in &plan.changes {
        if let Some(filter) = machine_filter {
            if change.machine != filter {
                continue;
            }
        }

        if change.machine != current_machine {
            current_machine = change.machine.clone();
            println!("{}:", current_machine);
        }

        let symbol = match change.action {
            crate::core::types::PlanAction::Create => "+",
            crate::core::types::PlanAction::Update => "~",
            crate::core::types::PlanAction::Destroy => "-",
            crate::core::types::PlanAction::NoOp => " ",
        };
        println!("  {} {}", symbol, change.description);
    }

    println!();
    println!(
        "Plan: {} to add, {} to change, {} to destroy, {} unchanged.",
        plan.to_create, plan.to_update, plan.to_destroy, plan.unchanged
    );

    Ok(())
}

fn cmd_apply(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    force: bool,
    dry_run: bool,
) -> Result<(), String> {
    let config = parser::parse_config_file(file)?;
    let errors = parser::validate_config(&config);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("  ERROR: {}", e);
        }
        return Err("validation failed".to_string());
    }

    let cfg = executor::ApplyConfig {
        config: &config,
        state_dir,
        force,
        dry_run,
        machine_filter,
        resource_filter,
    };

    let results = executor::apply(&cfg)?;

    if dry_run {
        println!("Dry run — no changes applied.");
        return Ok(());
    }

    let mut total_converged = 0;
    let mut total_unchanged = 0;
    let mut total_failed = 0;

    for result in &results {
        println!(
            "{}: {} converged, {} unchanged, {} failed ({:.1}s)",
            result.machine,
            result.resources_converged,
            result.resources_unchanged,
            result.resources_failed,
            result.total_duration.as_secs_f64()
        );
        total_converged += result.resources_converged;
        total_unchanged += result.resources_unchanged;
        total_failed += result.resources_failed;
    }

    println!();
    if total_failed > 0 {
        println!(
            "Apply completed with errors: {} converged, {} unchanged, {} FAILED",
            total_converged, total_unchanged, total_failed
        );
        return Err(format!("{} resource(s) failed", total_failed));
    }

    println!(
        "Apply complete: {} converged, {} unchanged.",
        total_converged, total_unchanged
    );
    Ok(())
}

fn cmd_drift(
    state_dir: &Path,
    machine_filter: Option<&str>,
    tripwire_mode: bool,
) -> Result<(), String> {
    // List machine directories in state/
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut total_drift = 0;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }

        if !entry.path().is_dir() {
            continue;
        }

        if let Some(lock) = state::load_lock(state_dir, &name)? {
            println!("Checking {} ({} resources)...", name, lock.resources.len());
            let findings = drift::detect_drift(&lock);

            if findings.is_empty() {
                println!("  No drift detected.");
            } else {
                for f in &findings {
                    println!(
                        "  DRIFTED: {} ({})",
                        f.resource_id, f.detail
                    );
                    println!("    Expected: {}", f.expected_hash);
                    println!("    Actual:   {}", f.actual_hash);
                }
                total_drift += findings.len();
            }
        }
    }

    if total_drift > 0 {
        println!();
        println!("Drift detected: {} resource(s)", total_drift);
        if tripwire_mode {
            return Err(format!("{} drift finding(s)", total_drift));
        }
    } else {
        println!("No drift detected.");
    }

    Ok(())
}

fn cmd_status(state_dir: &Path, machine_filter: Option<&str>) -> Result<(), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut found = false;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }

        if !entry.path().is_dir() {
            continue;
        }

        if let Some(lock) = state::load_lock(state_dir, &name)? {
            found = true;
            println!("Machine: {} ({})", lock.machine, lock.hostname);
            println!("  Generated: {}", lock.generated_at);
            println!("  Generator: {}", lock.generator);
            println!("  Resources: {}", lock.resources.len());

            for (id, rl) in &lock.resources {
                let duration = rl
                    .duration_seconds
                    .map(|d| format!(" ({:.2}s)", d))
                    .unwrap_or_default();
                println!("    {}: {} [{}]{}", id, rl.status, rl.resource_type, duration);
            }
            println!();
        }
    }

    if !found {
        println!("No state found. Run `forjar apply` first.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj017_init() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("test-project");
        std::fs::create_dir_all(&sub).unwrap();
        cmd_init(&sub).unwrap();
        assert!(sub.join("forjar.yaml").exists());
        assert!(sub.join("state").is_dir());
    }

    #[test]
    fn test_fj017_init_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("forjar.yaml"), "exists").unwrap();
        let result = cmd_init(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_fj017_validate_valid() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        cmd_validate(&config).unwrap();
    }

    #[test]
    fn test_fj017_validate_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "2.0"
name: ""
machines: {}
resources: {}
"#,
        )
        .unwrap();
        let result = cmd_validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj017_status_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("state")).unwrap();
        cmd_status(&dir.path().join("state"), None).unwrap();
    }

    #[test]
    fn test_fj017_plan() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        cmd_plan(&config, None, None).unwrap();
    }
}
