//! Example: OCI Distribution v1.1 registry push protocol.
//!
//! Demonstrates the push pipeline types and command generation.
//! Run with: `cargo run --example registry_push`

fn main() {
    use forjar::core::store::registry_push;
    use forjar::core::types::{PushKind, PushResult};

    println!("=== OCI Distribution v1.1 Push Protocol ===\n");

    // 1. Validate push config
    let config = registry_push::RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: "myorg/myapp".into(),
        tag: "v1.0.0".into(),
        check_existing: true,
    };
    let errors = registry_push::validate_push_config(&config);
    println!("Config validation: {} errors", errors.len());

    // 2. Show protocol commands
    println!("\nProtocol commands:");
    println!(
        "  HEAD check: {}",
        registry_push::head_check_command("ghcr.io", "myorg/myapp", "sha256:abc123")
    );
    println!(
        "  Initiate: {}",
        registry_push::upload_initiate_command("ghcr.io", "myorg/myapp")
    );
    println!(
        "  Complete: {}",
        registry_push::upload_complete_command(
            "https://ghcr.io/v2/myorg/myapp/blobs/uploads/uuid-123",
            "sha256:abc123",
            "/tmp/layer.tar.gz",
        )
    );
    println!(
        "  Manifest: {}",
        registry_push::manifest_put_command(
            "ghcr.io",
            "myorg/myapp",
            "v1.0.0",
            "/tmp/manifest.json",
        )
    );

    // 3. Format push summary
    let results = vec![
        PushResult {
            kind: PushKind::Layer,
            digest: "sha256:base_layer_abc".into(),
            size: 45_000_000,
            existed: true,
            duration_secs: 0.0,
        },
        PushResult {
            kind: PushKind::Layer,
            digest: "sha256:app_layer_def".into(),
            size: 5_000_000,
            existed: false,
            duration_secs: 2.3,
        },
        PushResult {
            kind: PushKind::Config,
            digest: "sha256:config_ghi".into(),
            size: 1024,
            existed: false,
            duration_secs: 0.1,
        },
        PushResult {
            kind: PushKind::Manifest,
            digest: "sha256:manifest_jkl".into(),
            size: 2048,
            existed: false,
            duration_secs: 0.2,
        },
    ];
    println!("\n{}", registry_push::format_push_summary(&results));
}
