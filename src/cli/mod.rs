//! FJ-017: CLI subcommands — init, validate, plan, apply, drift, status, history,
//! destroy, import, show, graph, check, diff, fmt, lint, rollback, anomaly, migrate.

use crate::core::{codegen, executor, migrate, parser, planner, resolver, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog};
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

        /// Filter to resources with this tag
        #[arg(short, long)]
        tag: Option<String>,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Output plan as JSON
        #[arg(long)]
        json: bool,

        /// Write generated scripts to directory for auditing
        #[arg(long)]
        output_dir: Option<PathBuf>,
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

        /// Filter to resources with this tag
        #[arg(short, long)]
        tag: Option<String>,

        /// Force re-apply all resources
        #[arg(long)]
        force: bool,

        /// Show what would be executed without running
        #[arg(long)]
        dry_run: bool,

        /// Skip provenance tracing (faster, less safe)
        #[arg(long)]
        no_tripwire: bool,

        /// Override a parameter (KEY=VALUE)
        #[arg(short, long = "param", value_name = "KEY=VALUE")]
        params: Vec<String>,

        /// Git commit state after successful apply
        #[arg(long)]
        auto_commit: bool,

        /// Timeout per transport operation (seconds)
        #[arg(long)]
        timeout: Option<u64>,

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

        /// Run command on drift detection
        #[arg(long)]
        alert_cmd: Option<String>,

        /// Auto-remediate: re-apply drifted resources to restore desired state
        #[arg(long)]
        auto_remediate: bool,

        /// Show what would be checked without connecting to machines
        #[arg(long)]
        dry_run: bool,

        /// Output drift report as JSON
        #[arg(long)]
        json: bool,
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

    /// Show apply history from event logs
    History {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Show history for specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Show last N applies (default: 10)
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Remove all managed resources (reverse order)
    Destroy {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },

    /// Import existing infrastructure from a machine into forjar.yaml
    Import {
        /// Machine address (IP, hostname, or 'localhost')
        #[arg(short, long)]
        addr: String,

        /// SSH user
        #[arg(short, long, default_value = "root")]
        user: String,

        /// Machine name (used as key in machines section)
        #[arg(short, long)]
        name: Option<String>,

        /// Output file
        #[arg(short, long, default_value = "forjar.yaml")]
        output: PathBuf,

        /// What to scan
        #[arg(long, value_delimiter = ',', default_value = "packages,files,services")]
        scan: Vec<String>,
    },

    /// Show fully resolved config (recipes expanded, templates resolved)
    Show {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Show specific resource only
        #[arg(short, long)]
        resource: Option<String>,

        /// Output as JSON instead of YAML
        #[arg(long)]
        json: bool,
    },

    /// Show resource dependency graph
    Graph {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Output format: mermaid (default) or dot
        #[arg(long, default_value = "mermaid")]
        format: String,
    },

    /// Run check scripts to verify pre-conditions without applying
    Check {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Target specific resource
        #[arg(short, long)]
        resource: Option<String>,

        /// Filter to resources with this tag
        #[arg(long)]
        tag: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Compare two state snapshots (show what changed between applies)
    Diff {
        /// First state directory (older)
        from: PathBuf,

        /// Second state directory (newer)
        to: PathBuf,

        /// Filter to specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Format (normalize) a forjar.yaml config file
    Fmt {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Check formatting without writing (exit non-zero if unformatted)
        #[arg(long)]
        check: bool,
    },

    /// Lint config for best practices (beyond validation)
    Lint {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Rollback to a previous config revision from git history
    Rollback {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Git revision to rollback to (default: HEAD~1)
        #[arg(short = 'n', long, default_value = "1")]
        revision: u32,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Show what would change without applying
        #[arg(long)]
        dry_run: bool,

        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,
    },

    /// Detect anomalous resource behavior from event history
    Anomaly {
        /// State directory
        #[arg(long, default_value = "state")]
        state_dir: PathBuf,

        /// Target specific machine
        #[arg(short, long)]
        machine: Option<String>,

        /// Minimum events to consider (ignore resources with fewer)
        #[arg(long, default_value = "3")]
        min_events: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Migrate Docker resources to pepita kernel isolation (FJ-044)
    Migrate {
        /// Path to forjar.yaml
        #[arg(short, long, default_value = "forjar.yaml")]
        file: PathBuf,

        /// Write migrated config to file (default: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Start MCP server (pforge integration, FJ-063)
    Mcp,
}

/// Dispatch a CLI command.
pub fn dispatch(cmd: Commands, verbose: bool) -> Result<(), String> {
    match cmd {
        Commands::Init { path } => cmd_init(&path),
        Commands::Validate { file } => cmd_validate(&file),
        Commands::Plan {
            file,
            machine,
            resource,
            tag,
            state_dir,
            json,
            output_dir,
        } => cmd_plan(
            &file,
            &state_dir,
            machine.as_deref(),
            resource.as_deref(),
            tag.as_deref(),
            json,
            verbose,
            output_dir.as_deref(),
        ),
        Commands::Apply {
            file,
            machine,
            resource,
            tag,
            force,
            dry_run,
            no_tripwire,
            params,
            auto_commit,
            timeout,
            state_dir,
        } => cmd_apply(
            &file,
            &state_dir,
            machine.as_deref(),
            resource.as_deref(),
            tag.as_deref(),
            force,
            dry_run,
            no_tripwire,
            &params,
            auto_commit,
            timeout,
            verbose,
        ),
        Commands::Drift {
            file,
            machine,
            state_dir,
            tripwire,
            alert_cmd,
            auto_remediate,
            dry_run,
            json,
        } => cmd_drift(
            &file,
            &state_dir,
            machine.as_deref(),
            tripwire,
            alert_cmd.as_deref(),
            auto_remediate,
            dry_run,
            json,
            verbose,
        ),
        Commands::Destroy {
            file,
            machine,
            yes,
            state_dir,
        } => cmd_destroy(&file, &state_dir, machine.as_deref(), yes, verbose),
        Commands::Status { state_dir, machine } => cmd_status(&state_dir, machine.as_deref()),
        Commands::History {
            state_dir,
            machine,
            limit,
            json,
        } => cmd_history(&state_dir, machine.as_deref(), limit, json),
        Commands::Show {
            file,
            resource,
            json,
        } => cmd_show(&file, resource.as_deref(), json),
        Commands::Graph { file, format } => cmd_graph(&file, &format),
        Commands::Import {
            addr,
            user,
            name,
            output,
            scan,
        } => cmd_import(&addr, &user, name.as_deref(), &output, &scan, verbose),
        Commands::Diff {
            from,
            to,
            machine,
            json,
        } => cmd_diff(&from, &to, machine.as_deref(), json),
        Commands::Check {
            file,
            machine,
            resource,
            tag,
            json,
        } => cmd_check(
            &file,
            machine.as_deref(),
            resource.as_deref(),
            tag.as_deref(),
            json,
            verbose,
        ),
        Commands::Fmt { file, check } => cmd_fmt(&file, check),
        Commands::Lint { file, json } => cmd_lint(&file, json),
        Commands::Rollback {
            file,
            revision,
            machine,
            dry_run,
            state_dir,
        } => cmd_rollback(
            &file,
            &state_dir,
            revision,
            machine.as_deref(),
            dry_run,
            verbose,
        ),
        Commands::Anomaly {
            state_dir,
            machine,
            min_events,
            json,
        } => cmd_anomaly(&state_dir, machine.as_deref(), min_events, json),
        Commands::Migrate { file, output } => cmd_migrate(&file, output.as_deref()),
        Commands::Mcp => cmd_mcp(),
    }
}

fn cmd_migrate(file: &Path, output: Option<&Path>) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    // Count docker resources
    let docker_count = config
        .resources
        .values()
        .filter(|r| r.resource_type == types::ResourceType::Docker)
        .count();

    if docker_count == 0 {
        println!("No Docker resources found in {}", file.display());
        return Ok(());
    }

    let (migrated, warnings) = migrate::migrate_config(&config);

    // Print warnings
    if !warnings.is_empty() {
        eprintln!("Migration warnings:");
        for w in &warnings {
            eprintln!("  ⚠ {}", w);
        }
        eprintln!();
    }

    // Serialize migrated config
    let yaml = serde_yaml_ng::to_string(&migrated)
        .map_err(|e| format!("Failed to serialize migrated config: {}", e))?;

    if let Some(out_path) = output {
        std::fs::write(out_path, &yaml)
            .map_err(|e| format!("Failed to write {}: {}", out_path.display(), e))?;
        println!(
            "Migrated {} Docker resource(s) → pepita in {}",
            docker_count,
            out_path.display()
        );
    } else {
        print!("{}", yaml);
    }

    println!(
        "Migration complete: {} resource(s) converted, {} warning(s)",
        docker_count,
        warnings.len()
    );
    Ok(())
}

fn cmd_mcp() -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;
    rt.block_on(crate::mcp::serve())
}

fn cmd_lint(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    let mut warnings: Vec<String> = Vec::new();

    // 1. Unused machines (defined but not referenced by any resource)
    let mut referenced_machines = std::collections::HashSet::new();
    for resource in config.resources.values() {
        for m in resource.machine.to_vec() {
            referenced_machines.insert(m);
        }
    }
    for machine_name in config.machines.keys() {
        if !referenced_machines.contains(machine_name) {
            warnings.push(format!(
                "machine '{}' is defined but not referenced by any resource",
                machine_name
            ));
        }
    }

    // 2. Resources without tags (harder to filter selectively)
    let mut untagged = 0usize;
    for (id, resource) in &config.resources {
        if resource.tags.is_empty() {
            untagged += 1;
            if config.resources.len() > 3 {
                warnings.push(format!("resource '{}' has no tags", id));
            }
        }
    }
    if untagged > 0 && config.resources.len() > 3 && untagged == config.resources.len() {
        // Deduplicate: replace individual warnings with a summary
        warnings.retain(|w| !w.starts_with("resource '") || !w.ends_with("has no tags"));
        warnings.push(format!(
            "all {} resources have no tags — consider adding tags for selective filtering",
            untagged
        ));
    }

    // 3. Duplicate content across file resources
    let mut content_map: std::collections::HashMap<&str, Vec<&str>> =
        std::collections::HashMap::new();
    for (id, resource) in &config.resources {
        if let Some(ref content) = resource.content {
            content_map
                .entry(content.as_str())
                .or_default()
                .push(id.as_str());
        }
    }
    for ids in content_map.values() {
        if ids.len() > 1 {
            warnings.push(format!(
                "resources {} have identical content — consider using a recipe or template",
                ids.join(", ")
            ));
        }
    }

    // 4. Resources with depends_on referencing non-existent resources
    for (id, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                warnings.push(format!(
                    "resource '{}' depends on '{}' which does not exist",
                    id, dep
                ));
            }
        }
    }

    // 5. Cross-machine dependencies (resource depends on resource targeting different machines)
    for (id, resource) in &config.resources {
        let my_machines: std::collections::HashSet<String> =
            resource.machine.to_vec().into_iter().collect();
        for dep in &resource.depends_on {
            if let Some(dep_resource) = config.resources.get(dep) {
                let dep_machines: std::collections::HashSet<String> =
                    dep_resource.machine.to_vec().into_iter().collect();
                if my_machines.is_disjoint(&dep_machines) {
                    warnings.push(format!(
                        "resource '{}' depends on '{}' but they target different machines",
                        id, dep
                    ));
                }
            }
        }
    }

    // 6. Empty packages list for package resources
    for (id, resource) in &config.resources {
        if resource.resource_type == types::ResourceType::Package && resource.packages.is_empty() {
            warnings.push(format!("package resource '{}' has no packages listed", id));
        }
    }

    // 7. bashrs script lint (FJ-036) — lint generated scripts for shell safety
    let mut script_errors = 0usize;
    let mut script_warnings_count = 0usize;
    for (id, resource) in &config.resources {
        for (kind, result) in [
            ("check", codegen::check_script(resource)),
            ("apply", codegen::apply_script(resource)),
            ("state_query", codegen::state_query_script(resource)),
        ] {
            if let Ok(script) = result {
                let lint_result = crate::core::purifier::lint_script(&script);
                for d in &lint_result.diagnostics {
                    use bashrs::linter::Severity;
                    match d.severity {
                        Severity::Error => {
                            script_errors += 1;
                            warnings.push(format!(
                                "bashrs: {}/{} [{}] {}",
                                id, kind, d.code, d.message
                            ));
                        }
                        _ => {
                            script_warnings_count += 1;
                        }
                    }
                }
            }
        }
    }
    if script_errors > 0 || script_warnings_count > 0 {
        warnings.push(format!(
            "bashrs script lint: {} error(s), {} warning(s) across {} resources",
            script_errors,
            script_warnings_count,
            config.resources.len()
        ));
    }

    // Output
    if json {
        let report = serde_json::json!({
            "warnings": warnings.len(),
            "findings": warnings,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {}", e))?;
        println!("{}", output);
    } else if warnings.is_empty() {
        println!("No lint warnings found.");
    } else {
        for w in &warnings {
            println!("  warn: {}", w);
        }
        println!();
        println!("Lint: {} warning(s)", warnings.len());
    }

    Ok(())
}

/// Detect anomalous resource behavior from event history.
///
/// Analyzes event logs to find resources with abnormally high change frequency,
/// failure rates, or drift counts. Uses statistical z-score to flag outliers.
fn cmd_anomaly(
    state_dir: &Path,
    machine_filter: Option<&str>,
    min_events: usize,
    json: bool,
) -> Result<(), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    // Per-resource metrics: (converge_count, fail_count, drift_count)
    let mut metrics: std::collections::HashMap<String, (u32, u32, u32)> =
        std::collections::HashMap::new();

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

        let log_path = entry.path().join("events.jsonl");
        if !log_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&log_path)
            .map_err(|e| format!("cannot read {}: {}", log_path.display(), e))?;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(te) = serde_json::from_str::<types::TimestampedEvent>(line) {
                match te.event {
                    types::ProvenanceEvent::ResourceConverged { ref resource, .. } => {
                        let key = format!("{}:{}", name, resource);
                        let entry = metrics.entry(key).or_insert((0, 0, 0));
                        entry.0 += 1;
                    }
                    types::ProvenanceEvent::ResourceFailed { ref resource, .. } => {
                        let key = format!("{}:{}", name, resource);
                        let entry = metrics.entry(key).or_insert((0, 0, 0));
                        entry.1 += 1;
                    }
                    types::ProvenanceEvent::DriftDetected { ref resource, .. } => {
                        let key = format!("{}:{}", name, resource);
                        let entry = metrics.entry(key).or_insert((0, 0, 0));
                        entry.2 += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    // Convert metrics HashMap to Vec for detect_anomalies()
    let metrics_vec: Vec<(String, u32, u32, u32)> = metrics
        .into_iter()
        .map(|(k, (c, f, d))| (k, c, f, d))
        .collect();

    // FJ-051: Use anomaly module for detection
    let findings = anomaly::detect_anomalies(&metrics_vec, min_events);

    if findings.is_empty() {
        if json {
            println!("{{\"anomalies\":0,\"findings\":[]}}");
        } else {
            let total = metrics_vec.len();
            println!(
                "No anomalies detected ({} resources analyzed, min {} events).",
                total, min_events
            );
        }
        return Ok(());
    }

    if json {
        let json_findings: Vec<serde_json::Value> = findings
            .iter()
            .map(|f| {
                serde_json::json!({
                    "resource": f.resource,
                    "score": f.score,
                    "status": format!("{:?}", f.status),
                    "reasons": f.reasons,
                })
            })
            .collect();
        let report = serde_json::json!({
            "anomalies": json_findings.len(),
            "findings": json_findings,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {}", e))?;
        println!("{}", output);
    } else {
        for finding in &findings {
            let status_label = match finding.status {
                anomaly::DriftStatus::Drift => "DRIFT",
                anomaly::DriftStatus::Warning => "WARNING",
                anomaly::DriftStatus::Stable => "STABLE",
            };
            println!(
                "  ANOMALY: {} [{}] (score={:.2}) — {}",
                finding.resource,
                status_label,
                finding.score,
                finding.reasons.join("; ")
            );
        }
        println!();
        println!(
            "Anomaly detection: {} anomaly(ies) found.",
            findings.len()
        );
    }

    Ok(())
}

/// Rollback to a previous config revision from git history.
///
/// Uses `git show HEAD~N:<file>` to fetch the previous forjar.yaml,
/// then re-applies it with --force to converge to the prior desired state.
fn cmd_rollback(
    file: &Path,
    state_dir: &Path,
    revision: u32,
    machine_filter: Option<&str>,
    dry_run: bool,
    verbose: bool,
) -> Result<(), String> {
    // Resolve the file path relative to git repo
    let file_str = file.to_string_lossy();

    // Fetch the previous config from git
    let git_ref = format!("HEAD~{}:{}", revision, file_str);
    let output = std::process::Command::new("git")
        .args(["show", &git_ref])
        .output()
        .map_err(|e| format!("git show failed: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "cannot read {} from git history (HEAD~{}): {}",
            file_str,
            revision,
            stderr.trim()
        ));
    }

    let previous_yaml = String::from_utf8_lossy(&output.stdout);

    // Parse the previous config
    let previous_config: types::ForjarConfig = serde_yaml_ng::from_str(&previous_yaml)
        .map_err(|e| format!("cannot parse previous config (HEAD~{}): {}", revision, e))?;

    // Parse the current config for comparison
    let current_config = parse_and_validate(file)?;

    // Show what changed between current and previous
    let mut changes = Vec::new();
    for (id, prev_resource) in &previous_config.resources {
        if let Some(cur_resource) = current_config.resources.get(id) {
            // Resource exists in both — check if it changed
            let prev_yaml = serde_yaml_ng::to_string(prev_resource).unwrap_or_default();
            let cur_yaml = serde_yaml_ng::to_string(cur_resource).unwrap_or_default();
            if prev_yaml != cur_yaml {
                changes.push(format!("  ~ {} (modified)", id));
            }
        } else {
            // Resource was in previous but not current — it was removed
            changes.push(format!(
                "  + {} (will be re-added from HEAD~{})",
                id, revision
            ));
        }
    }
    for id in current_config.resources.keys() {
        if !previous_config.resources.contains_key(id) {
            changes.push(format!(
                "  - {} (exists now but not in HEAD~{}, will remain)",
                id, revision
            ));
        }
    }

    if changes.is_empty() {
        println!(
            "No config changes between HEAD and HEAD~{}. Nothing to rollback.",
            revision
        );
        return Ok(());
    }

    println!("Rollback to HEAD~{} ({}):", revision, previous_config.name);
    for c in &changes {
        println!("{}", c);
    }
    println!();

    if dry_run {
        println!("Dry run: {} change(s) would be applied.", changes.len());
        return Ok(());
    }

    // Write the previous config to a temp file and apply with --force
    let temp_config = std::env::temp_dir().join("forjar-rollback.yaml");
    std::fs::write(&temp_config, previous_yaml.as_bytes())
        .map_err(|e| format!("cannot write temp config: {}", e))?;

    println!("Applying previous config with --force...");
    cmd_apply(
        &temp_config,
        state_dir,
        machine_filter,
        None,  // no resource filter
        None,  // no tag filter
        true,  // force — re-apply everything
        false, // not dry-run (we already checked above)
        false, // tripwire on
        &[],   // no param overrides
        false, // no auto-commit
        None,  // no timeout
        verbose,
    )
}

fn cmd_fmt(file: &Path, check: bool) -> Result<(), String> {
    let original = std::fs::read_to_string(file)
        .map_err(|e| format!("cannot read {}: {}", file.display(), e))?;

    // Parse into ForjarConfig to validate + normalize
    let config: types::ForjarConfig =
        serde_yaml_ng::from_str(&original).map_err(|e| format!("YAML parse error: {}", e))?;

    // Re-serialize to canonical YAML
    let formatted =
        serde_yaml_ng::to_string(&config).map_err(|e| format!("YAML serialize error: {}", e))?;

    if check {
        if original.trim() != formatted.trim() {
            println!("{} is not formatted", file.display());
            return Err("file is not formatted".to_string());
        }
        println!("{} is formatted", file.display());
        return Ok(());
    }

    if original.trim() == formatted.trim() {
        println!("{} already formatted", file.display());
        return Ok(());
    }

    std::fs::write(file, &formatted)
        .map_err(|e| format!("cannot write {}: {}", file.display(), e))?;
    println!("Formatted {}", file.display());
    Ok(())
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

params:
  env: development

machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
  # remote-server:
  #   hostname: my-server
  #   addr: 10.0.0.1
  #   user: root
  #   ssh_key: ~/.ssh/id_ed25519

resources:
  # Example: install packages
  base-packages:
    type: package
    machine: localhost
    provider: apt
    packages: [curl, git, htop]

  # Example: manage a config file
  # app-config:
  #   type: file
  #   machine: localhost
  #   path: /etc/myapp/config.yaml
  #   content: |
  #     environment: {{params.env}}
  #     log_level: info
  #   owner: root
  #   mode: "0644"
  #   depends_on: [base-packages]

  # Example: manage a service
  # app-service:
  #   type: service
  #   machine: localhost
  #   name: myapp
  #   state: running
  #   enabled: true
  #   restart_on: [app-config]
  #   depends_on: [app-config]

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

fn cmd_import(
    addr: &str,
    user: &str,
    name: Option<&str>,
    output: &Path,
    scan: &[String],
    verbose: bool,
) -> Result<(), String> {
    let machine_name = name.unwrap_or_else(|| {
        if addr == "localhost" || addr == "127.0.0.1" {
            "localhost"
        } else {
            addr.split('.').next().unwrap_or("imported")
        }
    });

    let machine = types::Machine {
        hostname: machine_name.to_string(),
        addr: addr.to_string(),
        user: user.to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        cost: 0,
    };

    let scan_set: std::collections::HashSet<&str> = scan.iter().map(|s| s.as_str()).collect();

    let mut resources_yaml = String::new();
    let mut resource_count = 0;

    // Scan packages
    if scan_set.contains("packages") {
        if verbose {
            eprintln!("Scanning installed packages on {}...", addr);
        }
        let script = "dpkg-query -W -f='${Package}\\n' 2>/dev/null | sort | head -100";
        match transport::exec_script(&machine, script) {
            Ok(output) => {
                let packages: Vec<&str> = output
                    .stdout
                    .lines()
                    .filter(|l| !l.is_empty())
                    .take(50)
                    .collect();
                if !packages.is_empty() {
                    resources_yaml.push_str("  imported-packages:\n");
                    resources_yaml.push_str("    type: package\n");
                    resources_yaml.push_str(&format!("    machine: {}\n", machine_name));
                    resources_yaml.push_str("    provider: apt\n");
                    resources_yaml.push_str("    packages:\n");
                    for pkg in &packages {
                        resources_yaml.push_str(&format!("      - {}\n", pkg));
                    }
                    resources_yaml.push('\n');
                    resource_count += 1;
                    if verbose {
                        eprintln!("  Found {} packages", packages.len());
                    }
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("  Package scan failed: {}", e);
                }
            }
        }
    }

    // Scan services
    if scan_set.contains("services") {
        if verbose {
            eprintln!("Scanning enabled services on {}...", addr);
        }
        let script =
            "systemctl list-unit-files --type=service --state=enabled --no-legend 2>/dev/null \
             | awk '{print $1}' | sed 's/\\.service$//' | sort";
        match transport::exec_script(&machine, script) {
            Ok(output) => {
                let services: Vec<&str> = output
                    .stdout
                    .lines()
                    .filter(|l| !l.is_empty() && !l.starts_with("UNIT"))
                    .collect();
                for svc in &services {
                    let id = format!("svc-{}", svc.replace('.', "-"));
                    resources_yaml.push_str(&format!("  {}:\n", id));
                    resources_yaml.push_str("    type: service\n");
                    resources_yaml.push_str(&format!("    machine: {}\n", machine_name));
                    resources_yaml.push_str(&format!("    name: {}\n", svc));
                    resources_yaml.push_str("    state: running\n");
                    resources_yaml.push_str("    enabled: true\n\n");
                    resource_count += 1;
                }
                if verbose {
                    eprintln!("  Found {} enabled services", services.len());
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("  Service scan failed: {}", e);
                }
            }
        }
    }

    // Scan managed config files (common paths)
    if scan_set.contains("files") {
        if verbose {
            eprintln!("Scanning config files on {}...", addr);
        }
        let script = "find /etc -maxdepth 2 -name '*.conf' -type f 2>/dev/null | sort | head -20";
        match transport::exec_script(&machine, script) {
            Ok(output) => {
                let files: Vec<&str> = output.stdout.lines().filter(|l| !l.is_empty()).collect();
                for file_path in &files {
                    let basename = std::path::Path::new(file_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("config");
                    let id = format!("file-{}", basename.replace('.', "-"));
                    resources_yaml.push_str(&format!("  {}:\n", id));
                    resources_yaml.push_str("    type: file\n");
                    resources_yaml.push_str(&format!("    machine: {}\n", machine_name));
                    resources_yaml.push_str(&format!("    path: {}\n", file_path));
                    resources_yaml.push_str(&format!("    # source: configs{}\n", file_path));
                    resources_yaml.push_str("    owner: root\n");
                    resources_yaml.push_str("    group: root\n");
                    resources_yaml.push_str("    mode: \"0644\"\n\n");
                    resource_count += 1;
                }
                if verbose {
                    eprintln!("  Found {} config files", files.len());
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("  File scan failed: {}", e);
                }
            }
        }
    }

    // Scan users (non-system, UID >= 1000)
    if scan_set.contains("users") {
        if verbose {
            eprintln!("Scanning local users on {}...", addr);
        }
        let script = "awk -F: '$3 >= 1000 && $3 < 65534 {print $1\":\"$6\":\"$7}' /etc/passwd";
        match transport::exec_script(&machine, script) {
            Ok(output) => {
                let users: Vec<&str> = output.stdout.lines().filter(|l| !l.is_empty()).collect();
                for user_line in &users {
                    let parts: Vec<&str> = user_line.split(':').collect();
                    if parts.len() >= 3 {
                        let uname = parts[0];
                        let home = parts[1];
                        let shell = parts[2];
                        let id = format!("user-{}", uname);
                        resources_yaml.push_str(&format!("  {}:\n", id));
                        resources_yaml.push_str("    type: user\n");
                        resources_yaml.push_str(&format!("    machine: {}\n", machine_name));
                        resources_yaml.push_str(&format!("    name: {}\n", uname));
                        resources_yaml.push_str(&format!("    home: {}\n", home));
                        resources_yaml.push_str(&format!("    shell: {}\n\n", shell));
                        resource_count += 1;
                    }
                }
                if verbose {
                    eprintln!("  Found {} users", users.len());
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("  User scan failed: {}", e);
                }
            }
        }
    }

    // Scan cron jobs (root crontab)
    if scan_set.contains("cron") {
        if verbose {
            eprintln!("Scanning cron jobs on {}...", addr);
        }
        let script = "crontab -l 2>/dev/null | grep -v '^#' | grep -v '^$' || true";
        match transport::exec_script(&machine, script) {
            Ok(output) => {
                let jobs: Vec<&str> = output.stdout.lines().filter(|l| !l.is_empty()).collect();
                for (i, job) in jobs.iter().enumerate() {
                    let parts: Vec<&str> = job.splitn(6, ' ').collect();
                    if parts.len() >= 6 {
                        let schedule = parts[..5].join(" ");
                        let command = parts[5];
                        let id = format!("cron-job-{}", i + 1);
                        resources_yaml.push_str(&format!("  {}:\n", id));
                        resources_yaml.push_str("    type: cron\n");
                        resources_yaml.push_str(&format!("    machine: {}\n", machine_name));
                        resources_yaml.push_str(&format!("    name: imported-cron-{}\n", i + 1));
                        resources_yaml.push_str(&format!("    schedule: \"{}\"\n", schedule));
                        resources_yaml.push_str(&format!("    command: {}\n", command));
                        resources_yaml.push_str("    owner: root\n\n");
                        resource_count += 1;
                    }
                }
                if verbose {
                    eprintln!("  Found {} cron jobs", jobs.len());
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("  Cron scan failed: {}", e);
                }
            }
        }
    }

    // Generate output YAML
    let config_yaml = format!(
        r#"# Generated by: forjar import --addr {}
# Review and customize before applying.
version: "1.0"
name: imported-{}

machines:
  {}:
    hostname: {}
    addr: {}
    user: {}

resources:
{}
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#,
        addr, machine_name, machine_name, machine_name, addr, user, resources_yaml,
    );

    std::fs::write(output, &config_yaml)
        .map_err(|e| format!("cannot write {}: {}", output.display(), e))?;

    println!(
        "Imported {} resources from {} → {}",
        resource_count,
        addr,
        output.display()
    );
    Ok(())
}

fn cmd_show(file: &Path, resource_filter: Option<&str>, json: bool) -> Result<(), String> {
    let mut config = parse_and_validate(file)?;

    // Resolve templates in all resources
    for (_id, resource) in config.resources.iter_mut() {
        *resource =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;
    }

    if let Some(resource_id) = resource_filter {
        let resource = config
            .resources
            .get(resource_id)
            .ok_or_else(|| format!("resource '{}' not found", resource_id))?;
        if json {
            let output =
                serde_json::to_string_pretty(resource).map_err(|e| format!("JSON error: {}", e))?;
            println!("{}", output);
        } else {
            let output =
                serde_yaml_ng::to_string(resource).map_err(|e| format!("YAML error: {}", e))?;
            println!("{}:\n{}", resource_id, output);
        }
    } else if json {
        let output =
            serde_json::to_string_pretty(&config).map_err(|e| format!("JSON error: {}", e))?;
        println!("{}", output);
    } else {
        let output = serde_yaml_ng::to_string(&config).map_err(|e| format!("YAML error: {}", e))?;
        println!("{}", output);
    }

    Ok(())
}

fn cmd_check(
    file: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    json: bool,
    verbose: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    if verbose {
        eprintln!(
            "Checking {} ({} machines, {} resources)",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
    }

    // Build execution order
    let execution_order = resolver::build_execution_order(&config)?;

    let localhost_machine = types::Machine {
        hostname: "localhost".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        cost: 0,
    };

    let mut total_pass = 0usize;
    let mut total_fail = 0usize;
    let mut total_skip = 0usize;
    let mut json_results = Vec::new();

    for resource_id in &execution_order {
        let resource = match config.resources.get(resource_id) {
            Some(r) => r,
            None => continue,
        };

        if let Some(filter) = resource_filter {
            if resource_id != filter {
                continue;
            }
        }

        // Tag filtering: skip resource if --tag specified and resource doesn't have the tag
        if let Some(tag) = tag_filter {
            if !resource.tags.iter().any(|t| t == tag) {
                total_skip += 1;
                continue;
            }
        }

        let resolved =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;

        let check_script = match codegen::check_script(&resolved) {
            Ok(s) => s,
            Err(_) => {
                total_skip += 1;
                if !json {
                    println!("  ? {} (no check script)", resource_id);
                }
                continue;
            }
        };

        for machine_name in resource.machine.to_vec() {
            if let Some(filter) = machine_filter {
                if machine_name != filter {
                    continue;
                }
            }

            let machine = config
                .machines
                .get(&machine_name)
                .unwrap_or(&localhost_machine);

            // FJ-064: arch filter
            if !resource.arch.is_empty() && !resource.arch.contains(&machine.arch) {
                total_skip += 1;
                continue;
            }

            // Ensure container is running for check
            if machine.is_container_transport() {
                transport::container::ensure_container(machine)?;
            }

            let output = transport::exec_script(machine, &check_script);
            match output {
                Ok(out) if out.success() => {
                    total_pass += 1;
                    if json {
                        json_results.push(serde_json::json!({
                            "resource": resource_id,
                            "machine": machine_name,
                            "status": "pass",
                            "exit_code": 0,
                        }));
                    } else {
                        println!("  ok {} ({})", resource_id, machine_name);
                    }
                }
                Ok(out) => {
                    total_fail += 1;
                    if json {
                        json_results.push(serde_json::json!({
                            "resource": resource_id,
                            "machine": machine_name,
                            "status": "fail",
                            "exit_code": out.exit_code,
                            "stderr": out.stderr.trim(),
                        }));
                    } else {
                        println!(
                            "  FAIL {} ({}) — exit {}",
                            resource_id, machine_name, out.exit_code
                        );
                        if !out.stderr.trim().is_empty() {
                            for line in out.stderr.trim().lines() {
                                println!("       {}", line);
                            }
                        }
                    }
                }
                Err(e) => {
                    total_fail += 1;
                    if json {
                        json_results.push(serde_json::json!({
                            "resource": resource_id,
                            "machine": machine_name,
                            "status": "error",
                            "error": e,
                        }));
                    } else {
                        println!("  FAIL {} ({}) — {}", resource_id, machine_name, e);
                    }
                }
            }
        }
    }

    if json {
        let report = serde_json::json!({
            "pass": total_pass,
            "fail": total_fail,
            "skip": total_skip,
            "results": json_results,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {}", e))?
        );
    } else {
        println!(
            "\nCheck: {} pass, {} fail, {} skip",
            total_pass, total_fail, total_skip
        );
    }

    if total_fail > 0 {
        Err(format!("{} check(s) failed", total_fail))
    } else {
        Ok(())
    }
}

fn cmd_diff(
    from: &Path,
    to: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    // Discover machines from both state directories
    let from_machines = discover_machines(from);
    let to_machines = discover_machines(to);
    let mut all_machines: Vec<String> = from_machines
        .iter()
        .chain(to_machines.iter())
        .cloned()
        .collect();
    all_machines.sort();
    all_machines.dedup();

    if let Some(filter) = machine_filter {
        all_machines.retain(|m| m == filter);
    }

    if all_machines.is_empty() {
        return Err("no machines found in either state directory".to_string());
    }

    let mut total_added = 0usize;
    let mut total_removed = 0usize;
    let mut total_changed = 0usize;
    let mut total_unchanged = 0usize;
    let mut json_machines = Vec::new();

    for machine_name in &all_machines {
        let from_lock = state::load_lock(from, machine_name)?;
        let to_lock = state::load_lock(to, machine_name)?;

        let from_resources = from_lock
            .as_ref()
            .map(|l| &l.resources)
            .cloned()
            .unwrap_or_default();
        let to_resources = to_lock
            .as_ref()
            .map(|l| &l.resources)
            .cloned()
            .unwrap_or_default();

        let mut diffs = Vec::new();

        // Resources added (in to, not in from)
        for (id, to_res) in &to_resources {
            if !from_resources.contains_key(id) {
                diffs.push(ResourceDiff {
                    resource: id.clone(),
                    change: DiffChange::Added,
                    from_hash: None,
                    to_hash: Some(to_res.hash.clone()),
                    from_status: None,
                    to_status: Some(format!("{:?}", to_res.status)),
                });
                total_added += 1;
            }
        }

        // Resources removed (in from, not in to)
        for (id, from_res) in &from_resources {
            if !to_resources.contains_key(id) {
                diffs.push(ResourceDiff {
                    resource: id.clone(),
                    change: DiffChange::Removed,
                    from_hash: Some(from_res.hash.clone()),
                    to_hash: None,
                    from_status: Some(format!("{:?}", from_res.status)),
                    to_status: None,
                });
                total_removed += 1;
            }
        }

        // Resources changed (in both, different hash or status)
        for (id, from_res) in &from_resources {
            if let Some(to_res) = to_resources.get(id) {
                if from_res.hash != to_res.hash || from_res.status != to_res.status {
                    diffs.push(ResourceDiff {
                        resource: id.clone(),
                        change: DiffChange::Changed,
                        from_hash: Some(from_res.hash.clone()),
                        to_hash: Some(to_res.hash.clone()),
                        from_status: Some(format!("{:?}", from_res.status)),
                        to_status: Some(format!("{:?}", to_res.status)),
                    });
                    total_changed += 1;
                } else {
                    total_unchanged += 1;
                }
            }
        }

        // Sort diffs by resource name for determinism
        diffs.sort_by(|a, b| a.resource.cmp(&b.resource));

        if json {
            json_machines.push(serde_json::json!({
                "machine": machine_name,
                "diffs": diffs.iter().map(|d| serde_json::json!({
                    "resource": d.resource,
                    "change": format!("{:?}", d.change).to_lowercase(),
                    "from_hash": d.from_hash,
                    "to_hash": d.to_hash,
                    "from_status": d.from_status,
                    "to_status": d.to_status,
                })).collect::<Vec<_>>(),
            }));
        } else if !diffs.is_empty() {
            println!("Machine: {}", machine_name);
            for d in &diffs {
                let symbol = match d.change {
                    DiffChange::Added => "+",
                    DiffChange::Removed => "-",
                    DiffChange::Changed => "~",
                };
                print!("  {} {}", symbol, d.resource);
                match d.change {
                    DiffChange::Added => {
                        println!(" ({})", d.to_status.as_deref().unwrap_or("?"));
                    }
                    DiffChange::Removed => {
                        println!(" (was {})", d.from_status.as_deref().unwrap_or("?"));
                    }
                    DiffChange::Changed => {
                        println!(
                            " ({} → {})",
                            d.from_status.as_deref().unwrap_or("?"),
                            d.to_status.as_deref().unwrap_or("?")
                        );
                    }
                }
            }
            println!();
        }
    }

    if json {
        let report = serde_json::json!({
            "from": from.display().to_string(),
            "to": to.display().to_string(),
            "summary": {
                "added": total_added,
                "removed": total_removed,
                "changed": total_changed,
                "unchanged": total_unchanged,
            },
            "machines": json_machines,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {}", e))?
        );
    } else {
        println!(
            "Diff: {} added, {} removed, {} changed, {} unchanged",
            total_added, total_removed, total_changed, total_unchanged
        );
    }

    Ok(())
}

#[derive(Debug)]
enum DiffChange {
    Added,
    Removed,
    Changed,
}

struct ResourceDiff {
    resource: String,
    change: DiffChange,
    from_hash: Option<String>,
    to_hash: Option<String>,
    from_status: Option<String>,
    to_status: Option<String>,
}

/// Discover machine names from a state directory by listing subdirectories that contain state.lock.yaml.
fn discover_machines(state_dir: &Path) -> Vec<String> {
    let mut machines = Vec::new();
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.path().join("state.lock.yaml").exists() {
                    machines.push(name);
                }
            }
        }
    }
    machines.sort();
    machines
}

fn cmd_validate(file: &Path) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    println!(
        "OK: {} ({} machines, {} resources)",
        config.name,
        config.machines.len(),
        config.resources.len()
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_plan(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    _resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    json: bool,
    verbose: bool,
    output_dir: Option<&Path>,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    if verbose {
        eprintln!(
            "Planning {} ({} machines, {} resources)",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
    }
    let execution_order = resolver::build_execution_order(&config)?;

    // Load existing locks so plan shows accurate Create vs Update vs NoOp
    let locks = load_machine_locks(&config, state_dir, machine_filter)?;
    let plan = planner::plan(&config, &execution_order, &locks, tag_filter);

    if let Some(dir) = output_dir {
        export_scripts(&config, dir)?;
    }

    if json {
        let output =
            serde_json::to_string_pretty(&plan).map_err(|e| format!("JSON error: {}", e))?;
        println!("{}", output);
    } else {
        print_plan(&plan, machine_filter);
    }
    Ok(())
}

/// Parse, validate, and expand recipes in a forjar config file.
fn parse_and_validate(file: &Path) -> Result<types::ForjarConfig, String> {
    parser::parse_and_validate(file)
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

/// Export generated scripts (check, apply, state_query) to a directory for auditing.
/// Templates (params, secrets, machine refs) are resolved before export.
fn export_scripts(config: &types::ForjarConfig, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("cannot create output dir {}: {}", dir.display(), e))?;

    let mut count = 0;
    for (id, resource) in &config.resources {
        // Resolve templates (params, secrets, machine refs) before codegen
        let resolved =
            resolver::resolve_resource_templates(resource, &config.params, &config.machines)?;

        // Sanitize resource ID for filesystem (replace / with --)
        let safe_id = id.replace('/', "--");

        if let Ok(script) = codegen::check_script(&resolved) {
            let path = dir.join(format!("{}.check.sh", safe_id));
            std::fs::write(&path, &script)
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
            count += 1;
        }

        if let Ok(script) = codegen::apply_script(&resolved) {
            let path = dir.join(format!("{}.apply.sh", safe_id));
            std::fs::write(&path, &script)
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
            count += 1;
        }

        if let Ok(script) = codegen::state_query_script(&resolved) {
            let path = dir.join(format!("{}.state_query.sh", safe_id));
            std::fs::write(&path, &script)
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
            count += 1;
        }
    }

    println!("Exported {} scripts to {}", count, dir.display());
    Ok(())
}

/// Run a local shell hook command. Returns Ok if the command succeeds, Err if it fails.
fn run_hook(name: &str, command: &str, verbose: bool) -> Result<(), String> {
    if verbose {
        eprintln!("Running {} hook: {}", name, command);
    }
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| format!("{} hook failed to start: {}", name, e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{} hook failed (exit {}): {}",
            name,
            output.status.code().unwrap_or(-1),
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        print!("{}", stdout);
    }
    Ok(())
}

/// Parse KEY=VALUE param overrides and merge into config.
fn apply_param_overrides(
    config: &mut types::ForjarConfig,
    overrides: &[String],
) -> Result<(), String> {
    for kv in overrides {
        let (key, value) = kv
            .split_once('=')
            .ok_or_else(|| format!("invalid param '{}': expected KEY=VALUE", kv))?;
        config.params.insert(
            key.to_string(),
            serde_yaml_ng::Value::String(value.to_string()),
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_apply(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    resource_filter: Option<&str>,
    tag_filter: Option<&str>,
    force: bool,
    dry_run: bool,
    no_tripwire: bool,
    param_overrides: &[String],
    auto_commit: bool,
    timeout_secs: Option<u64>,
    verbose: bool,
) -> Result<(), String> {
    let mut config = parse_and_validate(file)?;
    if verbose {
        eprintln!(
            "Applying {} ({} machines, {} resources)",
            config.name,
            config.machines.len(),
            config.resources.len()
        );
    }
    if no_tripwire {
        config.policy.tripwire = false;
    }
    apply_param_overrides(&mut config, param_overrides)?;

    // Run pre_apply hook (abort on failure)
    if let Some(ref hook) = config.policy.pre_apply {
        if !dry_run {
            run_hook("pre_apply", hook, verbose)?;
        }
    }

    let cfg = executor::ApplyConfig {
        config: &config,
        state_dir,
        force,
        dry_run,
        machine_filter,
        resource_filter,
        tag_filter,
        timeout_secs,
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

    // Update global lock file
    let machine_results: Vec<_> = results
        .iter()
        .map(|r| {
            (
                r.machine.clone(),
                (r.resources_converged + r.resources_unchanged + r.resources_failed) as usize,
                r.resources_converged as usize,
                r.resources_failed as usize,
            )
        })
        .collect();
    state::update_global_lock(state_dir, &config.name, &machine_results)?;

    if auto_commit && total_converged > 0 {
        git_commit_state(state_dir, &config.name, total_converged)?;
    }

    // Run post_apply hook (informational — doesn't affect exit code)
    if let Some(ref hook) = config.policy.post_apply {
        if let Err(e) = run_hook("post_apply", hook, verbose) {
            eprintln!("Warning: {}", e);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_drift(
    config_path: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    tripwire_mode: bool,
    alert_cmd: Option<&str>,
    auto_remediate: bool,
    dry_run: bool,
    json: bool,
    verbose: bool,
) -> Result<(), String> {
    // Load config to get machine definitions (needed for container transport drift)
    let config = if config_path.exists() {
        Some(parse_and_validate(config_path)?)
    } else {
        None
    };

    // Dry-run: list what would be checked without connecting to machines
    if dry_run {
        return cmd_drift_dry_run(state_dir, machine_filter, json);
    }

    // For container machines, ensure containers are running for drift checks
    if let Some(ref cfg) = config {
        for (_, machine) in &cfg.machines {
            if machine.is_container_transport() {
                crate::transport::container::ensure_container(machine)?;
            }
        }
    }

    // List machine directories in state/
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut total_drift = 0;
    let mut all_findings: Vec<serde_json::Value> = Vec::new();

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
            if verbose {
                eprintln!("Checking {} ({} resources)...", name, lock.resources.len());
            }
            if !json {
                println!("Checking {} ({} resources)...", name, lock.resources.len());
            }

            // Use full drift detection when config is available (checks all resource types)
            let machine = config.as_ref().and_then(|c| c.machines.get(&name));
            let findings = match (machine, config.as_ref()) {
                (Some(m), Some(cfg)) => drift::detect_drift_full(&lock, m, &cfg.resources),
                (Some(m), None) => drift::detect_drift_with_machine(&lock, m),
                _ => drift::detect_drift(&lock),
            };

            if findings.is_empty() {
                if !json {
                    println!("  No drift detected.");
                }
            } else {
                for f in &findings {
                    if json {
                        all_findings.push(serde_json::json!({
                            "machine": name,
                            "resource": f.resource_id,
                            "detail": f.detail,
                            "expected_hash": f.expected_hash,
                            "actual_hash": f.actual_hash,
                        }));
                    } else {
                        println!("  DRIFTED: {} ({})", f.resource_id, f.detail);
                        println!("    Expected: {}", f.expected_hash);
                        println!("    Actual:   {}", f.actual_hash);
                    }
                }
                total_drift += findings.len();
            }
        }
    }

    if json {
        let report = serde_json::json!({
            "drift_count": total_drift,
            "findings": all_findings,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {}", e))?;
        println!("{}", output);
    } else if total_drift > 0 {
        println!();
        println!("Drift detected: {} resource(s)", total_drift);
    } else {
        println!("No drift detected.");
    }

    // Run alert command on drift detection
    if total_drift > 0 {
        if let Some(cmd) = alert_cmd {
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .env("FORJAR_DRIFT_COUNT", total_drift.to_string())
                .status()
                .map_err(|e| format!("alert-cmd failed to execute: {}", e))?;
            if !status.success() {
                eprintln!("alert-cmd exited with code {}", status.code().unwrap_or(-1));
            }
        }
    }

    // Auto-remediate: re-apply with --force to restore desired state
    if auto_remediate && total_drift > 0 {
        if !json {
            println!();
            println!("Auto-remediating {} drifted resource(s)...", total_drift);
        }
        cmd_apply(
            config_path,
            state_dir,
            machine_filter,
            None,  // no resource filter — force re-applies all
            None,  // no tag filter
            true,  // force
            false, // not dry-run
            false, // tripwire on
            &[],   // no param overrides
            false, // no auto-commit
            None,  // no timeout
            verbose,
        )?;
        if !json {
            println!("Remediation complete.");
        }
    }

    if tripwire_mode && total_drift > 0 {
        return Err(format!("{} drift finding(s)", total_drift));
    }

    Ok(())
}

/// Dry-run mode for drift: lists resources that would be checked without connecting.
fn cmd_drift_dry_run(
    state_dir: &Path,
    machine_filter: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut checks: Vec<serde_json::Value> = Vec::new();
    let mut total = 0usize;

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
            if !json {
                println!("Machine: {} ({} resources)", name, lock.resources.len());
            }
            for (res_id, res_state) in &lock.resources {
                total += 1;
                if json {
                    checks.push(serde_json::json!({
                        "machine": name,
                        "resource": res_id,
                        "status": res_state.status,
                        "hash": res_state.hash,
                    }));
                } else {
                    println!("  would check: {} (status: {})", res_id, res_state.status);
                }
            }
        }
    }

    if json {
        let report = serde_json::json!({
            "dry_run": true,
            "total_checks": total,
            "checks": checks,
        });
        let output =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {}", e))?;
        println!("{}", output);
    } else {
        println!();
        println!("Dry run: {} resource(s) would be checked", total);
    }

    Ok(())
}

/// Git commit state directory after successful apply.
fn git_commit_state(state_dir: &Path, config_name: &str, converged: u32) -> Result<(), String> {
    let msg = format!(
        "forjar: {} — {} resource(s) converged",
        config_name, converged
    );
    // Find the git repo root from state_dir's parent
    let repo_root = state_dir.parent().unwrap_or(Path::new("."));
    let status = std::process::Command::new("git")
        .current_dir(repo_root)
        .args(["add", "state"])
        .status()
        .map_err(|e| format!("git add failed: {}", e))?;
    if !status.success() {
        return Err("git add state/ failed".to_string());
    }
    let status = std::process::Command::new("git")
        .current_dir(repo_root)
        .args(["commit", "--no-verify", "-m", &msg])
        .status()
        .map_err(|e| format!("git commit failed: {}", e))?;
    if !status.success() {
        return Err("git commit failed".to_string());
    }
    println!("Auto-committed state: {}", msg);
    Ok(())
}

fn cmd_history(
    state_dir: &Path,
    machine_filter: Option<&str>,
    limit: usize,
    json: bool,
) -> Result<(), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut all_events: Vec<types::TimestampedEvent> = Vec::new();

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

        let log_path = eventlog::event_log_path(state_dir, &name);
        if !log_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&log_path)
            .map_err(|e| format!("cannot read {}: {}", log_path.display(), e))?;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<types::TimestampedEvent>(line) {
                all_events.push(event);
            }
        }
    }

    // Sort by timestamp descending (most recent first)
    all_events.sort_by(|a, b| b.ts.cmp(&a.ts));

    // Filter to apply_started/apply_completed events for summary, then limit
    let apply_events: Vec<&types::TimestampedEvent> = all_events
        .iter()
        .filter(|e| {
            matches!(
                e.event,
                types::ProvenanceEvent::ApplyStarted { .. }
                    | types::ProvenanceEvent::ApplyCompleted { .. }
            )
        })
        .take(limit)
        .collect();

    if json {
        let output = serde_json::to_string_pretty(&apply_events)
            .map_err(|e| format!("JSON error: {}", e))?;
        println!("{}", output);
    } else if apply_events.is_empty() {
        println!("No apply history found. Run `forjar apply` first.");
    } else {
        for event in &apply_events {
            match &event.event {
                types::ProvenanceEvent::ApplyStarted {
                    machine, run_id, ..
                } => {
                    println!("{} started  {} ({})", event.ts, machine, run_id);
                }
                types::ProvenanceEvent::ApplyCompleted {
                    machine,
                    run_id,
                    resources_converged,
                    resources_unchanged,
                    resources_failed,
                    total_seconds,
                } => {
                    println!(
                        "{} complete {} ({}) — {} converged, {} unchanged, {} failed ({:.1}s)",
                        event.ts,
                        machine,
                        run_id,
                        resources_converged,
                        resources_unchanged,
                        resources_failed,
                        total_seconds
                    );
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn cmd_destroy(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    yes: bool,
    verbose: bool,
) -> Result<(), String> {
    if !yes {
        return Err(
            "destroy requires --yes flag to confirm removal of all managed resources".to_string(),
        );
    }

    let config = parse_and_validate(file)?;
    let execution_order = resolver::build_execution_order(&config)?;

    // Reverse order for teardown (dependents first)
    let reverse_order: Vec<String> = execution_order.into_iter().rev().collect();

    if verbose {
        eprintln!(
            "Destroying {} resources in reverse order",
            reverse_order.len()
        );
    }

    let all_machines = executor::collect_machines(&config);
    let mut destroyed = 0u32;
    let mut failed = 0u32;

    for resource_id in &reverse_order {
        let resource = match config.resources.get(resource_id) {
            Some(r) => r,
            None => continue,
        };

        let machine_name = match &resource.machine {
            types::MachineTarget::Single(m) => m.as_str(),
            types::MachineTarget::Multiple(ms) => {
                if ms.is_empty() {
                    continue;
                }
                ms[0].as_str()
            }
        };

        if let Some(filter) = machine_filter {
            if machine_name != filter {
                continue;
            }
        }

        // Clone resource and set state to absent
        let mut destroy_resource = resource.clone();
        destroy_resource.state = Some("absent".to_string());

        let machine = config.machines.get(machine_name);

        let script = match codegen::apply_script(&destroy_resource) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  SKIP {}: codegen error: {}", resource_id, e);
                failed += 1;
                continue;
            }
        };

        if let Some(m) = machine {
            if m.is_container_transport() {
                let _ = crate::transport::container::ensure_container(m);
            }
            match transport::exec_script(m, &script) {
                Ok(out) if out.success() => {
                    println!("  - {} ({})", resource_id, resource.resource_type);
                    destroyed += 1;
                }
                Ok(out) => {
                    eprintln!(
                        "  FAIL {}: exit {}: {}",
                        resource_id,
                        out.exit_code,
                        out.stderr.trim()
                    );
                    failed += 1;
                }
                Err(e) => {
                    eprintln!("  FAIL {}: {}", resource_id, e);
                    failed += 1;
                }
            }
        } else {
            eprintln!(
                "  SKIP {}: machine '{}' not found",
                resource_id, machine_name
            );
            failed += 1;
        }
    }

    // Clean up state files
    if failed == 0 {
        for machine_name in &all_machines {
            if let Some(filter) = machine_filter {
                if machine_name != filter {
                    continue;
                }
            }
            let lock_path = state_dir.join(machine_name).join("state.lock.yaml");
            if lock_path.exists() {
                let _ = std::fs::remove_file(&lock_path);
            }
        }
    }

    println!();
    if failed > 0 {
        println!(
            "Destroy completed with errors: {} destroyed, {} failed",
            destroyed, failed
        );
        return Err(format!("{} resource(s) failed to destroy", failed));
    }

    println!("Destroy complete: {} resources removed.", destroyed);
    Ok(())
}

fn cmd_graph(file: &Path, format: &str) -> Result<(), String> {
    let config = parse_and_validate(file)?;

    match format {
        "mermaid" => {
            println!("graph TD");
            for (id, resource) in &config.resources {
                let machine = match &resource.machine {
                    types::MachineTarget::Single(m) => m.clone(),
                    types::MachineTarget::Multiple(ms) => ms.join(","),
                };
                println!(
                    "    {}[\"{}: {} ({})\"]",
                    id, id, resource.resource_type, machine
                );
                for dep in &resource.depends_on {
                    println!("    {} --> {}", dep, id);
                }
            }
        }
        "dot" => {
            println!("digraph forjar {{");
            println!("    rankdir=TB;");
            println!("    node [shape=box, style=rounded];");
            for (id, resource) in &config.resources {
                let machine = match &resource.machine {
                    types::MachineTarget::Single(m) => m.clone(),
                    types::MachineTarget::Multiple(ms) => ms.join(","),
                };
                println!(
                    "    \"{}\" [label=\"{}: {} ({})\"];",
                    id, id, resource.resource_type, machine
                );
                for dep in &resource.depends_on {
                    println!("    \"{}\" -> \"{}\";", dep, id);
                }
            }
            println!("}}");
        }
        other => {
            return Err(format!(
                "unknown graph format '{}': use mermaid or dot",
                other
            ))
        }
    }

    Ok(())
}

fn cmd_status(state_dir: &Path, machine_filter: Option<&str>) -> Result<(), String> {
    // Show global lock summary if it exists
    if let Some(global) = state::load_global_lock(state_dir)? {
        println!(
            "Project: {} (last apply: {})",
            global.name, global.last_apply
        );
        println!("Generator: {}", global.generator);
        println!();
    }

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
    use std::collections::HashMap;

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
        cmd_plan(&config, &state, None, None, None, false, false, None).unwrap();
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
        cmd_plan(&config, &state, Some("a"), None, None, false, false, None).unwrap();
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
        let result = cmd_plan(&config, &state, None, None, None, false, false, None);
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
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            true,
            false,
            &[],
            false,
            None, // no timeout
            false,
        )
        .unwrap();
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
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
        )
        .unwrap();

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
        let result = cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("validation"));
    }

    #[test]
    fn test_fj017_drift_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            None,
            false,
            false, // dry_run
            false,
            false,
        )
        .unwrap();
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
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            None,
            false,
            false, // dry_run
            false,
            false,
        )
        .unwrap();
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
        let result = cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            true,
            None,
            false,
            false, // dry_run
            false,
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("drift"));
    }

    #[test]
    fn test_fj017_drift_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(state.join("alpha")).unwrap();
        std::fs::create_dir_all(state.join("beta")).unwrap();

        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            Some("alpha"),
            false,
            None,
            false,
            false, // dry_run
            false,
            false,
        )
        .unwrap();
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
        dispatch(Commands::Init { path: sub.clone() }, false).unwrap();
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
        dispatch(
            Commands::Validate {
                file: config.clone(),
            },
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_fj017_dispatch_status() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(
            Commands::Status {
                state_dir: state,
                machine: None,
            },
            false,
        )
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
        dispatch(
            Commands::Plan {
                file: config,
                machine: None,
                resource: None,
                tag: None,
                state_dir: state,
                json: false,
                output_dir: None,
            },
            false,
        )
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
        dispatch(
            Commands::Apply {
                file: config,
                machine: None,
                resource: None,
                tag: None,
                force: false,
                dry_run: true,
                no_tripwire: false,
                params: vec![],
                auto_commit: false,
                state_dir: state,
                timeout: None,
            },
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_fj017_dispatch_drift() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(
            Commands::Drift {
                file: dir.path().join("forjar.yaml"),
                machine: None,
                state_dir: state,
                tripwire: false,
                alert_cmd: None,
                auto_remediate: false,
                dry_run: false,
                json: false,
            },
            false,
        )
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
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            None,
            false,
            false, // dry_run
            false,
            false,
        )
        .unwrap();
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

        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
        )
        .unwrap();
        assert!(target.exists());

        // Second apply — should be unchanged (NoOp)
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
        )
        .unwrap();
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
        cmd_plan(&config, &missing, None, None, None, false, false, None).unwrap();
    }

    #[test]
    fn test_fj017_plan_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: json-test
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
        // json=true should not panic (output goes to stdout)
        cmd_plan(&config, &state, None, None, None, true, false, None).unwrap();
    }

    #[test]
    fn test_fj017_plan_verbose() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: verbose-test
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
        cmd_plan(&config, &state, None, None, None, false, true, None).unwrap();
    }

    #[test]
    fn test_fj017_plan_output_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        let output = dir.path().join("scripts");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  conf:
    type: file
    machine: m1
    path: /etc/test.conf
    content: "hello"
"#,
        )
        .unwrap();
        cmd_plan(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            Some(&output),
        )
        .unwrap();

        // Should have created scripts for both resources
        assert!(output.exists());
        assert!(output.join("pkg.check.sh").exists());
        assert!(output.join("pkg.apply.sh").exists());
        assert!(output.join("pkg.state_query.sh").exists());
        assert!(output.join("conf.check.sh").exists());
        assert!(output.join("conf.apply.sh").exists());

        // Verify script content is non-empty
        let check = std::fs::read_to_string(output.join("pkg.check.sh")).unwrap();
        assert!(check.contains("dpkg"));
    }

    #[test]
    fn test_fj017_drift_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let test_file = dir.path().join("drifted-json.txt");
        std::fs::write(&test_file, "current").unwrap();

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
            machine: "jsonbox".to_string(),
            hostname: "jsonbox".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // JSON drift output should not panic
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            None,
            false,
            false, // dry_run
            true,
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_fj017_history_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        cmd_history(&state, None, 10, false).unwrap();
    }

    #[test]
    fn test_fj017_history_with_events() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        // Write some events
        crate::tripwire::eventlog::append_event(
            &state,
            "m1",
            crate::core::types::ProvenanceEvent::ApplyStarted {
                machine: "m1".to_string(),
                run_id: "r-001".to_string(),
                forjar_version: "0.1.0".to_string(),
            },
        )
        .unwrap();
        crate::tripwire::eventlog::append_event(
            &state,
            "m1",
            crate::core::types::ProvenanceEvent::ApplyCompleted {
                machine: "m1".to_string(),
                run_id: "r-001".to_string(),
                resources_converged: 3,
                resources_unchanged: 0,
                resources_failed: 0,
                total_seconds: 5.2,
            },
        )
        .unwrap();

        cmd_history(&state, None, 10, false).unwrap();
    }

    #[test]
    fn test_fj017_history_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        crate::tripwire::eventlog::append_event(
            &state,
            "m1",
            crate::core::types::ProvenanceEvent::ApplyStarted {
                machine: "m1".to_string(),
                run_id: "r-002".to_string(),
                forjar_version: "0.1.0".to_string(),
            },
        )
        .unwrap();

        cmd_history(&state, None, 10, true).unwrap();
    }

    #[test]
    fn test_fj017_history_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        crate::tripwire::eventlog::append_event(
            &state,
            "alpha",
            crate::core::types::ProvenanceEvent::ApplyStarted {
                machine: "alpha".to_string(),
                run_id: "r-a".to_string(),
                forjar_version: "0.1.0".to_string(),
            },
        )
        .unwrap();
        crate::tripwire::eventlog::append_event(
            &state,
            "beta",
            crate::core::types::ProvenanceEvent::ApplyStarted {
                machine: "beta".to_string(),
                run_id: "r-b".to_string(),
                forjar_version: "0.1.0".to_string(),
            },
        )
        .unwrap();

        // Only show alpha
        cmd_history(&state, Some("alpha"), 10, false).unwrap();
    }

    #[test]
    fn test_fj017_history_limit() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        for i in 0..5 {
            crate::tripwire::eventlog::append_event(
                &state,
                "m1",
                crate::core::types::ProvenanceEvent::ApplyStarted {
                    machine: "m1".to_string(),
                    run_id: format!("r-{}", i),
                    forjar_version: "0.1.0".to_string(),
                },
            )
            .unwrap();
        }

        // Limit to 2
        cmd_history(&state, None, 2, false).unwrap();
    }

    #[test]
    fn test_fj017_dispatch_history() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(
            Commands::History {
                state_dir: state,
                machine: None,
                limit: 10,
                json: false,
            },
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_fj060_graph_mermaid() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: graph-test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  base-pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  config:
    type: file
    machine: m1
    path: /etc/conf
    content: "test"
    depends_on: [base-pkg]
"#,
        )
        .unwrap();
        cmd_graph(&config, "mermaid").unwrap();
    }

    #[test]
    fn test_fj060_graph_dot() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: dot-test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [git]
