//! FJ-1383: Config merge — combine two forjar config files.

use crate::core::parser;
use std::path::Path;

/// Merge two forjar config files. Detects resource/machine collisions.
pub(crate) fn cmd_config_merge(
    file_a: &Path,
    file_b: &Path,
    output: Option<&Path>,
    allow_collisions: bool,
) -> Result<(), String> {
    let mut config_a = parser::parse_and_validate(file_a)?;
    let config_b = parser::parse_and_validate(file_b)?;

    // Detect machine collisions
    let machine_collisions: Vec<_> = config_b
        .machines
        .keys()
        .filter(|k| config_a.machines.contains_key(*k))
        .cloned()
        .collect();
    if !machine_collisions.is_empty() && !allow_collisions {
        return Err(format!(
            "machine name collision(s): {} — use --allow-collisions to override",
            machine_collisions.join(", ")
        ));
    }

    // Detect resource collisions
    let resource_collisions: Vec<_> = config_b
        .resources
        .keys()
        .filter(|k| config_a.resources.contains_key(*k))
        .cloned()
        .collect();
    if !resource_collisions.is_empty() && !allow_collisions {
        return Err(format!(
            "resource ID collision(s): {} — use --allow-collisions to override",
            resource_collisions.join(", ")
        ));
    }

    // Merge (right takes precedence on collisions)
    for (k, v) in config_b.machines {
        config_a.machines.insert(k, v);
    }
    for (k, v) in config_b.resources {
        config_a.resources.insert(k, v);
    }
    for (k, v) in config_b.params {
        config_a.params.entry(k).or_insert(v);
    }
    for (k, v) in config_b.outputs {
        config_a.outputs.entry(k).or_insert(v);
    }

    // Update name
    config_a.name = format!("{} + {}", config_a.name, config_b.name);

    let yaml =
        serde_yaml_ng::to_string(&config_a).map_err(|e| format!("serialize merged config: {e}"))?;

    match output {
        Some(path) => {
            std::fs::write(path, &yaml).map_err(|e| format!("write {}: {e}", path.display()))?;
            eprintln!("Merged config written to {}", path.display());
        }
        None => print!("{yaml}"),
    }

    let total_machines = config_a.machines.len();
    let total_resources = config_a.resources.len();
    eprintln!(
        "Merged: {} machines, {} resources ({} machine collision(s), {} resource collision(s))",
        total_machines,
        total_resources,
        machine_collisions.len(),
        resource_collisions.len()
    );

    Ok(())
}
