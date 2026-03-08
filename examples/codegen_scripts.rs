//! Generate shell scripts for different resource types.
//!
//! Usage: cargo run --example codegen_scripts

use forjar::core::codegen;
use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::collections::HashMap;

fn base(rt: ResourceType) -> Resource {
    Resource {
        resource_type: rt,
        machine: MachineTarget::Single("local".to_string()),
        state: None,
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
        name: None,
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
        image: None,
        ports: vec![],
        environment: vec![],
        volumes: vec![],
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
        recipe: None,
        inputs: HashMap::new(),
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
        task_mode: None,
        task_inputs: vec![],
        stages: vec![],
        cache: false,
        gpu_device: None,
        restart_delay: None,
        quality_gate: None,
        health_check: None,
        restart_policy: None,
        pre_apply: None,
        post_apply: None,
        lifecycle: None,
        store: false,
        sudo: false,
        script: None,
        gather: vec![],
        scatter: vec![],
        build_machine: None,
    }
}

fn main() {
    println!("=== Package Resource (apt) ===\n");
    let pkg = Resource {
        provider: Some("apt".to_string()),
        packages: vec!["curl".to_string(), "htop".to_string()],
        ..base(ResourceType::Package)
    };
    println!("-- Check script --");
    println!("{}", codegen::check_script(&pkg).unwrap());
    println!("\n-- Apply script --");
    println!("{}", codegen::apply_script(&pkg).unwrap());
    println!("\n-- State query script --");
    println!("{}", codegen::state_query_script(&pkg).unwrap());

    println!("\n=== File Resource ===\n");
    let file = Resource {
        path: Some("/etc/app/config.yaml".to_string()),
        content: Some("database:\n  host: localhost\n  port: 5432\n".to_string()),
        owner: Some("app".to_string()),
        group: Some("app".to_string()),
        mode: Some("0640".to_string()),
        ..base(ResourceType::File)
    };
    println!("-- Apply script --");
    println!("{}", codegen::apply_script(&file).unwrap());

    println!("\n=== File Resource (source transfer) ===\n");
    let source_file = Resource {
        path: Some("/opt/app/entrypoint.sh".to_string()),
        source: Some("examples/files/app-entrypoint.sh".to_string()),
        owner: Some("app".to_string()),
        mode: Some("0755".to_string()),
        ..base(ResourceType::File)
    };
    println!("-- Apply script (base64 transfer) --");
    println!("{}", codegen::apply_script(&source_file).unwrap());

    println!("\n=== Service Resource ===\n");
    let svc = Resource {
        state: Some("running".to_string()),
        name: Some("nginx".to_string()),
        enabled: Some(true),
        ..base(ResourceType::Service)
    };
    println!("-- Apply script --");
    println!("{}", codegen::apply_script(&svc).unwrap());

    println!("\n=== Mount Resource (NFS) ===\n");
    let mnt = Resource {
        path: Some("/mnt/data".to_string()),
        source: Some("192.168.1.10:/exports/data".to_string()),
        fs_type: Some("nfs".to_string()),
        options: Some("ro,hard,intr".to_string()),
        ..base(ResourceType::Mount)
    };
    println!("-- Apply script --");
    println!("{}", codegen::apply_script(&mnt).unwrap());

    println!("\n=== User Resource ===\n");
    let usr = Resource {
        name: Some("deploy".to_string()),
        shell: Some("/bin/bash".to_string()),
        home: Some("/home/deploy".to_string()),
        groups: vec!["docker".to_string(), "sudo".to_string()],
        ..base(ResourceType::User)
    };
    println!("-- Check script --");
    println!("{}", codegen::check_script(&usr).unwrap());
    println!("\n-- Apply script --");
    println!("{}", codegen::apply_script(&usr).unwrap());

    println!("\n=== Docker Resource ===\n");
    let docker = Resource {
        state: Some("running".to_string()),
        name: Some("web".to_string()),
        image: Some("nginx:latest".to_string()),
        ports: vec!["8080:80".to_string()],
        environment: vec!["ENV=production".to_string()],
        restart: Some("unless-stopped".to_string()),
        ..base(ResourceType::Docker)
    };
    println!("-- Apply script --");
    println!("{}", codegen::apply_script(&docker).unwrap());

    println!("\n=== Cron Resource ===\n");
    let cron = Resource {
        owner: Some("root".to_string()),
        name: Some("db-backup".to_string()),
        schedule: Some("0 2 * * *".to_string()),
        command: Some("/opt/scripts/backup-db.sh".to_string()),
        ..base(ResourceType::Cron)
    };
    println!("-- Apply script --");
    println!("{}", codegen::apply_script(&cron).unwrap());

    println!("\n=== Network Resource (ufw) ===\n");
    let fw = Resource {
        name: Some("ssh-access".to_string()),
        protocol: Some("tcp".to_string()),
        port: Some("22".to_string()),
        action: Some("allow".to_string()),
        from_addr: Some("10.0.0.0/8".to_string()),
        ..base(ResourceType::Network)
    };
    println!("-- Apply script --");
    println!("{}", codegen::apply_script(&fw).unwrap());

    // GPU resources — multi-vendor (FJ-1005)
    for (backend, driver, cuda, rocm) in [
        ("nvidia", "550", Some("12.4"), None),
        ("rocm", "6.3", None, Some("6.3")),
        ("cpu", "", None, None),
    ] {
        println!("\n=== GPU Resource ({backend}) ===\n");
        let gpu = Resource {
            gpu_backend: Some(backend.to_string()),
            driver_version: Some(driver.to_string()),
            cuda_version: cuda.map(String::from),
            rocm_version: rocm.map(String::from),
            persistence_mode: Some(true),
            compute_mode: Some("default".to_string()),
            ..base(ResourceType::Gpu)
        };
        println!("-- Check script --");
        println!("{}", codegen::check_script(&gpu).unwrap());
        println!("\n-- Apply script --");
        println!("{}", codegen::apply_script(&gpu).unwrap());
        println!("\n-- State query script --");
        println!("{}", codegen::state_query_script(&gpu).unwrap());
    }

    // Cargo package with bootstrap (FJ-1005)
    println!("\n=== Package Resource (cargo + bootstrap) ===\n");
    let cargo_pkg = Resource {
        provider: Some("cargo".to_string()),
        packages: vec!["realizar".to_string()],
        ..base(ResourceType::Package)
    };
    println!("-- Apply script --");
    println!("{}", codegen::apply_script(&cargo_pkg).unwrap());
}