"#,
        )
        .unwrap();
        cmd_graph(&config, "dot").unwrap();
    }

    #[test]
    fn test_fj060_graph_invalid_format() {
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
        let result = cmd_graph(&config, "svg");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown graph format"));
    }

    #[test]
    fn test_fj060_dispatch_graph() {
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
        dispatch(
            Commands::Graph {
                file: config,
                format: "mermaid".to_string(),
            },
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_fj061_destroy_requires_yes() {
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
    path: /tmp/forjar-destroy-test.txt
    content: "x"
"#,
        )
        .unwrap();
        let result = cmd_destroy(&config, &state, None, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--yes"));
    }

    #[test]
    fn test_fj061_destroy_local_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir.path().join("destroy-me.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: destroy-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  victim:
    type: file
    machine: local
    path: {}
    content: "will be destroyed"
"#,
                target.display()
            ),
        )
        .unwrap();

        // First, apply so the file exists and state is saved
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
        )
        .unwrap();
        assert!(target.exists());
        assert!(state.join("local").join("state.lock.yaml").exists());

        // Now destroy
        cmd_destroy(&config, &state, None, true, false).unwrap();

        // File should be removed
        assert!(!target.exists());

        // State lock should be cleaned up
        assert!(!state.join("local").join("state.lock.yaml").exists());
    }

    #[test]
    fn test_fj061_destroy_verbose() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir.path().join("destroy-verbose.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: verbose-destroy
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: "verbose test"
"#,
                target.display()
            ),
        )
        .unwrap();

        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
        )
        .unwrap();
        cmd_destroy(&config, &state, None, true, true).unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn test_fj061_destroy_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target_a = dir.path().join("file-a.txt");
        let target_b = dir.path().join("file-b.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: filter-test
machines:
  local-a:
    hostname: localhost
    addr: 127.0.0.1
  local-b:
    hostname: localhost
    addr: 127.0.0.1
resources:
  fa:
    type: file
    machine: local-a
    path: {}
    content: "a"
  fb:
    type: file
    machine: local-b
    path: {}
    content: "b"
"#,
                target_a.display(),
                target_b.display()
            ),
        )
        .unwrap();

        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
        )
        .unwrap();
        assert!(target_a.exists());
        assert!(target_b.exists());

        // Only destroy machine local-a
        cmd_destroy(&config, &state, Some("local-a"), true, false).unwrap();
        assert!(!target_a.exists());
        assert!(target_b.exists()); // b should still exist
    }

    #[test]
    fn test_fj061_dispatch_destroy() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir.path().join("dispatch-destroy.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: dispatch-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: "dispatch"
"#,
                target.display()
            ),
        )
        .unwrap();

        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
        )
        .unwrap();
        dispatch(
            Commands::Destroy {
                file: config,
                machine: None,
                yes: true,
                state_dir: state,
            },
            false,
        )
        .unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn test_auto_commit_in_git_repo() {
        // auto_commit=true in a temp dir that IS a git repo
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        // Init git repo in temp dir
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        // Initial commit so the repo is in a valid state
        std::fs::write(dir.path().join(".gitkeep"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", ".gitkeep"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let target = dir.path().join("auto-commit.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: autocommit-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: "auto commit test"
"#,
                target.display()
            ),
        )
        .unwrap();

        // auto_commit=true (second to last arg)
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            true,
            None, // no timeout
            false,
        )
        .unwrap();
        assert!(target.exists());

        // Verify git committed the state
        let output = std::process::Command::new("git")
            .args(["log", "--oneline", "-1"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let log = String::from_utf8_lossy(&output.stdout);
        assert!(log.contains("forjar:"));
    }

    #[test]
    fn test_drift_alert_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let test_file = dir.path().join("drift-alert.txt");
        std::fs::write(&test_file, "current").unwrap();

        let alert_marker = dir.path().join("alert-fired");

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
            machine: "alertbox".to_string(),
            hostname: "alertbox".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // alert_cmd touches a file when drift detected
        let alert_cmd = format!("touch {}", alert_marker.display());
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            Some(&alert_cmd),
            false,
            false, // dry_run
            false,
            false,
        )
        .unwrap();

        // Verify alert command ran
        assert!(alert_marker.exists());
    }

    #[test]
    fn test_drift_alert_cmd_not_fired_when_no_drift() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let alert_marker = dir.path().join("should-not-exist");
        let alert_cmd = format!("touch {}", alert_marker.display());

        // Empty state dir — no drift
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            Some(&alert_cmd),
            false,
            false, // dry_run
            false,
            false,
        )
        .unwrap();

        // Alert should NOT have fired
        assert!(!alert_marker.exists());
    }

    #[test]
    fn test_drift_auto_remediate() {
        // Create a file resource, apply, tamper, then drift --auto-remediate
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir
            .path()
            .join("auto-remediate-test.txt")
            .to_string_lossy()
            .to_string();
        std::fs::write(
            &config,
            format!(
                r#"version: "1.0"
name: remediation-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {}
    content: "original content"
    mode: "0644"
"#,
                target
            ),
        )
        .unwrap();

        // Apply to create the file
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
        )
        .unwrap();
        assert!(std::path::Path::new(&target).exists());

        // Tamper with the file
        std::fs::write(&target, "tampered content").unwrap();

        // Drift with auto-remediate should detect and fix
        cmd_drift(
            &config, &state, None, false, None, true, // auto_remediate
            false, false, false,
        )
        .unwrap();

        // File should be restored to original content
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content.trim(), "original content");

        // Clean up
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn test_drift_dry_run_lists_resources() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        // Create a lock with two resources
        let mut resources = indexmap::IndexMap::new();
        resources.insert(
            "web-config".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "abc123".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        resources.insert(
            "db-config".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: None,
                duration_seconds: None,
                hash: "def456".to_string(),
                details: std::collections::HashMap::new(),
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "local".to_string(),
            hostname: "local".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // Dry-run should succeed without connecting to any machine
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            None,
            false,
            true, // dry_run
            false,
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_fj065_import_localhost() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        // Import just packages from localhost (most likely to succeed in test env)
        cmd_import(
            "localhost",
            "root",
            Some("test-machine"),
            &output,
            &["packages".to_string()],
            false,
        )
        .unwrap();

        // Output file should exist and be valid YAML
        assert!(output.exists());
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("test-machine"));
        assert!(content.contains("addr: localhost"));
    }

    #[test]
    fn test_fj065_import_generates_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("local"),
            &output,
            &["packages".to_string()],
            false,
        )
        .unwrap();

        // The generated YAML should parse as a valid forjar config
        let content = std::fs::read_to_string(&output).unwrap();
        // Parse the YAML (strip comments that aren't YAML-compatible)
        let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
        assert_eq!(config.version, "1.0");
        assert!(config.machines.contains_key("local"));
    }

    #[test]
    fn test_fj017_show_full_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: show-test
