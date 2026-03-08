//! FJ-52/54: Image generation — autoinstall user-data and Android Magisk modules.
//!
//! Demonstrates generating Ubuntu autoinstall YAML from machine config,
//! and Android init.rc fragments for Magisk module packaging.
//!
//! Usage: cargo run --example image_generation

use forjar::core::types::Machine;

fn main() {
    println!("=== FJ-52: Autoinstall User-Data Generation ===\n");

    let machine = Machine {
        hostname: "edge-node-01".to_string(),
        addr: "10.0.1.50".to_string(),
        user: "deploy".to_string(),
        arch: "amd64".to_string(),
        ssh_key: None, // No SSH key for demo (avoids file read)
        roles: vec!["compute".to_string(), "storage".to_string()],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };

    // Generate user-data using the public internal helper
    let user_data = forjar::cli::image_cmd::generate_user_data(
        "edge-node-01",
        &machine,
        "auto-lvm",
        "en_US.UTF-8",
        "America/New_York",
    )
    .unwrap();

    println!("--- Generated user-data (auto-lvm) ---");
    println!("{user_data}");

    // ZFS storage layout variant
    println!("--- Generated user-data (auto-zfs) ---");
    let zfs_data = forjar::cli::image_cmd::generate_user_data(
        "edge-node-01",
        &machine,
        "auto-zfs",
        "en_US.UTF-8",
        "UTC",
    )
    .unwrap();
    for line in zfs_data.lines().filter(|l| l.contains("name: zfs")) {
        println!("  {line}");
    }

    // Explicit disk path
    println!("\n--- Storage: explicit /dev/nvme0n1 ---");
    let disk_data = forjar::cli::image_cmd::generate_user_data(
        "edge-node-01",
        &machine,
        "/dev/nvme0n1",
        "en_US.UTF-8",
        "UTC",
    )
    .unwrap();
    for line in disk_data.lines().filter(|l| l.contains("nvme")) {
        println!("  {line}");
    }

    // Firstboot service
    println!("\n--- Firstboot systemd service ---");
    let firstboot = forjar::cli::image_cmd::firstboot_service_command();
    println!("{firstboot}");

    // FJ-54: Android init.rc
    println!("=== FJ-54: Android init.rc Fragment ===\n");
    let init_rc = forjar::cli::image_android::generate_init_rc("pixel-7");
    println!("{init_rc}");

    // Invalid disk layout error
    println!("=== Error handling: invalid disk layout ===\n");
    let err = forjar::cli::image_cmd::generate_user_data(
        "test",
        &machine,
        "btrfs-raid",
        "en_US.UTF-8",
        "UTC",
    );
    println!("Expected error: {}", err.unwrap_err());

    println!("\n=== Machine resolution ===\n");
    println!("Single machine in config: auto-selected (no --machine flag needed)");
    println!("Multiple machines: --machine flag required (error lists available names)");
}
