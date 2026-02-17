//! FJ-017: CLI subcommands — plan, apply, drift, status, init, validate.

use crate::core::{executor, parser, planner, resolver, state, types};
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

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
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
            state_dir,
        } => cmd_plan(&file, &state_dir, machine.as_deref(), resource.as_deref()),
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
        Commands::Status { state_dir, machine } => cmd_status(&state_dir, machine.as_deref()),
    }
}

fn cmd_init(path: &Path) -> Result<(), String> {
    let config_path = path.join("forjar.yaml");
    if config_path.exists() {
        return Err(format!("{} already exists", config_path.display()));
    }

    let state_dir = path.join("state");
    std::fs::create_dir_all(&state_dir).map_err(|e| format!("cannot create state dir: {}", e))?;

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
    state_dir: &Path,
    machine_filter: Option<&str>,
    _resource_filter: Option<&str>,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let execution_order = resolver::build_execution_order(&config)?;

    // Load existing locks so plan shows accurate Create vs Update vs NoOp
    let locks = load_machine_locks(&config, state_dir, machine_filter)?;
    let plan = planner::plan(&config, &execution_order, &locks);

    print_plan(&plan, machine_filter);
    Ok(())
}

/// Parse and validate a forjar config file, returning errors if invalid.
fn parse_and_validate(file: &Path) -> Result<types::ForjarConfig, String> {
    let config = parser::parse_config_file(file)?;
    let errors = parser::validate_config(&config);
    if errors.is_empty() {
        return Ok(config);
    }
    for e in &errors {
        eprintln!("  ERROR: {}", e);
    }
    Err("validation failed".to_string())
}

/// Load lock files for machines referenced in the config.
fn load_machine_locks(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<std::collections::HashMap<String, types::StateLock>, String> {
    let mut locks = std::collections::HashMap::new();
    if !state_dir.exists() {
        return Ok(locks);
    }
    for machine_name in config.machines.keys() {
        if let Some(filter) = machine_filter {
            if machine_name != filter {
                continue;
            }
        }
        if let Some(lock) = state::load_lock(state_dir, machine_name)? {
            locks.insert(machine_name.clone(), lock);
        }
    }
    Ok(locks)
}

/// Display a plan to stdout.
fn print_plan(plan: &types::ExecutionPlan, machine_filter: Option<&str>) {
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
            current_machine.clone_from(&change.machine);
            println!("{}:", current_machine);
        }
        let symbol = match change.action {
            types::PlanAction::Create => "+",
            types::PlanAction::Update => "~",
            types::PlanAction::Destroy => "-",
            types::PlanAction::NoOp => " ",
        };
        println!("  {} {}", symbol, change.description);
    }

    println!();
    println!(
        "Plan: {} to add, {} to change, {} to destroy, {} unchanged.",
        plan.to_create, plan.to_update, plan.to_destroy, plan.unchanged
    );
}

fn cmd_apply(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    force: bool,
    dry_run: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

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
                    println!("  DRIFTED: {} ({})", f.resource_id, f.detail);
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
                println!(
                    "    {}: {} [{}]{}",
                    id, rl.status, rl.resource_type, duration
                );
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
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        cmd_plan(&config, &state, None, None).unwrap();
    }

    #[test]
    fn test_fj017_plan_with_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 1.1.1.1
  b:
    hostname: b
    addr: 2.2.2.2
resources:
  pkg-a:
    type: package
    machine: a
    provider: apt
    packages: [curl]
  pkg-b:
    type: package
    machine: b
    provider: apt
    packages: [wget]
"#,
        )
        .unwrap();
        cmd_plan(&config, &state, Some("a"), None).unwrap();
    }

    #[test]
    fn test_fj017_plan_validation_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
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
        let result = cmd_plan(&config, &state, None, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("validation"));
    }

    #[test]
    fn test_fj017_apply_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-cli-dry-run.txt
    content: "test"
