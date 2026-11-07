//! IMG-001 quorum defect C1: a digest-pinned local base must layer the manifest
//! the pin names, not whichever manifest happens to be first in index.json.

use super::base_resolve::load_local_base;
use super::sandbox_exec::sha256_digest;
use std::path::Path;

fn put(blobs: &Path, bytes: &[u8]) -> String {
    let d = sha256_digest(bytes);
    std::fs::write(blobs.join(d.trim_start_matches("sha256:")), bytes).unwrap();
    d
}

/// Writes one single-layer image into `blobs`; returns (manifest digest, layer digest).
fn image(blobs: &Path, tag: &str) -> (String, String) {
    let layer = put(blobs, format!("layer-{tag}").as_bytes());
    let diff = sha256_digest(format!("diff-{tag}").as_bytes());
    let cfg = serde_json::json!({
        "architecture": "amd64", "os": "linux",
        "rootfs": {"type": "layers", "diff_ids": [diff]},
        "config": {}
    });
    let cfg_d = put(blobs, cfg.to_string().as_bytes());
    let m = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {"mediaType": "application/vnd.oci.image.config.v1+json", "digest": cfg_d, "size": 1},
        "layers": [{"mediaType": "application/vnd.oci.image.layer.v1.tar+gzip", "digest": layer, "size": 1}]
    });
    (put(blobs, m.to_string().as_bytes()), layer)
}

#[test]
fn pinned_base_layers_the_manifest_the_pin_names_not_manifests_zero() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("staging");
    let blobs = dir.join("blobs/sha256");
    std::fs::create_dir_all(&blobs).unwrap();
    let (other_m, other_layer) = image(&blobs, "other");
    let (pinned_m, pinned_layer) = image(&blobs, "pinned");
    let index = serde_json::json!({
        "schemaVersion": 2,
        "manifests": [
            {"mediaType": "application/vnd.oci.image.manifest.v1+json", "digest": other_m, "size": 1},
            {"mediaType": "application/vnd.oci.image.manifest.v1+json", "digest": pinned_m, "size": 1}
        ]
    });
    std::fs::write(dir.join("index.json"), index.to_string()).unwrap();

    let pinned_ref = format!("reg.example/app@{pinned_m}");
    std::fs::rename(
        &dir,
        root.path()
            .join(super::base_resolve::local_base_dir_name(&pinned_ref)),
    )
    .unwrap();
    let got = load_local_base(root.path(), &pinned_ref).unwrap().unwrap();
    assert_eq!(got.layers.layers.len(), 1);
    assert_eq!(
        got.layers.layers[0].digest, pinned_layer,
        "layered the image at manifests[0], not the pinned one"
    );
    assert_ne!(got.layers.layers[0].digest, other_layer);
}
