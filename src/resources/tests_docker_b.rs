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
        gpu_backend: None,
        driver_version: None,
        cuda_version: None,
        rocm_version: None,
        devices: vec![],
        persistence_mode: None,
        compute_mode: None,
        gpu_memory_limit_mb: None,
        output_artifacts: vec![],
        completion_check: None,
        timeout: None,
        working_dir: None,
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
        script: None,
    }
}

// --- FJ-132: Docker edge case tests ---

#[test]
fn test_fj132_apply_empty_ports_env_volumes() {
    let r = make_docker_resource("web", "nginx:latest");
    let script = apply_script(&r);
    assert!(
        !script.contains("-p '"),
        "empty ports should not add -p flags"
    );
    assert!(
        !script.contains("-e '"),
        "empty env should not add -e flags"
    );
    assert!(
        !script.contains("-v '"),
        "empty volumes should not add -v flags"
    );
}

#[test]
fn test_fj132_apply_no_restart_no_flag() {
    let mut r = make_docker_resource("web", "nginx:latest");
    r.restart = None;
    let script = apply_script(&r);
    assert!(
        !script.contains("--restart"),
        "no restart policy = no --restart flag"
    );
}

#[test]
fn test_fj132_state_query_contains_inspect() {
    let r = make_docker_resource("web", "nginx:latest");
    let script = state_query_script(&r);
    assert!(
        script.contains("docker inspect"),
        "state_query should use docker inspect"
    );
    assert!(
        script.contains("'web'"),
        "state_query should reference container name"
    );
}

#[test]
fn test_fj132_check_script_format() {
    let r = make_docker_resource("web", "nginx:latest");
    let script = check_script(&r);
    assert!(
        script.contains("docker inspect"),
        "check should inspect container"
    );
    assert!(script.contains("'web'"), "check should reference name");
}

#[test]
fn test_fj132_apply_scripts_idempotent() {
    let r = make_docker_resource("web", "nginx:latest");
    let s1 = apply_script(&r);
    let s2 = apply_script(&r);
    assert_eq!(s1, s2, "apply_script must be idempotent");
}

// -- FJ-036: Additional docker resource tests --

#[test]
fn test_fj036_docker_apply_absent_removes() {
    let mut r = make_docker_resource("stale-app", "myapp:old");
    r.state = Some("absent".to_string());
    let script = apply_script(&r);
    assert!(
        script.contains("docker rm 'stale-app'"),
        "absent must generate docker rm"
    );
    assert!(
        script.contains("docker stop 'stale-app'"),
        "absent must generate docker stop before rm"
    );
    assert!(
        !script.contains("docker pull"),
        "absent must not pull image"
    );
    assert!(
        !script.contains("docker run"),
        "absent must not run container"
    );
}

#[test]
fn test_fj036_docker_check_running_container() {
    let r = make_docker_resource("api-server", "api:v2");
    let script = check_script(&r);
    assert!(
        script.contains("docker inspect"),
        "check should use docker inspect"
    );
    assert!(
        script.contains("'api-server'"),
        "check should reference container name"
    );
    assert!(
        script.contains("State.Running"),
        "check should query running state"
    );
    assert!(
        script.contains("exists:api-server"),
        "check should emit exists token"
    );
    assert!(
        script.contains("missing:api-server"),
        "check should emit missing token"
    );
}

#[test]
fn test_fj036_docker_apply_with_ports_and_volumes() {
    let mut r = make_docker_resource("webapp", "myapp:latest");
    r.ports = vec!["8080:80".to_string(), "8443:443".to_string()];
    r.volumes = vec![
        "/host/data:/container/data".to_string(),
        "/host/logs:/var/log/app".to_string(),
    ];
    let script = apply_script(&r);
    assert!(
        script.contains("-p '8080:80'"),
        "first port mapping missing"
    );
    assert!(
        script.contains("-p '8443:443'"),
        "second port mapping missing"
    );
    assert!(
        script.contains("-v '/host/data:/container/data'"),
        "first volume mapping missing"
    );
    assert!(
        script.contains("-v '/host/logs:/var/log/app'"),
        "second volume mapping missing"
    );
    assert!(
        script.contains("docker run -d"),
        "must run in detached mode"
    );
    assert!(
        script.contains("--name 'webapp'"),
        "must name the container"
    );
}

// -- Coverage boost tests --

#[test]
fn test_docker_check_running() {
    let r = make_docker_resource("redis-cache", "redis:7-alpine");
    let script = check_script(&r);
    assert!(
        script.contains("docker inspect -f"),
        "check must use docker inspect: {script}"
    );
    assert!(
        script.contains("State.Running"),
        "check must query running state: {script}"
    );
    assert!(
        script.contains("'redis-cache'"),
        "check must reference container name: {script}"
    );
    assert!(
        script.contains("exists:redis-cache"),
        "check must emit exists token: {script}"
    );
    assert!(
        script.contains("missing:redis-cache"),
        "check must emit missing token: {script}"
    );
}

