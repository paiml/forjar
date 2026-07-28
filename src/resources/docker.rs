//! FJ-030: Docker container resource handler.
//!
//! Manages Docker containers as resources: pull, run, stop, remove.
//! This is distinct from container *transport* (FJ-021) — this manages
//! containers deployed ON machines, not containers used AS machines.

use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::verdict;

/// Generate shell script to check if a container is running.
pub fn check_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let n = sh_squote(name);
    verdict::single(
        &format!("docker inspect -f '{{{{.State.Running}}}}' {n} >/dev/null 2>&1"),
        &format!("exists:{name}"),
        &format!("missing:{name}"),
    )
}

/// Generate shell script to manage a container.
pub fn apply_script(resource: &Resource) -> String {
    let name = resource.name.as_deref().unwrap_or("unknown");
    let state = resource.state.as_deref().unwrap_or("running");
    let image = resource.image.as_deref().unwrap_or("unknown");
    let n = sh_squote(name);

    match state {
        "absent" => format!(
            "set -euo pipefail\n\
             docker stop {n} 2>/dev/null || true\n\
             docker rm {n} 2>/dev/null || true"
        ),
        "stopped" => format!(
            "set -euo pipefail\n\
             docker stop {n} 2>/dev/null || true"
        ),
        _ => {
            // "running" or "present"
            let mut lines = vec![
                "set -euo pipefail".to_string(),
                format!("docker pull {}", sh_squote(image)),
            ];

            // Stop and remove existing container if it exists
            lines.push(format!("docker stop {n} 2>/dev/null || true"));
            lines.push(format!("docker rm {n} 2>/dev/null || true"));

            // Build run command
            let mut run_args = vec!["docker run -d".to_string()];
            run_args.push(format!("--name {n}"));

            if let Some(ref restart) = resource.restart {
                run_args.push(format!("--restart {}", sh_squote(restart)));
            }

            for port in &resource.ports {
                run_args.push(format!("-p {}", sh_squote(port)));
            }

            for env in &resource.environment {
                run_args.push(format!("-e {}", sh_squote(env)));
            }

            for vol in &resource.volumes {
                run_args.push(format!("-v {}", sh_squote(vol)));
            }

            run_args.push(sh_squote(image));

            // Append command if specified (intentionally arbitrary shell).
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
    let n = sh_squote(name);
    format!(
        "docker inspect {n} 2>/dev/null && echo {} || echo {}",
        sh_squote(&format!("container={name}")),
        sh_squote(&format!("container=MISSING:{name}"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn docker_resource(name: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Docker,
            machine: MachineTarget::Single("m1".to_string()),
            name: Some(name.to_string()),
            image: Some("nginx:latest".to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn fj154_docker_name_quote_neutralized() {
        let r = docker_resource("c';reboot;'");
        let script = apply_script(&r);
        assert!(script.contains("'\\''"), "{script}");
        assert!(!script.contains("docker stop 'c';reboot"), "{script}");
    }

    #[test]
    fn fj154_docker_image_and_run_fields_quoted() {
        let mut r = docker_resource("web");
        r.image = Some("img';id;'".to_string());
        r.ports = vec!["8080:80".to_string()];
        r.environment = vec!["KEY=v';id;'".to_string()];
        r.volumes = vec!["/data:/data".to_string()];
        let script = apply_script(&r);
        assert!(script.contains("'\\''"), "{script}");
        assert!(script.contains("-p '8080:80'"), "{script}");
        assert!(script.contains("-v '/data:/data'"), "{script}");
    }

    #[test]
    fn fj154_docker_benign_unchanged() {
        let r = docker_resource("web");
        let script = apply_script(&r);
        assert!(script.contains("docker pull 'nginx:latest'"));
        assert!(script.contains("--name 'web'"));
        assert!(check_script(&r).contains("exists:web"));
        assert!(state_query_script(&r).contains("container=MISSING:web"));
    }
}
