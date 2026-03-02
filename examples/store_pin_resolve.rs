//! Demonstrates forjar's pin resolution: version parsing, hash computation,
//! and resolution command generation for all providers.
//!
//! Run: `cargo run --example store_pin_resolve`

use forjar::core::store::pin_resolve::{parse_resolved_version, pin_hash, resolution_command};

fn main() {
    println!("=== Forjar Pin Resolution Demo ===\n");
    demo_resolution_commands();
    demo_version_parsing();
    demo_hash_determinism();
    println!("\n=== All pin resolution demos passed ===");
}

fn demo_resolution_commands() {
    println!("--- 1. Resolution Commands per Provider ---");

    let providers = [
        ("apt", "nginx"),
        ("cargo", "ripgrep"),
        ("nix", "nixpkgs#ripgrep"),
        ("uv", "requests"),
        ("docker", "alpine"),
        ("tofu", "aws_instance"),
        ("terraform", "aws_instance"),
    ];

    for (provider, name) in providers {
        match resolution_command(provider, name) {
            Some(cmd) => println!("  {provider:12} → {cmd}"),
            None => println!("  {provider:12} → (no resolution command)"),
        }
    }

    // All providers should have resolution commands
    assert!(resolution_command("apt", "curl").is_some());
    assert!(resolution_command("cargo", "serde").is_some());
    assert!(resolution_command("nix", "nixpkgs#hello").is_some());
    assert!(resolution_command("uv", "flask").is_some());
    assert!(resolution_command("docker", "ubuntu").is_some());

    // Unknown provider returns None
    assert!(resolution_command("unknown", "pkg").is_none());
    println!("  All providers generate valid resolution commands");
}

fn demo_version_parsing() {
    println!("\n--- 2. Version Parsing from Provider Output ---");

    let test_cases = [
        (
            "apt",
            "nginx:\n  Installed: 1.24.0-2\n  Candidate: 1.24.0-2\n  Version table:",
            Some("1.24.0-2"),
        ),
        (
            "cargo",
            "ripgrep = \"14.1.0\"    # Fast regex search\n",
            Some("14.1.0"),
        ),
        ("nix", "2.18.1", Some("2.18.1")),
        (
            "uv",
            "Available versions: 2.31.0, 2.30.0, 2.29.0\n",
            Some("2.31.0"),
        ),
        (
            "docker",
            "sha256:abc123def456\n",
            Some("sha256:abc123def456"),
        ),
        ("apt", "", None),
        ("cargo", "no matches found\n", None),
    ];

    for (provider, output, expected) in test_cases {
        let result = parse_resolved_version(provider, output);
        let display = result.as_deref().unwrap_or("None");
        let exp = expected.unwrap_or("None");
        let ok = result.as_deref() == expected;
        println!(
            "  {provider:8} output[..20]={:20} → {display:20} (expected {exp}) {}",
            &output[..output.len().min(20)],
            if ok { "OK" } else { "FAIL" }
        );
        assert_eq!(result.as_deref(), expected);
    }
    println!("  All version parsing tests passed");
}

fn demo_hash_determinism() {
    println!("\n--- 3. Pin Hash Determinism ---");

    let h1 = pin_hash("apt", "nginx", "1.24.0");
    let h2 = pin_hash("apt", "nginx", "1.24.0");
    assert_eq!(h1, h2);
    println!("  Same inputs → same hash: {}", &h1[..32]);

    let h3 = pin_hash("apt", "nginx", "1.25.0");
    assert_ne!(h1, h3);
    println!("  Different version → different hash: {}", &h3[..32]);

    let h4 = pin_hash("cargo", "nginx", "1.24.0");
    assert_ne!(h1, h4);
    println!("  Different provider → different hash: {}", &h4[..32]);

    let h5 = pin_hash("apt", "curl", "1.24.0");
    assert_ne!(h1, h5);
    println!("  Different name → different hash: {}", &h5[..32]);

    // Hash format
    assert!(h1.starts_with("blake3:"));
    println!("  Hash format: blake3:<hex>");
    println!("  Pin hash determinism verified");
}