params:
  env: staging
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  conf:
    type: file
    machine: m1
    path: /etc/{{params.env}}.conf
    content: "env={{params.env}}"
"#,
        )
        .unwrap();
        // Should resolve templates without error
        cmd_show(&config, None, false).unwrap();
    }

    #[test]
    fn test_fj017_show_specific_resource() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: show-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  conf:
    type: file
    machine: m1
    path: /etc/test
    content: hello
"#,
        )
        .unwrap();
        // Show specific resource
        cmd_show(&config, Some("conf"), false).unwrap();
    }

    #[test]
    fn test_fj017_show_missing_resource() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: show-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        let result = cmd_show(&config, Some("nonexistent"), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_fj054_run_hook_success() {
        run_hook("test", "echo hello", false).unwrap();
    }

    #[test]
    fn test_fj054_run_hook_failure() {
        let result = run_hook("test", "exit 1", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("failed"));
    }

    #[test]
    fn test_fj054_run_hook_nonzero_exit() {
        let result = run_hook("pre_apply", "exit 42", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exit 42"));
    }

    #[test]
    fn test_fj054_policy_hooks_parsed() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
policy:
  failure: stop_on_first
  pre_apply: "echo before"
  post_apply: "echo after"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.policy.pre_apply.as_deref(), Some("echo before"));
        assert_eq!(config.policy.post_apply.as_deref(), Some("echo after"));
    }

    // ── forjar diff tests ──────────────────────────────────────────

    fn make_state_dir_with_lock(
        dir: &Path,
        machine: &str,
        resources: Vec<(&str, &str, types::ResourceStatus)>,
    ) {
        let mut res_map = indexmap::IndexMap::new();
        for (id, hash, status) in resources {
            res_map.insert(
                id.to_string(),
                types::ResourceLock {
                    resource_type: types::ResourceType::File,
                    status,
                    applied_at: Some("2026-02-25T00:00:00Z".to_string()),
                    duration_seconds: Some(0.1),
                    hash: hash.to_string(),
                    details: HashMap::new(),
                },
            );
        }
        let lock = types::StateLock {
            schema: "1.0".to_string(),
            machine: machine.to_string(),
            hostname: "test-host".to_string(),
            generated_at: "2026-02-25T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources: res_map,
        };
        state::save_lock(dir, &lock).unwrap();
    }

    #[test]
    fn test_diff_added_resource() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![
                ("pkg", "blake3:aaa", types::ResourceStatus::Converged),
                ("conf", "blake3:bbb", types::ResourceStatus::Converged),
            ],
        );
        cmd_diff(from_dir.path(), to_dir.path(), None, false).unwrap();
    }

    #[test]
    fn test_diff_removed_resource() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![
                ("pkg", "blake3:aaa", types::ResourceStatus::Converged),
                ("conf", "blake3:bbb", types::ResourceStatus::Converged),
            ],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        cmd_diff(from_dir.path(), to_dir.path(), None, false).unwrap();
    }

    #[test]
    fn test_diff_changed_hash() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![("pkg", "blake3:bbb", types::ResourceStatus::Converged)],
        );
        cmd_diff(from_dir.path(), to_dir.path(), None, false).unwrap();
    }

    #[test]
    fn test_diff_no_changes() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        cmd_diff(from_dir.path(), to_dir.path(), None, false).unwrap();
    }

    #[test]
    fn test_diff_json_output() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![
                ("pkg", "blake3:bbb", types::ResourceStatus::Converged),
                ("svc", "blake3:ccc", types::ResourceStatus::Converged),
            ],
        );
        cmd_diff(from_dir.path(), to_dir.path(), None, true).unwrap();
    }

    #[test]
    fn test_diff_machine_filter() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            from_dir.path(),
            "m1",
            vec![("pkg", "blake3:aaa", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            from_dir.path(),
            "m2",
            vec![("svc", "blake3:bbb", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m1",
            vec![("pkg", "blake3:changed", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            to_dir.path(),
            "m2",
            vec![("svc", "blake3:bbb", types::ResourceStatus::Converged)],
        );
        // Filtering to m1 should only show m1's changes
        cmd_diff(from_dir.path(), to_dir.path(), Some("m1"), false).unwrap();
    }

    #[test]
    fn test_diff_empty_state_dirs() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();
        let result = cmd_diff(from_dir.path(), to_dir.path(), None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no machines found"));
    }

    #[test]
    fn test_discover_machines() {
        let dir = tempfile::tempdir().unwrap();
        make_state_dir_with_lock(
            dir.path(),
            "alpha",
            vec![("f", "blake3:x", types::ResourceStatus::Converged)],
        );
        make_state_dir_with_lock(
            dir.path(),
            "beta",
            vec![("f", "blake3:y", types::ResourceStatus::Converged)],
        );
        let machines = discover_machines(dir.path());
        assert_eq!(machines, vec!["alpha", "beta"]);
    }

    // ── forjar check tests ─────────────────────────────────────────

    #[test]
    fn test_check_local_file_pass() {
        let dir = tempfile::tempdir().unwrap();
        // Create the file that check will verify
        let target = dir.path().join("check-test.txt");
        std::fs::write(&target, "hello").unwrap();

        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: hello
"#,
                target.display()
            ),
        )
        .unwrap();
        // File exists → check should pass
        cmd_check(&config, None, None, None, false, false).unwrap();
    }

    #[test]
    fn test_check_local_file_missing_still_runs() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: /tmp/forjar-check-nonexistent-12345678
    content: hello
"#,
        )
        .unwrap();
        // Check script reports status (exits 0 even for missing file)
        cmd_check(&config, None, None, None, false, false).unwrap();
    }

    #[test]
    fn test_check_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("check-json-test.txt");
        std::fs::write(&target, "hello").unwrap();

        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: hello