"#,
        )
        .unwrap();
        cmd_apply(&config, &state, None, None, false, true).unwrap();
    }

    #[test]
    fn test_fj017_apply_real() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-cli-apply-test.txt
    content: "hello from cli test"
policy:
  tripwire: true
  lock_file: true
"#,
        )
        .unwrap();
        cmd_apply(&config, &state, None, None, false, false).unwrap();

        // Verify file was created
        assert!(std::path::Path::new("/tmp/forjar-cli-apply-test.txt").exists());

        // Verify lock was saved
        let lock = crate::core::state::load_lock(&state, "local").unwrap();
        assert!(lock.is_some());

        let _ = std::fs::remove_file("/tmp/forjar-cli-apply-test.txt");
    }

    #[test]
    fn test_fj017_apply_validation_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
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
        let result = cmd_apply(&config, &state, None, None, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("validation"));
    }

    #[test]
    fn test_fj017_drift_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        cmd_drift(&state, None, false).unwrap();
    }

    #[test]
    fn test_fj017_drift_with_lock() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        // Create a lock with a file resource
        let test_file = dir.path().join("tracked.txt");
        std::fs::write(&test_file, "stable content").unwrap();
        let hash = crate::tripwire::hasher::hash_file(&test_file).unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(test_file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(hash),
        );
        resources.insert(
            "tracked-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: "blake3:x".to_string(),
                details,
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "testbox".to_string(),
            hostname: "testbox".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // No drift expected
        cmd_drift(&state, None, false).unwrap();
    }

    #[test]
    fn test_fj017_drift_with_actual_drift_tripwire() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let test_file = dir.path().join("drifted.txt");
        std::fs::write(&test_file, "original").unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(test_file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:wrong_hash".to_string()),
        );
        resources.insert(
            "drifted-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: "blake3:x".to_string(),
                details,
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "driftbox".to_string(),
            hostname: "driftbox".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // Tripwire mode should error on drift
        let result = cmd_drift(&state, None, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("drift"));
    }

    #[test]
    fn test_fj017_drift_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(state.join("alpha")).unwrap();
        std::fs::create_dir_all(state.join("beta")).unwrap();

        cmd_drift(&state, Some("alpha"), false).unwrap();
    }

    #[test]
    fn test_fj017_status_with_lock() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let lock = crate::core::state::new_lock("mybox", "mybox-host");
        crate::core::state::save_lock(&state, &lock).unwrap();

        cmd_status(&state, None).unwrap();
    }

    #[test]
    fn test_fj017_status_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let lock = crate::core::state::new_lock("target", "target-host");
        crate::core::state::save_lock(&state, &lock).unwrap();

        cmd_status(&state, Some("target")).unwrap();
        cmd_status(&state, Some("nonexistent")).unwrap();
    }

    #[test]
    fn test_fj017_dispatch_init() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("dispatch-test");
        std::fs::create_dir_all(&sub).unwrap();
        dispatch(Commands::Init { path: sub.clone() }).unwrap();
        assert!(sub.join("forjar.yaml").exists());
    }

    #[test]
    fn test_fj017_dispatch_validate() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#,
        )
        .unwrap();
        dispatch(Commands::Validate {
            file: config.clone(),
        })
        .unwrap();
    }

    #[test]
    fn test_fj017_dispatch_status() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(Commands::Status {
            state_dir: state,
            machine: None,
        })
        .unwrap();
    }

    #[test]
    fn test_fj017_dispatch_plan() {
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
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(Commands::Plan {
            file: config,
            machine: None,
            resource: None,
            state_dir: state,
        })
        .unwrap();
    }

    #[test]
    fn test_fj017_dispatch_apply_dry() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: /tmp/forjar-dispatch-dry.txt
    content: "x"