#[test]
fn test_docker_state_query_with_network() {
    let mut r = make_docker_resource("api", "api-server:v3");
    r.restart = Some("on-failure:5".to_string());
    let query = state_query_script(&r);
    assert!(
        query.contains("docker inspect 'api'"),
        "state_query must inspect container: {query}"
    );
    assert!(
        query.contains("container=api"),
        "state_query must emit container token: {query}"
    );
    assert!(
        query.contains("container=MISSING:api"),
        "state_query must emit missing token: {query}"
    );

    let apply = apply_script(&r);
    assert!(
        apply.contains("--restart 'on-failure:5'"),
        "apply must include restart policy: {apply}"
    );
}

#[test]
fn test_fj153_stopped_ignores_ports_env_volumes() {
    let mut r = make_docker_resource("web", "nginx:latest");
    r.state = Some("stopped".to_string());
    r.ports = vec!["8080:80".to_string()];
    r.environment = vec!["KEY=val".to_string()];
    r.volumes = vec!["/data:/data".to_string()];
    r.restart = Some("always".to_string());
    r.command = Some("./start".to_string());
    let script = apply_script(&r);
    assert!(script.contains("docker stop"), "stopped must stop");
    assert!(!script.contains("docker run"), "stopped must not run");
    assert!(!script.contains("-p '"), "stopped must not map ports");
    assert!(!script.contains("-e '"), "stopped must not set env");
    assert!(!script.contains("-v '"), "stopped must not mount volumes");
    assert!(
        !script.contains("--restart"),
        "stopped must not set restart"
    );
}

#[test]
fn test_fj153_absent_ignores_ports_env_volumes() {
    let mut r = make_docker_resource("old", "nginx:latest");
    r.state = Some("absent".to_string());
    r.ports = vec!["8080:80".to_string()];
    r.environment = vec!["KEY=val".to_string()];
    r.volumes = vec!["/data:/data".to_string()];
    r.restart = Some("always".to_string());
    r.command = Some("./start".to_string());
    let script = apply_script(&r);
    assert!(script.contains("docker stop 'old'"));
    assert!(script.contains("docker rm 'old'"));
    assert!(!script.contains("docker pull"));
    assert!(!script.contains("docker run"));
    assert!(!script.contains("-p '"));
    assert!(!script.contains("-e '"));
    assert!(!script.contains("-v '"));
}

#[test]
fn test_fj153_explicit_present_state() {
    let mut r = make_docker_resource("web", "nginx:latest");
    r.state = Some("present".to_string());
    let script = apply_script(&r);
    assert!(script.contains("docker pull"));
    assert!(script.contains("docker run"));
}

#[test]
fn test_fj153_env_with_special_chars() {
    let mut r = make_docker_resource("app", "myapp:v1");
    r.environment = vec![
        "DB_URL=postgres://user:pass@host:5432/db".to_string(),
        "JSON={\"key\":\"value\"}".to_string(),
    ];
    let script = apply_script(&r);
    assert!(script.contains("-e 'DB_URL=postgres://user:pass@host:5432/db'"));
    assert!(script.contains("-e 'JSON={\"key\":\"value\"}'"));
}

#[test]
fn test_fj153_large_port_list() {
    let mut r = make_docker_resource("web", "nginx:latest");
    r.ports = (8000..8006).map(|p| format!("{}:{}", p, p)).collect();
    let script = apply_script(&r);
    assert_eq!(script.matches("-p '").count(), 6);
}

#[test]
fn test_docker_apply_stop_then_run() {
    let mut r = make_docker_resource("app-server", "myapp:v2");
    r.state = Some("running".to_string());
    r.ports = vec!["3000:3000".to_string()];
    r.environment = vec!["NODE_ENV=production".to_string()];
    r.restart = Some("always".to_string());
    let script = apply_script(&r);

    let pull_idx = script.find("docker pull 'myapp:v2'").unwrap();
    let stop_idx = script.find("docker stop 'app-server'").unwrap();
    let rm_idx = script.find("docker rm 'app-server'").unwrap();
    let run_idx = script.find("docker run -d").unwrap();
    assert!(pull_idx < stop_idx, "pull must come before stop");
    assert!(stop_idx < rm_idx, "stop must come before rm");
    assert!(rm_idx < run_idx, "rm must come before run");

    assert!(
        script.contains("--name 'app-server'"),
        "run must name the container: {script}"
    );
    assert!(
        script.contains("--restart 'always'"),
        "run must set restart policy: {script}"
    );
    assert!(
        script.contains("-p '3000:3000'"),
        "run must map port: {script}"
    );
    assert!(
        script.contains("-e 'NODE_ENV=production'"),
        "run must set env: {script}"
    );
    assert!(
        script.contains("'myapp:v2'"),
        "run must reference image: {script}"
    );
}
