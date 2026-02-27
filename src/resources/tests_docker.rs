use super::docker::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};

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
        triggers: vec![],
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
        resource_group: None,
        when: None,
        count: None,
        for_each: None,
        chroot_dir: None,
        namespace_uid: None,
        namespace_gid: None,
        seccomp: false,
        netns: false,
        cpuset: None,
        memory_limit: None,
        overlay_lower: None,
        overlay_upper: None,
        overlay_work: None,
        overlay_merged: None,
        format: None,
        quantization: None,
        checksum: None,
        cache_dir: None,
        driver_version: None,
        cuda_version: None,
        devices: vec![],
        persistence_mode: None,
        compute_mode: None,
        gpu_memory_limit_mb: None,
        pre_apply: None,
        post_apply: None,
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