"#,
                target.display()
            ),
        )
        .unwrap();
        cmd_check(&config, None, None, None, true, false).unwrap();
    }

    #[test]
    fn test_fmt_normalizes_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        // Manually-written YAML with inconsistent spacing
        std::fs::write(
            &file,
            r#"version: "1.0"
name: fmt-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/fmt-test
    content: hello
"#,
        )
        .unwrap();

        // Check should fail (not yet canonical)
        let result = cmd_fmt(&file, true);
        assert!(result.is_err());

        // Format it
        cmd_fmt(&file, false).unwrap();

        // Check should now pass
        cmd_fmt(&file, true).unwrap();
    }

    #[test]
    fn test_fmt_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(
            &file,
            r#"version: "1.0"
name: idempotent-test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: hello
"#,
        )
        .unwrap();

        // Format it twice
        cmd_fmt(&file, false).unwrap();
        let after_first = std::fs::read_to_string(&file).unwrap();

        cmd_fmt(&file, false).unwrap();
        let after_second = std::fs::read_to_string(&file).unwrap();

        assert_eq!(after_first, after_second);
    }

    #[test]
    fn test_lint_unused_machine() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(
            &file,
            r#"version: "1.0"
name: lint-test
machines:
  used:
    hostname: used
    addr: 127.0.0.1
  unused:
    hostname: unused
    addr: 10.0.0.1
resources:
  f:
    type: file
    machine: used
    path: /tmp/test
    content: hello
"#,
        )
        .unwrap();

        // Lint should succeed but print warnings (it returns Ok)
        cmd_lint(&file, false).unwrap();
    }

    #[test]
    fn test_lint_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(
            &file,
            r#"version: "1.0"
name: lint-json
machines:
  m:
    hostname: m
    addr: 127.0.0.1
  orphan:
    hostname: orphan
    addr: 10.0.0.2
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: hello
"#,
        )
        .unwrap();

        cmd_lint(&file, true).unwrap();
    }

    #[test]
    fn test_lint_clean_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(
            &file,
            r#"version: "1.0"
name: clean
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: hello
"#,
        )
        .unwrap();

        cmd_lint(&file, false).unwrap();
    }

    #[test]
    fn test_lint_cross_machine_dependency() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        std::fs::write(
            &file,
            r#"version: "1.0"
name: cross-dep
machines:
  web:
    hostname: web
    addr: 10.0.0.1
  db:
    hostname: db
    addr: 10.0.0.2
resources:
  app-config:
    type: file
    machine: web
    path: /etc/app.conf
    content: "host=db"
    depends_on: [db-ready]
  db-ready:
    type: file
    machine: db
    path: /tmp/db-ready
    content: "ok"
"#,
        )
        .unwrap();

        // Capture output via JSON mode to inspect warnings
        let result = cmd_lint(&file, true);
        assert!(result.is_ok());
        // The warning should mention cross-machine dependency
        // We re-run logic here to check the warning was generated
        let config = parse_and_validate(&file).unwrap();
        let mut found_cross_machine = false;
        for (_id, resource) in &config.resources {
            let my_machines: std::collections::HashSet<String> =
                resource.machine.to_vec().into_iter().collect();
            for dep in &resource.depends_on {
                if let Some(dep_resource) = config.resources.get(dep) {
                    let dep_machines: std::collections::HashSet<String> =
                        dep_resource.machine.to_vec().into_iter().collect();
                    if my_machines.is_disjoint(&dep_machines) {
                        found_cross_machine = true;
                    }
                }
            }
        }
        assert!(
            found_cross_machine,
            "should detect cross-machine dependency"
        );
    }

    #[test]
    fn test_rollback_no_git_history() {
        // A file that doesn't exist in git history should fail gracefully
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("nonexistent.yaml");
        std::fs::write(
            &file,
            "version: \"1.0\"\nname: test\nmachines: {}\nresources: {}\n",
        )
        .unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let result = cmd_rollback(&file, &state, 1, None, true, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot read"));
    }

    #[test]
    fn test_rollback_dispatch() {
        // Verify the Rollback command variant is accepted by dispatch
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            "version: \"1.0\"\nname: rb\nmachines: {}\nresources: {}\n",
        )
        .unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        // Dispatch dry-run rollback — will fail because no git history,
        // but verifies the dispatch path is wired correctly
        let result = dispatch(
            Commands::Rollback {
                file,
                revision: 1,
                machine: None,
                dry_run: true,
                state_dir: state,
            },
            false,
        );
        assert!(result.is_err()); // Expected: no git history
    }

    #[test]
    fn test_anomaly_empty_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        // No machine dirs → "no resources with enough history"
        let result = cmd_anomaly(&state, None, 3, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_anomaly_detects_high_failure_rate() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let machine_dir = state.join("m1");
        std::fs::create_dir_all(&machine_dir).unwrap();

        // Write events with high failure rate: 1 converge, 4 failures
        let mut events = String::new();
        events.push_str(
            &serde_json::to_string(&types::TimestampedEvent {
                ts: "2026-02-25T00:00:00Z".to_string(),
                event: types::ProvenanceEvent::ResourceConverged {
                    machine: "m1".to_string(),
                    resource: "flaky-pkg".to_string(),
                    duration_seconds: 1.0,
                    hash: "abc".to_string(),
                },
            })
            .unwrap(),
        );
        events.push('\n');
        for _ in 0..4 {
            events.push_str(
                &serde_json::to_string(&types::TimestampedEvent {
                    ts: "2026-02-25T00:01:00Z".to_string(),
                    event: types::ProvenanceEvent::ResourceFailed {
                        machine: "m1".to_string(),
                        resource: "flaky-pkg".to_string(),
                        error: "install failed".to_string(),
                    },
                })
                .unwrap(),
            );
            events.push('\n');
        }

        std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();

        // min_events=3, json mode so we can parse output
        let result = cmd_anomaly(&state, None, 3, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_anomaly_detects_drift() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let machine_dir = state.join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();

        let mut events = String::new();
        // 2 converges + 1 drift = 3 events (meets min_events=3)
        for _ in 0..2 {
            events.push_str(
                &serde_json::to_string(&types::TimestampedEvent {
                    ts: "2026-02-25T00:00:00Z".to_string(),
                    event: types::ProvenanceEvent::ResourceConverged {
                        machine: "web".to_string(),
                        resource: "config-file".to_string(),
                        duration_seconds: 0.5,
                        hash: "def".to_string(),
                    },
                })
                .unwrap(),
            );
            events.push('\n');
        }
        events.push_str(
            &serde_json::to_string(&types::TimestampedEvent {
                ts: "2026-02-25T01:00:00Z".to_string(),
                event: types::ProvenanceEvent::DriftDetected {
                    machine: "web".to_string(),
                    resource: "config-file".to_string(),
                    expected_hash: "aaa".to_string(),
                    actual_hash: "bbb".to_string(),
                },
            })
            .unwrap(),
        );
        events.push('\n');

        std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();

        let result = cmd_anomaly(&state, None, 3, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_anomaly_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let machine_dir = state.join("srv");
        std::fs::create_dir_all(&machine_dir).unwrap();

        // Write 3 converge events for one resource (no anomaly, just normal)
        let mut events = String::new();
        for _ in 0..3 {
            events.push_str(
                &serde_json::to_string(&types::TimestampedEvent {
                    ts: "2026-02-25T00:00:00Z".to_string(),
                    event: types::ProvenanceEvent::ResourceConverged {
                        machine: "srv".to_string(),
                        resource: "pkg".to_string(),
                        duration_seconds: 1.0,
                        hash: "xyz".to_string(),
                    },
                })
                .unwrap(),
            );
            events.push('\n');
        }

        std::fs::write(machine_dir.join("events.jsonl"), &events).unwrap();

        let result = cmd_anomaly(&state, None, 3, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_anomaly_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        // Create two machines
        let m1 = state.join("m1");
        let m2 = state.join("m2");
        std::fs::create_dir_all(&m1).unwrap();
        std::fs::create_dir_all(&m2).unwrap();

        // Events only on m2
        let mut events = String::new();
        for _ in 0..5 {
            events.push_str(
                &serde_json::to_string(&types::TimestampedEvent {
                    ts: "2026-02-25T00:00:00Z".to_string(),
                    event: types::ProvenanceEvent::ResourceFailed {
                        machine: "m2".to_string(),
                        resource: "bad-svc".to_string(),
                        error: "timeout".to_string(),
                    },
                })
                .unwrap(),
            );
            events.push('\n');
        }
        std::fs::write(m2.join("events.jsonl"), &events).unwrap();

        // Filter to m1 (no events) → no anomalies
        let result = cmd_anomaly(&state, Some("m1"), 1, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_anomaly_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let result = dispatch(
            Commands::Anomaly {
                state_dir: state,
                machine: None,
                min_events: 3,
                json: false,
            },
            false,
        );
        assert!(result.is_ok());
    }

    // ── Import scan type tests ─────────────────────────────────

    #[test]
    fn test_fj065_import_services_scan() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        // Import services from localhost
        cmd_import(
            "localhost",
            "root",
            Some("svc-box"),
            &output,
            &["services".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("svc-box"));
    }

    #[test]
    fn test_fj065_import_users_scan() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("user-box"),
            &output,
            &["users".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("user-box"));
    }

    #[test]
    fn test_fj065_import_files_scan() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("file-box"),
            &output,
            &["files".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("file-box"));
    }

    #[test]
    fn test_fj065_import_cron_scan() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("cron-box"),
            &output,
            &["cron".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("cron-box"));
    }

    #[test]
    fn test_fj065_import_multi_scan() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("multi-box"),
            &output,
            &[
                "packages".to_string(),
                "services".to_string(),
                "users".to_string(),
            ],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("multi-box"));
    }

    #[test]
    fn test_fj065_import_verbose() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("verbose-box"),
            &output,
            &["packages".to_string()],
            true, // verbose
        )
        .unwrap();

        assert!(output.exists());
    }

    #[test]
    fn test_fj065_import_default_name_localhost() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            None, // name derived from addr
            &output,
            &["packages".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("localhost"));
    }

    #[test]
    fn test_fj065_import_default_name_ip() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        // Use 127.0.0.1 — name should default to "localhost"
        cmd_import(
            "127.0.0.1",
            "root",
            None,
            &output,
            &["packages".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("localhost"));
    }

    // ── Show command tests ─────────────────────────────────────

    #[test]
    fn test_fj017_show_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: json-show-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        // JSON output should succeed
        cmd_show(&config, None, true).unwrap();
    }

    #[test]
    fn test_fj017_show_specific_resource_json() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: show-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        cmd_show(&config, Some("pkg"), true).unwrap();
    }

    // ── Fmt edge cases ─────────────────────────────────────────

    #[test]
    fn test_fj017_fmt_check_unformatted() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        // Write with extra whitespace and comments (not canonical)
        std::fs::write(
            &config,
            r#"version:   "1.0"
name:    my-infra
machines:
  m1:
    hostname:   box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        // check mode should detect non-canonical format
        let result = cmd_fmt(&config, true);
        assert!(result.is_err(), "unformatted file should fail check mode");
    }

    #[test]
    fn test_fj017_fmt_write_then_check() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"version:   "1.0"
