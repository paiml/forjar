//! Tests for OCI Distribution v1.1 registry push (FJ-2105).

use super::registry_push::*;
use crate::core::types::{PushKind, PushResult};

#[test]
fn validate_push_config_valid() {
    let config = RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: "myorg/myapp".into(),
        tag: "v1".into(),
        check_existing: true,
    };
    let errors = validate_push_config(&config);
    assert!(errors.is_empty());
}

#[test]
fn validate_push_config_empty_registry() {
    let config = RegistryPushConfig {
        registry: String::new(),
        name: "app".into(),
        tag: "v1".into(),
        check_existing: false,
    };
    let errors = validate_push_config(&config);
    assert!(errors.iter().any(|e| e.contains("registry")));
}

#[test]
fn validate_push_config_empty_name() {
    let config = RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: String::new(),
        tag: "v1".into(),
        check_existing: false,
    };
    let errors = validate_push_config(&config);
    assert!(errors.iter().any(|e| e.contains("name")));
}

#[test]
fn validate_push_config_empty_tag() {
    let config = RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: "app".into(),
        tag: String::new(),
        check_existing: false,
    };
    let errors = validate_push_config(&config);
    assert!(errors.iter().any(|e| e.contains("tag")));
}

#[test]
fn validate_push_config_url_instead_of_host() {
    let config = RegistryPushConfig {
        registry: "https://ghcr.io".into(),
        name: "app".into(),
        tag: "v1".into(),
        check_existing: false,
    };
    let errors = validate_push_config(&config);
    assert!(errors.iter().any(|e| e.contains("hostname")));
}

#[test]
fn head_check_command_format() {
    let cmd = head_check_command("ghcr.io", "myorg/app", "sha256:abc123");
    assert!(cmd.contains("--head"));
    assert!(cmd.contains("ghcr.io"));
    assert!(cmd.contains("myorg/app"));
    assert!(cmd.contains("sha256:abc123"));
    assert!(cmd.contains("/v2/"));
    assert!(cmd.contains("/blobs/"));
}

#[test]
fn upload_initiate_command_format() {
    let cmd = upload_initiate_command("docker.io", "library/alpine");
    assert!(cmd.contains("-X POST"));
    assert!(cmd.contains("/v2/library/alpine/blobs/uploads/"));
}

#[test]
fn upload_complete_command_format() {
    let cmd = upload_complete_command(
        "https://docker.io/v2/library/alpine/blobs/uploads/uuid-123",
        "sha256:abc",
        "/tmp/layer.tar.gz",
    );
    assert!(cmd.contains("-X PUT"));
    assert!(cmd.contains("@/tmp/layer.tar.gz"));
    assert!(cmd.contains("digest=sha256:abc"));
}

#[test]
fn manifest_put_command_format() {
    let cmd = manifest_put_command("ghcr.io", "myorg/app", "v1.0", "/tmp/manifest.json");
    assert!(cmd.contains("-X PUT"));
    assert!(cmd.contains("application/vnd.oci.image.manifest.v1+json"));
    assert!(cmd.contains("/v2/myorg/app/manifests/v1.0"));
    assert!(cmd.contains("@/tmp/manifest.json"));
}

#[test]
fn parse_location_header_found() {
    let headers = "HTTP/1.1 202 Accepted\r\n\
                   Location: https://ghcr.io/v2/myorg/app/blobs/uploads/uuid-456\r\n\
                   Content-Length: 0\r\n";
    let loc = parse_location_header(headers);
    assert_eq!(
        loc.as_deref(),
        Some("https://ghcr.io/v2/myorg/app/blobs/uploads/uuid-456")
    );
}

#[test]
fn parse_location_header_case_insensitive() {
    let headers = "HTTP/1.1 202 Accepted\r\nlocation: https://example.com/upload\r\n";
    let loc = parse_location_header(headers);
    assert_eq!(loc.as_deref(), Some("https://example.com/upload"));
}

#[test]
fn parse_location_header_missing() {
    let headers = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n";
    let loc = parse_location_header(headers);
    assert!(loc.is_none());
}

#[test]
fn format_push_summary_all_new() {
    let results = vec![
        PushResult {
            kind: PushKind::Layer,
            digest: "sha256:aaa".into(),
            size: 1_000_000,
            existed: false,
            duration_secs: 1.5,
        },
        PushResult {
            kind: PushKind::Config,
            digest: "sha256:bbb".into(),
            size: 512,
            existed: false,
            duration_secs: 0.1,
        },
        PushResult {
            kind: PushKind::Manifest,
            digest: "sha256:ccc".into(),
            size: 2048,
            existed: false,
            duration_secs: 0.2,
        },
    ];
    let summary = format_push_summary(&results);
    assert!(summary.contains("3 uploaded"));
    assert!(summary.contains("0 skipped"));
    assert!(summary.contains("[push] layer"));
    assert!(summary.contains("[push] config"));
    assert!(summary.contains("[push] manifest"));
}

#[test]
fn format_push_summary_mixed() {
    let results = vec![
        PushResult {
            kind: PushKind::Layer,
            digest: "sha256:aaa".into(),
            size: 5_000_000,
            existed: true,
            duration_secs: 0.0,
        },
        PushResult {
            kind: PushKind::Layer,
            digest: "sha256:bbb".into(),
            size: 3_000_000,
            existed: false,
            duration_secs: 2.0,
        },
    ];
    let summary = format_push_summary(&results);
    assert!(summary.contains("1 uploaded"));
    assert!(summary.contains("1 skipped"));
    assert!(summary.contains("[skip] layer"));
    assert!(summary.contains("[push] layer"));
}