"#,
        )
        .unwrap();
        dispatch(Commands::Apply {
            file: config,
            machine: None,
            resource: None,
            force: false,
            dry_run: true,
            state_dir: state,
        })
        .unwrap();
    }

    #[test]
    fn test_fj017_dispatch_drift() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(Commands::Drift {
            file: dir.path().join("forjar.yaml"),
            machine: None,
            state_dir: state,
            tripwire: false,
        })
        .unwrap();
    }

    #[test]
    fn test_fj017_status_with_resources_and_duration() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "web-pkg".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::Package,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-02-16T14:00:00Z".to_string()),
                duration_seconds: Some(2.34),
                hash: "blake3:abc".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        resources.insert(
            "web-svc".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::Service,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-02-16T14:00:01Z".to_string()),
                duration_seconds: None, // no duration — exercises unwrap_or_default branch
                hash: "blake3:def".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "webbox".to_string(),
            hostname: "webbox.example.com".to_string(),
            generated_at: "2026-02-16T14:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // Exercises the full resource iteration path with duration display
        cmd_status(&state, None).unwrap();
    }

    #[test]
    fn test_fj017_status_dir_with_non_dir_entry() {
        // Tests the `!entry.path().is_dir()` skip path
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        // Create a regular file inside state/ — should be skipped
        std::fs::write(state.join("not-a-machine"), "junk").unwrap();
        cmd_status(&state, None).unwrap();
    }

    #[test]
    fn test_fj017_drift_no_tripwire_still_reports() {
        // Exercises the total_drift > 0 && !tripwire_mode path (Ok, not Err)
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let test_file = dir.path().join("drifted2.txt");
        std::fs::write(&test_file, "current").unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(test_file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:mismatched".to_string()),
        );
        resources.insert(
            "drifted-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: "blake3:x".to_string(),
                details,
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "driftbox2".to_string(),
            hostname: "driftbox2".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // tripwire_mode=false: drift detected but should still be Ok(())
        cmd_drift(&state, None, false).unwrap();
    }

    #[test]
    fn test_fj017_apply_with_results_summary() {
        // Tests the full apply path with real local execution, covering the
        // results iteration and summary output lines
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir.path().join("apply-summary.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  summary-file:
    type: file
    machine: local
    path: {}
    content: "summary test"
"#,
                target.display()
            ),
        )
        .unwrap();

        cmd_apply(&config, &state, None, None, false, false).unwrap();
        assert!(target.exists());

        // Second apply — should be unchanged (NoOp)
        cmd_apply(&config, &state, None, None, false, false).unwrap();
    }

    #[test]
    fn test_fj017_load_machine_locks_missing_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = serde_yaml_ng::from_str::<types::ForjarConfig>(
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources: {}
"#,
        )
        .unwrap();
        // State dir doesn't exist → returns empty map
        let missing = dir.path().join("nonexistent");
        let locks = load_machine_locks(&config, &missing, None).unwrap();
        assert!(locks.is_empty());
    }

    #[test]
    fn test_fj017_print_plan_update_and_destroy_symbols() {
        // Exercises the Update (~) and Destroy (-) match arms in print_plan
        let plan = types::ExecutionPlan {
            name: "symbol-test".to_string(),
            changes: vec![
                types::PlannedChange {
                    resource_id: "r1".to_string(),
                    machine: "m1".to_string(),
                    resource_type: types::ResourceType::File,
                    action: types::PlanAction::Update,
                    description: "update /etc/conf".to_string(),
                },
                types::PlannedChange {
                    resource_id: "r2".to_string(),
                    machine: "m1".to_string(),
                    resource_type: types::ResourceType::File,
                    action: types::PlanAction::Destroy,
                    description: "destroy /tmp/old".to_string(),
                },
            ],
            execution_order: vec!["r1".to_string(), "r2".to_string()],
            to_create: 0,
            to_update: 1,
            to_destroy: 1,
            unchanged: 0,
        };
        // Just verify it doesn't panic — output goes to stdout
        print_plan(&plan, None);
    }

    #[test]
    fn test_fj017_plan_nonexistent_state_dir() {
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
        // Plan with nonexistent state dir → everything shows as Create
        let missing = dir.path().join("no-state");
        cmd_plan(&config, &missing, None, None).unwrap();
    }
}