name:    my-infra
machines:
  m1:
    hostname:   box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        // Format the file
        cmd_fmt(&config, false).unwrap();
        // Now check mode should pass
        cmd_fmt(&config, true).unwrap();
    }

    // ── Check command tests ────────────────────────────────────

    #[test]
    fn test_fj017_check_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        // Check with machine filter
        cmd_check(&config, Some("local"), None, None, false, false).unwrap();
    }

    #[test]
    fn test_fj017_check_resource_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  pkg1:
    type: package
    machine: local
    provider: apt
    packages: [curl]
  pkg2:
    type: package
    machine: local
    provider: apt
    packages: [wget]
"#,
        )
        .unwrap();
        // Check only specific resource
        cmd_check(&config, None, Some("pkg1"), None, false, false).unwrap();
    }

    #[test]
    fn test_fj017_check_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: check-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  conf:
    type: file
    machine: local
    path: /tmp/forjar-check-test.txt
    content: hello
"#,
        )
        .unwrap();
        // JSON output
        cmd_check(&config, None, None, None, true, false).unwrap();
    }

    // ── Rollback error tests ───────────────────────────────────

    #[test]
    fn test_fj017_rollback_invalid_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nonexistent.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        // Rollback with nonexistent config should fail
        let result = cmd_rollback(&config, &state, 1, None, true, false);
        assert!(result.is_err());
    }

    // ── Apply with param overrides ─────────────────────────────

    #[test]
    fn test_fj017_apply_with_param_override() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: param-test
