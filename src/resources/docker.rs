//! FJ-030: Docker container resource handler.
//!
//! Manages Docker containers as resources: pull, run, stop, remove.
//! This is distinct from container *transport* (FJ-021) — this manages
//! containers deployed ON machines, not containers used AS machines.

use crate::core::types::Resource;

/// Generate shell script to check if a container is running.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    format!(
        "docker inspect -f '{{{{.State.Running}}}}' '{}' 2>/dev/null && echo 'exists:{}' || echo 'missing:{}'",
        name, name, name
    )
}

/// Generate shell script to manage a container.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let state = resource.state.as_deref().unwrap_or("running");
    let image = resource.image.as_deref().unwrap_or("unknown");

    match state {
        "absent" => format!(
            "set -euo pipefail\n\
             docker stop '{}' 2>/dev/null || true\n\
             docker rm '{}' 2>/dev/null || true",
            name, name
        ),
        "stopped" => format!(
            "set -euo pipefail\n\
             docker stop '{}' 2>/dev/null || true",
            name
        ),
        _ => {
            // "running" or "present"
            let mut lines = vec![
                "set -euo pipefail".to_string(),
                format!("docker pull '{}'", image),
            ];

            // Stop and remove existing container if it exists
            lines.push(format!("docker stop '{}' 2>/dev/null || true", name));
            lines.push(format!("docker rm '{}' 2>/dev/null || true", name));

            // Build run command
            let mut run_args = vec!["docker run -d".to_string()];
            run_args.push(format!("--name '{}'", name));

            if let Some(ref restart) = resource.restart {
                run_args.push(format!("--restart '{}'", restart));
            }

            for port in &resource.ports {
                run_args.push(format!("-p '{}'", port));
            }

            for env in &resource.environment {
                run_args.push(format!("-e '{}'", env));
            }

            for vol in &resource.volumes {
                run_args.push(format!("-v '{}'", vol));
            }

            run_args.push(format!("'{}'", image));

            // Append command if specified
            if let Some(ref cmd) = resource.command {
                run_args.push(cmd.clone());
            }

            lines.push(run_args.join(" \\\n  "));

            lines.join("\n")
        }
    }
}

/// Generate shell to query container state (for BLAKE3 hashing).
pub fn state_query_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    format!(
        "docker inspect '{}' 2>/dev/null && echo 'container={}' || echo 'container=MISSING:{}'",
        name, name, name
    )
}