#[test]
fn format_push_summary_empty() {
    let results: Vec<PushResult> = Vec::new();
    let summary = format_push_summary(&results);
    assert!(summary.contains("0 uploaded"));
    assert!(summary.contains("0 skipped"));
}

#[test]
fn blob_descriptor_creation() {
    let blob = BlobDescriptor {
        digest: "sha256:abc".into(),
        size: 1024,
        path: "/tmp/blob".into(),
        kind: PushKind::Layer,
    };
    assert_eq!(blob.digest, "sha256:abc");
    assert_eq!(blob.size, 1024);
}

#[test]
fn push_image_missing_oci_dir() {
    let config = RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: "app".into(),
        tag: "v1".into(),
        check_existing: true,
    };
    let result = push_image(std::path::Path::new("/nonexistent/oci"), &config);
    assert!(result.is_err());
}

#[test]
fn push_image_missing_index() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("blobs/sha256")).unwrap();
    let config = RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: "app".into(),
        tag: "v1".into(),
        check_existing: false,
    };
    let result = push_image(dir.path(), &config);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("index.json"));
}

#[test]
fn discover_blobs_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("blobs/sha256")).unwrap();
    let blobs = discover_blobs(dir.path()).unwrap();
    assert!(blobs.is_empty());
}

#[test]
fn discover_blobs_with_files() {
    let dir = tempfile::tempdir().unwrap();
    let blobs_dir = dir.path().join("blobs/sha256");
    std::fs::create_dir_all(&blobs_dir).unwrap();
    std::fs::write(blobs_dir.join("abc123"), b"layer data").unwrap();
    std::fs::write(blobs_dir.join("def456"), b"config data").unwrap();

    let blobs = discover_blobs(dir.path()).unwrap();
    assert_eq!(blobs.len(), 2);
    assert!(blobs.iter().all(|b| b.digest.starts_with("sha256:")));
}

#[test]
fn discover_blobs_no_dir() {
    let dir = tempfile::tempdir().unwrap();
    let blobs = discover_blobs(dir.path()).unwrap();
    assert!(blobs.is_empty());
}

#[test]
fn validate_push_config_multiple_errors() {
    let config = RegistryPushConfig {
        registry: String::new(),
        name: String::new(),
        tag: String::new(),
        check_existing: false,
    };
    let errors = validate_push_config(&config);
    assert_eq!(errors.len(), 3);
}

#[test]
fn push_result_index_kind() {
    let r = PushResult {
        kind: PushKind::Index,
        digest: "sha256:idx".into(),
        size: 256,
        existed: false,
        duration_secs: 0.1,
    };
    let summary = format_push_summary(&[r]);
    assert!(summary.contains("[push] index"));
}

/// F27: discover_blobs classifies config/layer/manifest from index.json chain.
#[test]
fn discover_blobs_classifies_from_index() {
    let dir = tempfile::tempdir().unwrap();
    let blobs_dir = dir.path().join("blobs/sha256");
    std::fs::create_dir_all(&blobs_dir).unwrap();

    // Create layer blob
    let layer_data = b"layer tarball data";
    let layer_hash = blake3::hash(layer_data).to_hex().to_string();
    std::fs::write(blobs_dir.join(&layer_hash), layer_data).unwrap();

    // Create config blob
    let config_json = r#"{"architecture":"amd64","os":"linux","rootfs":{"type":"layers","diff_ids":[]}}"#;
    let config_hash = blake3::hash(config_json.as_bytes()).to_hex().to_string();
    std::fs::write(blobs_dir.join(&config_hash), config_json.as_bytes()).unwrap();

    // Create manifest referencing config + layer
    let manifest_json = format!(
        r#"{{"schemaVersion":2,"mediaType":"application/vnd.oci.image.manifest.v1+json","config":{{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"sha256:{config_hash}","size":{config_size}}},"layers":[{{"mediaType":"application/vnd.oci.image.layer.v1.tar+gzip","digest":"sha256:{layer_hash}","size":{layer_size}}}]}}"#,
        config_hash = config_hash,
        config_size = config_json.len(),
        layer_hash = layer_hash,
        layer_size = layer_data.len(),
    );
    let manifest_hash = blake3::hash(manifest_json.as_bytes()).to_hex().to_string();
    std::fs::write(blobs_dir.join(&manifest_hash), manifest_json.as_bytes()).unwrap();

    // Create index.json referencing manifest
    let index_json = format!(
        r#"{{"schemaVersion":2,"manifests":[{{"mediaType":"application/vnd.oci.image.manifest.v1+json","digest":"sha256:{manifest_hash}","size":{manifest_size}}}]}}"#,
        manifest_hash = manifest_hash,
        manifest_size = manifest_json.len(),
    );
    std::fs::write(dir.path().join("index.json"), index_json.as_bytes()).unwrap();

    let blobs = discover_blobs(dir.path()).unwrap();
    assert_eq!(blobs.len(), 3);

    let layer = blobs.iter().find(|b| b.digest.contains(&layer_hash)).unwrap();
    assert_eq!(layer.kind, PushKind::Layer);

    let config = blobs.iter().find(|b| b.digest.contains(&config_hash)).unwrap();
    assert_eq!(config.kind, PushKind::Config);

    let manifest = blobs.iter().find(|b| b.digest.contains(&manifest_hash)).unwrap();
    assert_eq!(manifest.kind, PushKind::Manifest);
}