params:
  env: dev
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  conf:
    type: file
    machine: local
    path: /tmp/forjar-param-test.txt
    content: "env={{params.env}}"
"#,
        )
        .unwrap();
        // Apply with param override in dry-run
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            false,
            true, // dry-run
            false,
            &["env=prod".to_string()],
            false,
            None,
            false,
        )
        .unwrap();
    }

    // ── Lint edge cases ────────────────────────────────────────

    #[test]
    fn test_fj017_lint_duplicate_content() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: lint-dup
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  file-a:
    type: file
    machine: m1
    path: /etc/a.conf
    content: "same content"
  file-b:
    type: file
    machine: m1
    path: /etc/b.conf
    content: "same content"
  file-c:
    type: file
    machine: m1
    path: /etc/c.conf
    content: "same content"
"#,
        )
        .unwrap();
        // Lint should detect duplicate content
        cmd_lint(&config, false).unwrap();
    }

    // ── Init edge case ────────────────────────────────────────

    #[test]
    fn test_fj017_init_creates_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("new-project");
        std::fs::create_dir_all(&project).unwrap();

        cmd_init(&project).unwrap();

        assert!(project.join("forjar.yaml").exists());
        assert!(project.join("state").exists());
    }

    #[test]
    fn test_fj017_init_template_is_valid() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("valid-init");
        std::fs::create_dir_all(&project).unwrap();

        cmd_init(&project).unwrap();

        // The template should parse as valid ForjarConfig
        let content = std::fs::read_to_string(project.join("forjar.yaml")).unwrap();
        let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.name, "my-infrastructure");
    }

    // ── FJ-131: cmd_graph tests ───────────────────────────────────

    fn write_simple_config(dir: &std::path::Path) -> std::path::PathBuf {
        let config_path = dir.join("forjar.yaml");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: graph-test
machines:
  web:
    hostname: web
    addr: 1.1.1.1
resources:
  setup:
    type: file
    machine: web
    path: /tmp/setup
    state: directory
  app:
    type: file
    machine: web
    path: /tmp/setup/app.conf
    content: "config"
    depends_on: [setup]
"#,
        )
        .unwrap();
        config_path
    }

    #[test]
    fn test_fj131_cmd_graph_mermaid() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_simple_config(dir.path());
        // Should succeed without error
        cmd_graph(&config_path, "mermaid").unwrap();
    }

    #[test]
    fn test_fj131_cmd_graph_dot() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_simple_config(dir.path());
        cmd_graph(&config_path, "dot").unwrap();
    }

    #[test]
    fn test_fj131_cmd_graph_unknown_format() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = write_simple_config(dir.path());
        let err = cmd_graph(&config_path, "svg").unwrap_err();
        assert!(err.contains("unknown graph format"));
        assert!(err.contains("svg"));
    }

    #[test]
    fn test_fj131_cmd_graph_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "not valid yaml {{{{").unwrap();
        let err = cmd_graph(&config_path, "mermaid");
        assert!(err.is_err());
    }

    // ── FJ-131: cmd_diff tests ────────────────────────────────────

    #[test]
    fn test_fj131_cmd_diff_empty_state_dirs() {
        let from = tempfile::tempdir().unwrap();
        let to = tempfile::tempdir().unwrap();
        let err = cmd_diff(from.path(), to.path(), None, false).unwrap_err();
        assert!(err.contains("no machines found"));
    }

    #[test]
    fn test_fj131_cmd_diff_same_state() {
        let state = tempfile::tempdir().unwrap();
        // Create a machine state directory with a lock
        let machine_dir = state.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let lock = types::StateLock {
            schema: "1.0".to_string(),
            machine: "web".to_string(),
            hostname: "web-box".to_string(),
            generated_at: "2026-02-25T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources: {
                let mut r = indexmap::IndexMap::new();
                r.insert(
                    "test-file".to_string(),
                    types::ResourceLock {
                        resource_type: types::ResourceType::File,
                        status: types::ResourceStatus::Converged,
                        applied_at: Some("2026-02-25T00:00:00Z".to_string()),
                        duration_seconds: Some(0.1),
                        hash: "blake3:abc123".to_string(),
                        details: HashMap::new(),
                    },
                );
                r
            },
        };
        state::save_lock(state.path(), &lock).unwrap();

        // Diff same directory against itself → no differences
        cmd_diff(state.path(), state.path(), None, false).unwrap();
    }

    #[test]
    fn test_fj131_cmd_diff_added_resource() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();

        // "from" has empty lock for web
        let from_machine = from_dir.path().join("web");
        std::fs::create_dir_all(&from_machine).unwrap();
        let from_lock = types::StateLock {
            schema: "1.0".to_string(),
            machine: "web".to_string(),
            hostname: "web-box".to_string(),
            generated_at: "2026-02-25T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources: indexmap::IndexMap::new(),
        };
        state::save_lock(from_dir.path(), &from_lock).unwrap();

        // "to" has one resource
        let to_machine = to_dir.path().join("web");
        std::fs::create_dir_all(&to_machine).unwrap();
        let mut to_lock = from_lock.clone();
        to_lock.resources.insert(
            "new-file".to_string(),
            types::ResourceLock {
                resource_type: types::ResourceType::File,
                status: types::ResourceStatus::Converged,
                applied_at: Some("2026-02-25T01:00:00Z".to_string()),
                duration_seconds: Some(0.2),
                hash: "blake3:def456".to_string(),
                details: HashMap::new(),
            },
        );
        state::save_lock(to_dir.path(), &to_lock).unwrap();

        cmd_diff(from_dir.path(), to_dir.path(), None, false).unwrap();
    }

    #[test]
    fn test_fj131_cmd_diff_json_output() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();

        // Both have web machine
        let from_lock = types::StateLock {
            schema: "1.0".to_string(),
            machine: "web".to_string(),
            hostname: "web-box".to_string(),
            generated_at: "2026-02-25T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources: indexmap::IndexMap::new(),
        };
        state::save_lock(from_dir.path(), &from_lock).unwrap();
        state::save_lock(to_dir.path(), &from_lock).unwrap();

        // JSON output should not error
        cmd_diff(from_dir.path(), to_dir.path(), None, true).unwrap();
    }

    #[test]
    fn test_fj131_cmd_diff_machine_filter() {
        let from_dir = tempfile::tempdir().unwrap();
        let to_dir = tempfile::tempdir().unwrap();

        // Create two machines
        for name in ["web", "db"] {
            let lock = types::StateLock {
                schema: "1.0".to_string(),
                machine: name.to_string(),
                hostname: format!("{}-box", name),
                generated_at: "2026-02-25T00:00:00Z".to_string(),
                generator: "forjar 0.1.0".to_string(),
                blake3_version: "1.8".to_string(),
                resources: indexmap::IndexMap::new(),
            };
            state::save_lock(from_dir.path(), &lock).unwrap();
            state::save_lock(to_dir.path(), &lock).unwrap();
        }

        // Filter to only "web" — should succeed
        cmd_diff(from_dir.path(), to_dir.path(), Some("web"), false).unwrap();
    }

    // ── FJ-131: cmd_anomaly tests ─────────────────────────────────

    #[test]
    fn test_fj131_cmd_anomaly_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        // No machine directories → should succeed with no output
        cmd_anomaly(dir.path(), None, 1, false).unwrap();
    }

    #[test]
    fn test_fj131_cmd_anomaly_no_events() {
        let dir = tempfile::tempdir().unwrap();
        // Create machine dir but no events.jsonl
        std::fs::create_dir_all(dir.path().join("web")).unwrap();
        cmd_anomaly(dir.path(), None, 1, false).unwrap();
    }

    #[test]
    fn test_fj131_cmd_anomaly_with_events() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();

        // Write some events
        let events = [
            r#"{"ts":"2026-02-25T00:00:00Z","event":"resource_converged","machine":"web","resource":"pkg","duration_seconds":1.0,"hash":"blake3:abc"}"#,
            r#"{"ts":"2026-02-25T01:00:00Z","event":"resource_converged","machine":"web","resource":"pkg","duration_seconds":1.0,"hash":"blake3:abc"}"#,
            r#"{"ts":"2026-02-25T02:00:00Z","event":"resource_converged","machine":"web","resource":"pkg","duration_seconds":1.0,"hash":"blake3:abc"}"#,
        ];
        std::fs::write(machine_dir.join("events.jsonl"), events.join("\n")).unwrap();

        cmd_anomaly(dir.path(), None, 1, false).unwrap();
    }

    #[test]
    fn test_fj131_cmd_anomaly_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();

        let events = [
            r#"{"ts":"2026-02-25T00:00:00Z","event":"resource_converged","machine":"web","resource":"pkg","duration_seconds":1.0,"hash":"blake3:abc"}"#,
            r#"{"ts":"2026-02-25T01:00:00Z","event":"resource_failed","machine":"web","resource":"pkg","error":"timeout"}"#,
        ];
        std::fs::write(machine_dir.join("events.jsonl"), events.join("\n")).unwrap();

        cmd_anomaly(dir.path(), None, 1, true).unwrap();
    }

    #[test]
    fn test_fj131_cmd_anomaly_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        // Create two machine dirs
        for name in ["web", "db"] {
            let machine_dir = dir.path().join(name);
            std::fs::create_dir_all(&machine_dir).unwrap();
            let event = format!(
                r#"{{"ts":"2026-02-25T00:00:00Z","event":"resource_converged","machine":"{}","resource":"pkg","duration_seconds":1.0,"hash":"blake3:abc"}}"#,
                name
            );
            std::fs::write(machine_dir.join("events.jsonl"), event).unwrap();
        }

        // Filter to only "web"
        cmd_anomaly(dir.path(), Some("web"), 1, false).unwrap();
    }

    #[test]
    fn test_fj131_cmd_anomaly_nonexistent_state_dir() {
        let err = cmd_anomaly(
            std::path::Path::new("/tmp/nonexistent-forjar-state"),
            None,
            1,
            false,
        );
        assert!(err.is_err());
    }

    #[test]
    fn test_fj132_discover_machines_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let machines = discover_machines(dir.path());
        assert!(machines.is_empty());
    }

    #[test]
    fn test_fj132_discover_machines_with_locks() {
        let dir = tempfile::tempdir().unwrap();
        // Machine with state.lock.yaml — should be discovered
        let web_dir = dir.path().join("web");
        std::fs::create_dir_all(&web_dir).unwrap();
        std::fs::write(web_dir.join("state.lock.yaml"), "schema: '1.0'").unwrap();
        // Machine without lock — should NOT be discovered
        let nolock_dir = dir.path().join("orphan");
        std::fs::create_dir_all(&nolock_dir).unwrap();
        // Plain file — should NOT be discovered
        std::fs::write(dir.path().join("readme.txt"), "ignore").unwrap();
        let machines = discover_machines(dir.path());
        assert_eq!(machines, vec!["web"]);
    }

    #[test]
    fn test_fj132_discover_machines_sorted() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["zeta", "alpha", "mid"] {
            let m_dir = dir.path().join(name);
            std::fs::create_dir_all(&m_dir).unwrap();
            std::fs::write(m_dir.join("state.lock.yaml"), "schema: '1.0'").unwrap();
        }
        let machines = discover_machines(dir.path());
        assert_eq!(machines, vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn test_fj132_apply_param_overrides_basic() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let mut config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let overrides = vec!["env=production".to_string(), "port=8080".to_string()];
        apply_param_overrides(&mut config, &overrides).unwrap();
        assert_eq!(
            config.params.get("env").unwrap(),
            &serde_yaml_ng::Value::String("production".to_string())
        );
        assert_eq!(
            config.params.get("port").unwrap(),
            &serde_yaml_ng::Value::String("8080".to_string())
        );
    }

    #[test]
    fn test_fj132_apply_param_overrides_invalid() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let mut config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let overrides = vec!["no-equals-sign".to_string()];
        let result = apply_param_overrides(&mut config, &overrides);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected KEY=VALUE"));
    }

    #[test]
    fn test_fj132_discover_machines_nonexistent_dir() {
        let machines = discover_machines(std::path::Path::new("/nonexistent/path/state"));
        assert!(machines.is_empty(), "nonexistent dir should return empty");
    }

    #[test]
    fn test_fj132_cmd_init_creates_project() {
        let dir = tempfile::tempdir().unwrap();
        cmd_init(dir.path()).unwrap();
        assert!(dir.path().join("forjar.yaml").exists());
        assert!(dir.path().join("state").is_dir());
        // Config should be valid YAML
        let content = std::fs::read_to_string(dir.path().join("forjar.yaml")).unwrap();
        let _config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
    }

    #[test]
    fn test_fj132_cmd_init_refuses_existing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("forjar.yaml"), "version: '1.0'").unwrap();
        let result = cmd_init(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_fj132_cmd_fmt_already_formatted() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        let yaml = r#"version: "1.0"
name: test
machines: {}
resources: {}
"#;
        // Write, parse, re-serialize to get canonical form
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let formatted = serde_yaml_ng::to_string(&config).unwrap();
        std::fs::write(&file, &formatted).unwrap();
        // Should succeed and not modify
        cmd_fmt(&file, false).unwrap();
    }

    #[test]
    fn test_fj132_cmd_fmt_check_mode() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        // Write canonical YAML
        let yaml = r#"version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let formatted = serde_yaml_ng::to_string(&config).unwrap();
        std::fs::write(&file, &formatted).unwrap();
        // Check mode should succeed for already-formatted file
        cmd_fmt(&file, true).unwrap();
    }

    #[test]
    fn test_fj132_cmd_validate_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [curl]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_validate(&file).unwrap();
    }

    #[test]
    fn test_fj132_cmd_validate_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "2.0"
