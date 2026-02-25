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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{MachineTarget, ResourceType};

    fn make_docker_resource(name: &str, image: &str) -> Resource {
        Resource {
            resource_type: ResourceType::Docker,
            machine: MachineTarget::Single("m1".to_string()),
            state: Some("running".to_string()),
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: Some(name.to_string()),
            enabled: None,
            restart_on: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: Some(image.to_string()),
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: std::collections::HashMap::new(),
            arch: vec![],
            tags: vec![],
        }
    }

    #[test]
    fn test_fj030_check_container() {
        let r = make_docker_resource("web", "nginx:latest");
        let script = check_script(&r);
        assert!(script.contains("docker inspect"));
        assert!(script.contains("'web'"));
        assert!(script.contains("exists:web"));
        assert!(script.contains("missing:web"));
    }

    #[test]
    fn test_fj030_apply_running() {
        let r = make_docker_resource("web", "nginx:latest");
        let script = apply_script(&r);
        assert!(script.contains("set -euo pipefail"));
        assert!(script.contains("docker pull 'nginx:latest'"));
        assert!(script.contains("docker run -d"));
        assert!(script.contains("--name 'web'"));
        assert!(script.contains("'nginx:latest'"));
    }

    #[test]
    fn test_fj030_apply_with_ports() {
        let mut r = make_docker_resource("web", "nginx:latest");
        r.ports = vec!["8080:80".to_string(), "443:443".to_string()];
        let script = apply_script(&r);
        assert!(script.contains("-p '8080:80'"));
        assert!(script.contains("-p '443:443'"));
    }

    #[test]
    fn test_fj030_apply_with_env() {
        let mut r = make_docker_resource("app", "myapp:v1");
        r.environment = vec!["DB_HOST=localhost".to_string()];
        let script = apply_script(&r);
        assert!(script.contains("-e 'DB_HOST=localhost'"));
    }

    #[test]
    fn test_fj030_apply_with_volumes() {
        let mut r = make_docker_resource("db", "postgres:15");
        r.volumes = vec!["/data/pg:/var/lib/postgresql/data".to_string()];
        let script = apply_script(&r);
        assert!(script.contains("-v '/data/pg:/var/lib/postgresql/data'"));
    }

    #[test]
    fn test_fj030_apply_with_restart() {
        let mut r = make_docker_resource("web", "nginx:latest");
        r.restart = Some("unless-stopped".to_string());
        let script = apply_script(&r);
        assert!(script.contains("--restart 'unless-stopped'"));
    }

    #[test]
    fn test_fj030_apply_absent() {
        let mut r = make_docker_resource("old", "nginx:latest");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("docker stop 'old'"));
        assert!(script.contains("docker rm 'old'"));
    }

    #[test]
    fn test_fj030_apply_stopped() {
        let mut r = make_docker_resource("app", "myapp:v1");
        r.state = Some("stopped".to_string());
        let script = apply_script(&r);
        assert!(script.contains("docker stop 'app'"));
        assert!(!script.contains("docker run"));
    }

    #[test]
    fn test_fj030_state_query() {
        let r = make_docker_resource("web", "nginx:latest");
        let script = state_query_script(&r);
        assert!(script.contains("docker inspect 'web'"));
        assert!(script.contains("container=MISSING:web"));
    }

    /// Verify single-quoting prevents injection.
    #[test]
    fn test_fj030_quoted_names() {
        let r = make_docker_resource("web; rm -rf /", "nginx:latest");
        let script = apply_script(&r);
        assert!(script.contains("'web; rm -rf /'"));
    }

    #[test]
    fn test_fj030_apply_with_command() {
        let mut r = make_docker_resource("worker", "myapp:v1");
        r.command = Some("./worker --queue=default".to_string());
        let script = apply_script(&r);
        assert!(script.contains("./worker --queue=default"));
    }

    #[test]
    fn test_fj030_apply_running_stops_existing() {
        // Running state should stop+rm existing before creating new
        let r = make_docker_resource("web", "nginx:latest");
        let script = apply_script(&r);
        let stop_idx = script.find("docker stop 'web'").unwrap();
        let rm_idx = script.find("docker rm 'web'").unwrap();
        let run_idx = script.find("docker run -d").unwrap();
        assert!(stop_idx < run_idx, "stop must come before run");
        assert!(rm_idx < run_idx, "rm must come before run");
    }

    #[test]
    fn test_fj030_absent_tolerant() {
        // absent uses || true to tolerate already-absent containers
        let mut r = make_docker_resource("gone", "nginx:latest");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(script.contains("|| true"));
    }

    #[test]
    fn test_fj030_stopped_tolerant() {
        let mut r = make_docker_resource("app", "myapp:v1");
        r.state = Some("stopped".to_string());
        let script = apply_script(&r);
        assert!(script.contains("|| true"));
    }

    #[test]
    fn test_fj030_default_state_is_running() {
        let mut r = make_docker_resource("app", "myapp:v1");
        r.state = None;
        let script = apply_script(&r);
        assert!(
            script.contains("docker run -d"),
            "default state should be running"
        );
    }

    #[test]
    fn test_fj030_apply_all_options() {
        let mut r = make_docker_resource("full", "myapp:v1");
        r.ports = vec!["8080:80".to_string()];
        r.environment = vec!["KEY=val".to_string()];
        r.volumes = vec!["/data:/app/data".to_string()];
        r.restart = Some("always".to_string());
        r.command = Some("./start".to_string());
        let script = apply_script(&r);
        assert!(script.contains("-p '8080:80'"));
        assert!(script.contains("-e 'KEY=val'"));
        assert!(script.contains("-v '/data:/app/data'"));
        assert!(script.contains("--restart 'always'"));
        assert!(script.contains("./start"));
    }

    // ── Edge-case tests (FJ-124) ─────────────────────────────────

    #[test]
    fn test_fj030_no_name_defaults_to_unknown() {
        let mut r = make_docker_resource("placeholder", "nginx:latest");
        r.name = None;
        let check = check_script(&r);
        assert!(check.contains("'unknown'"));
        let apply = apply_script(&r);
        assert!(apply.contains("--name 'unknown'"));
        let query = state_query_script(&r);
        assert!(query.contains("docker inspect 'unknown'"));
    }

    #[test]
    fn test_fj030_no_image_defaults_to_unknown() {
        let mut r = make_docker_resource("web", "placeholder");
        r.image = None;
        let script = apply_script(&r);
        assert!(script.contains("docker pull 'unknown'"));
        assert!(script.contains("'unknown'")); // image arg in run
    }

    #[test]
    fn test_fj030_multiple_ports_env_volumes() {
        let mut r = make_docker_resource("app", "myapp:v1");
        r.ports = vec![
            "80:80".to_string(),
            "443:443".to_string(),
            "8080:8080".to_string(),
        ];
        r.environment = vec!["A=1".to_string(), "B=2".to_string()];
        r.volumes = vec!["/a:/a".to_string(), "/b:/b".to_string()];
        let script = apply_script(&r);
        assert_eq!(script.matches("-p '").count(), 3);
        assert_eq!(script.matches("-e '").count(), 2);
        assert_eq!(script.matches("-v '").count(), 2);
    }

    #[test]
    fn test_fj030_absent_no_run_no_pull() {
        // absent should only stop+rm, never pull or run
        let mut r = make_docker_resource("old", "nginx:latest");
        r.state = Some("absent".to_string());
        let script = apply_script(&r);
        assert!(!script.contains("docker pull"));
        assert!(!script.contains("docker run"));
    }

    // --- FJ-132: Docker edge case tests ---

    #[test]
    fn test_fj132_apply_empty_ports_env_volumes() {
        // Empty lists should not produce spurious flags
        let r = make_docker_resource("web", "nginx:latest");
        let script = apply_script(&r);
        assert!(!script.contains("-p '"), "empty ports should not add -p flags");
        assert!(!script.contains("-e '"), "empty env should not add -e flags");
        assert!(!script.contains("-v '"), "empty volumes should not add -v flags");
    }

    #[test]
    fn test_fj132_apply_no_restart_no_flag() {
        // No restart policy should not add --restart flag
        let mut r = make_docker_resource("web", "nginx:latest");
        r.restart = None;
        let script = apply_script(&r);
        assert!(!script.contains("--restart"), "no restart policy = no --restart flag");
    }

    #[test]
    fn test_fj132_state_query_contains_inspect() {
        let r = make_docker_resource("web", "nginx:latest");
        let script = state_query_script(&r);
        assert!(script.contains("docker inspect"), "state_query should use docker inspect");
        assert!(script.contains("'web'"), "state_query should reference container name");
    }

    #[test]
    fn test_fj132_check_script_format() {
        let r = make_docker_resource("web", "nginx:latest");
        let script = check_script(&r);
        assert!(script.contains("docker inspect"), "check should inspect container");
        assert!(script.contains("'web'"), "check should reference name");
    }

    #[test]
    fn test_fj132_apply_scripts_idempotent() {
        let r = make_docker_resource("web", "nginx:latest");
        let s1 = apply_script(&r);
        let s2 = apply_script(&r);
        assert_eq!(s1, s2, "apply_script must be idempotent");
    }
}
