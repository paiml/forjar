//! Generate shell scripts for different resource types.
//!
//! Usage: cargo run --example codegen_scripts

use forjar::core::codegen;
use forjar::core::types::{MachineTarget, Resource, ResourceType};

fn main() {
    println!("=== Package Resource (apt) ===\n");
    let pkg = Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("local".to_string()),
        state: None,
        depends_on: vec![],
        provider: Some("apt".to_string()),
        packages: vec!["curl".to_string(), "htop".to_string()],
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
    };

    println!("-- Check script --");
    println!("{}", codegen::check_script(&pkg).unwrap());
    println!("\n-- Apply script --");
    println!("{}", codegen::apply_script(&pkg).unwrap());
    println!("\n-- State query script --");
    println!("{}", codegen::state_query_script(&pkg).unwrap());

    println!("\n=== File Resource ===\n");
    let file = Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("local".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        path: Some("/etc/app/config.yaml".to_string()),
        content: Some("database:\n  host: localhost\n  port: 5432\n".to_string()),
        source: None,
        target: None,
        owner: Some("app".to_string()),
        group: Some("app".to_string()),
        mode: Some("0640".to_string()),
        name: None,
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
        image: None,
        ports: vec![],
        environment: vec![],
        volumes: vec![],
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
    };

    println!("-- Apply script --");
    println!("{}", codegen::apply_script(&file).unwrap());

    println!("\n=== Service Resource ===\n");
    let svc = Resource {
        resource_type: ResourceType::Service,
        machine: MachineTarget::Single("local".to_string()),
        state: Some("running".to_string()),
        depends_on: vec![],
        provider: None,
        packages: vec![],
        path: None,
        content: None,
        source: None,
        target: None,
        owner: None,
        group: None,
        mode: None,
        name: Some("nginx".to_string()),
        enabled: Some(true),
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
        image: None,
        ports: vec![],
        environment: vec![],
        volumes: vec![],
        restart: None,
        protocol: None,
        port: None,
        action: None,
        from_addr: None,
    };

    println!("-- Apply script --");
    println!("{}", codegen::apply_script(&svc).unwrap());

    println!("\n=== Mount Resource (NFS) ===\n");
    let mnt = Resource {
        resource_type: ResourceType::Mount,
        machine: MachineTarget::Single("local".to_string()),
        state: None,
        depends_on: vec![],
        provider: None,
        packages: vec![],
        path: Some("/mnt/data".to_string()),
        content: None,
        source: Some("192.168.1.10:/exports/data".to_string()),
        target: None,
        owner: None,
        group: None,
        mode: None,
        name: None,
        enabled: None,
        restart_on: vec![],
        fs_type: Some("nfs".to_string()),
        options: Some("ro,hard,intr".to_string()),
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
    };

    println!("-- Apply script --");
    println!("{}", codegen::apply_script(&mnt).unwrap());
}