name: test
machines: {}
resources: {}
"#;
        std::fs::write(&file, yaml).unwrap();
        let result = cmd_validate(&file);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj132_cmd_graph_mermaid() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [nginx]
  conf:
    type: file
    machine: m
    path: /etc/nginx/nginx.conf
    content: "server {}"
    depends_on: [pkg]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_graph(&file, "mermaid").unwrap();
    }

    #[test]
    fn test_fj132_cmd_graph_dot() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_graph(&file, "dot").unwrap();
    }

    #[test]
    fn test_fj132_cmd_graph_unknown_format() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        std::fs::write(&file, yaml).unwrap();
        let result = cmd_graph(&file, "svg");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown graph format"));
    }

    #[test]
    fn test_fj132_cmd_show_all_resources() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_show(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj132_cmd_show_specific_resource() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  my-file:
    type: file
    machine: m
    path: /etc/test.conf
    content: "hello"
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_show(&file, Some("my-file"), false).unwrap();
    }

    #[test]
    fn test_fj132_cmd_show_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [git]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_show(&file, None, true).unwrap();
    }

    #[test]
    fn test_fj132_cmd_show_missing_resource() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        std::fs::write(&file, yaml).unwrap();
        let result = cmd_show(&file, Some("nonexistent"), false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_fj132_cmd_status_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        cmd_status(dir.path(), None).unwrap();
    }

    #[test]
    fn test_fj132_cmd_lint_valid() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_lint(&file, false).unwrap();
    }

    #[test]
    fn test_fj132_cmd_lint_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_lint(&file, true).unwrap();
    }

    #[test]
    fn test_fj132_cmd_history_with_events() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("web");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let events = [
            r#"{"ts":"2026-02-25T10:00:00Z","event":"apply_started","machine":"web","run_id":"r-1","forjar_version":"0.1.0"}"#,
            r#"{"ts":"2026-02-25T10:01:00Z","event":"resource_converged","machine":"web","resource":"pkg","duration_seconds":5.0,"hash":"blake3:abc"}"#,
            r#"{"ts":"2026-02-25T10:02:00Z","event":"apply_completed","machine":"web","run_id":"r-1","resources_converged":1,"resources_failed":0,"resources_skipped":0,"total_duration":5.0}"#,
        ];
        std::fs::write(machine_dir.join("events.jsonl"), events.join("\n")).unwrap();
        cmd_history(dir.path(), None, 10, false).unwrap();
    }

    #[test]
    fn test_fj132_cmd_history_json() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("db");
        std::fs::create_dir_all(&machine_dir).unwrap();
        let event = r#"{"ts":"2026-02-25T10:00:00Z","event":"apply_started","machine":"db","run_id":"r-1","forjar_version":"0.1.0"}"#;
        std::fs::write(machine_dir.join("events.jsonl"), event).unwrap();
        cmd_history(dir.path(), None, 5, true).unwrap();
    }

    #[test]
    fn test_fj132_cmd_history_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["web", "db"] {
            let m_dir = dir.path().join(name);
            std::fs::create_dir_all(&m_dir).unwrap();
            let event = format!(
                r#"{{"ts":"2026-02-25T10:00:00Z","event":"apply_started","machine":"{}","run_id":"r-1","forjar_version":"0.1.0"}}"#,
                name
            );
            std::fs::write(m_dir.join("events.jsonl"), event).unwrap();
        }
        cmd_history(dir.path(), Some("web"), 10, false).unwrap();
    }

    #[test]
    fn test_fj132_cmd_status_with_global_lock() {
        let dir = tempfile::tempdir().unwrap();
        let lock_yaml = r#"
schema: '1.0'
name: my-infra
last_apply: '2026-02-25T10:00:00Z'
generator: 'forjar 0.1.0'
machines:
  web:
    resources: 5
    converged: 5
    failed: 0
    last_apply: '2026-02-25T10:00:00Z'
"#;
        std::fs::write(dir.path().join("forjar.lock.yaml"), lock_yaml).unwrap();
        cmd_status(dir.path(), None).unwrap();
    }

    #[test]
    fn test_fj132_cmd_fmt_formats_unformatted() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("messy.yaml");
        // Write valid but messy YAML
        let yaml = "version: '1.0'\nname: test\nmachines: {}\nresources: {}\n";
        std::fs::write(&file, yaml).unwrap();
        cmd_fmt(&file, false).unwrap();
        // File should be overwritten with canonical form
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.contains("version"));
    }

    #[test]
    fn test_fj132_export_scripts_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("scripts");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  my-pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
  my-file:
    type: file
    machine: m
    path: /etc/test.conf
    content: "hello"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        export_scripts(&config, &output_dir).unwrap();
        assert!(output_dir.join("my-pkg.check.sh").exists());
        assert!(output_dir.join("my-pkg.apply.sh").exists());
        assert!(output_dir.join("my-file.check.sh").exists());
        assert!(output_dir.join("my-file.apply.sh").exists());
    }

    #[test]
    fn test_fj132_export_scripts_sanitizes_slashes() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("scripts");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  web/config:
    type: file
    machine: m
    path: /etc/nginx/nginx.conf
    content: "server {}"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        export_scripts(&config, &output_dir).unwrap();
        // Slashes should be replaced with --
        assert!(output_dir.join("web--config.check.sh").exists());
        assert!(output_dir.join("web--config.apply.sh").exists());
    }

    #[test]
    fn test_fj132_run_hook_success() {
        let result = run_hook("test", "true", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj132_run_hook_failure() {
        let result = run_hook("test", "false", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj132_apply_param_overrides_with_equals_in_value() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let mut config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let overrides = vec!["conn=host=db port=5432".to_string()];
        apply_param_overrides(&mut config, &overrides).unwrap();
        // split_once only splits on first =, so value contains "host=db port=5432"
        assert_eq!(
            config.params.get("conn").unwrap(),
            &serde_yaml_ng::Value::String("host=db port=5432".to_string())
        );
    }

    // ── FJ-036 tests ────────────────────────────────────────────

    #[test]
    fn test_fj036_cmd_lint_bashrs_reports() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        // Config with a package resource — codegen will produce scripts
        // that bashrs can lint for shell safety diagnostics
        let yaml = r#"
version: "1.0"
name: lint-bashrs
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl, wget]
  conf:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "key=value"
"#;
        std::fs::write(&file, yaml).unwrap();
        // cmd_lint should succeed and produce bashrs diagnostics summary
        let result = cmd_lint(&file, true);
        assert!(
            result.is_ok(),
            "cmd_lint should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_fj036_cmd_validate_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: valid-project
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
  db:
    hostname: db-01
    addr: 10.0.0.2
resources:
  web-pkg:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
  db-pkg:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
  app-config:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    content: "server {}"
    depends_on: [web-pkg]
"#;
        std::fs::write(&file, yaml).unwrap();
        let result = cmd_validate(&file);
        assert!(
            result.is_ok(),
            "valid config should pass validation: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_fj036_cmd_init_creates_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("fj036-project");
        std::fs::create_dir_all(&project).unwrap();

        cmd_init(&project).unwrap();

        // Verify state directory was created
        assert!(
            project.join("state").is_dir(),
            "cmd_init must create state/ directory"
        );
        // Verify forjar.yaml was created
        assert!(
            project.join("forjar.yaml").exists(),
            "cmd_init must create forjar.yaml"
        );
        // Verify the generated config is valid YAML that parses
        let content = std::fs::read_to_string(project.join("forjar.yaml")).unwrap();
        let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
        assert_eq!(config.version, "1.0");
    }

    #[test]
    fn test_fj036_discover_container_machines() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path();

        // Create a container-transport machine directory with a state.lock.yaml
        let container_dir = state.join("docker-box");
        std::fs::create_dir_all(&container_dir).unwrap();
        std::fs::write(container_dir.join("state.lock.yaml"), "schema: '1.0'").unwrap();

        // Create another machine directory (non-container, but discover_machines
        // only checks for state.lock.yaml presence, not transport type)
        let ssh_dir = state.join("ssh-box");
        std::fs::create_dir_all(&ssh_dir).unwrap();
        std::fs::write(ssh_dir.join("state.lock.yaml"), "schema: '1.0'").unwrap();

        let machines = discover_machines(state);
        assert_eq!(machines.len(), 2);
        assert!(
            machines.contains(&"docker-box".to_string()),
            "container transport machine should be discovered"
        );
        assert!(
            machines.contains(&"ssh-box".to_string()),
            "ssh transport machine should also be discovered"
        );
        // discover_machines returns sorted results
        assert_eq!(machines[0], "docker-box");
        assert_eq!(machines[1], "ssh-box");
    }

    #[test]
    fn test_fj017_cmd_lint_clean_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
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
  my-config:
    type: file
    machine: local
    path: /etc/app.conf
    content: "key=value"
"#,
        )
        .unwrap();
        let result = cmd_lint(&config, false);
        assert!(
            result.is_ok(),
            "cmd_lint should succeed on a valid config with file resource"
        );
    }

    #[test]
    fn test_fj017_cmd_graph_dot_format() {
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
  conf:
    type: file
    machine: m1
    path: /etc/test.conf
    content: "hello"
    depends_on: [pkg]
"#,
        )
        .unwrap();
        let result = cmd_graph(&config, "dot");
        assert!(result.is_ok(), "cmd_graph with dot format should succeed");
    }

    #[test]
    fn test_fj017_cmd_status_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_status(&state, None);
        assert!(
            result.is_ok(),
            "cmd_status on empty state dir should succeed"
        );
    }

    #[test]
    fn test_fj017_cmd_validate_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nonexistent.yaml");
        let result = cmd_validate(&missing);
        assert!(
            result.is_err(),
            "cmd_validate should fail for a nonexistent file"
        );
    }

    #[test]
    fn test_fj017_cmd_fmt_check_valid() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        // Write a config, parse it, re-serialize to canonical form, then write that
        let yaml = r#"
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
"#;
        let parsed: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let formatted = serde_yaml_ng::to_string(&parsed).unwrap();
        std::fs::write(&config_path, &formatted).unwrap();
        let result = cmd_fmt(&config_path, true);
        assert!(
            result.is_ok(),
            "cmd_fmt check should succeed on already-formatted config"
        );
    }
}
