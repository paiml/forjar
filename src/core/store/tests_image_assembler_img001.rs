//! IMG-001 (paiml/infra#2074): the image assembler must (a) report the manifest
//! digest, (b) name the image so `docker load` keeps the tag, and (c) layer a
//! declared base image under the built layers instead of emitting a scratch image.

use super::base_image::{extract_base_layers, BaseImageLayers};
use super::image_assembler::*;
use super::layer_builder::LayerEntry;
use crate::core::types::{
    ImageBuildPlan, LayerStrategy, OciDescriptor, OciImageConfig, OciIndex, OciLayerConfig,
    OciManifest,
};
use std::path::Path;

const BASE_CONFIG_HEX: &str = "aabbccdd00112233445566778899aabbccddeeff00112233445566778899aabb";
const BASE_LAYER_HEX: &str = "11223344556677889900aabbccddeeff00112233445566778899aabbccddeeff";
const BASE_DIFF_ID: &str =
    "sha256:99887766554433221100ffeeddccbbaa99887766554433221100ffeeddccbbaa";
pub(super) const BASE_MANIFEST_HEX: &str =
    "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";

fn plan() -> ImageBuildPlan {
    ImageBuildPlan {
        tag: "almacen:5000/paiml/aprender-build:spike".into(),
        base_image: None,
        layers: vec![LayerStrategy::Files {
            paths: vec!["/opt/x".into()],
        }],
        labels: vec![],
        entrypoint: None,
    }
}

fn entries() -> Vec<Vec<LayerEntry>> {
    vec![vec![LayerEntry::file("opt/x", b"x\n", 0o644)]]
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// A tiny but structurally valid single-layer base layout, as `docker save`
/// of a pulled image produces.
pub(super) fn write_base_layout(dir: &Path) {
    let blobs = dir.join("blobs/sha256");
    std::fs::create_dir_all(&blobs).unwrap();
    let mut cfg = OciImageConfig::linux_amd64(vec![BASE_DIFF_ID.into()]);
    cfg.config.env = vec!["PATH=/usr/local/sbin:/usr/bin".into()];
    std::fs::write(
        blobs.join(BASE_CONFIG_HEX),
        serde_json::to_vec(&cfg).unwrap(),
    )
    .unwrap();
    std::fs::write(blobs.join(BASE_LAYER_HEX), b"base-layer-bytes").unwrap();
    let manifest = OciManifest::new(
        format!("sha256:{BASE_CONFIG_HEX}"),
        vec![OciDescriptor::gzip_layer(
            format!("sha256:{BASE_LAYER_HEX}"),
            16,
        )],
    );
    std::fs::write(
        blobs.join(BASE_MANIFEST_HEX),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let index = OciIndex::single(OciDescriptor {
        media_type: "application/vnd.oci.image.manifest.v1+json".into(),
        digest: format!("sha256:{BASE_MANIFEST_HEX}"),
        size: 1,
        annotations: Default::default(),
    });
    std::fs::write(dir.join("index.json"), serde_json::to_vec(&index).unwrap()).unwrap();
}

fn base(dir: &Path) -> BaseImageLayers {
    write_base_layout(dir);
    extract_base_layers(dir).unwrap()
}

#[test]
fn img001_assembled_image_reports_the_manifest_digest() {
    let out = tempfile::tempdir().unwrap();
    let img = assemble_image(
        &plan(),
        &entries(),
        out.path(),
        &OciLayerConfig::default(),
        None,
    )
    .unwrap();
    let idx = read_json(&out.path().join("index.json"));
    let in_index = idx["manifests"][0]["digest"].as_str().unwrap();
    assert!(img.manifest_digest.starts_with("sha256:"));
    assert_eq!(img.manifest_digest, in_index);
    let hex = img.manifest_digest.strip_prefix("sha256:").unwrap();
    assert!(out.path().join(format!("blobs/sha256/{hex}")).exists());
}

#[test]
fn img001_index_names_the_image_so_load_keeps_the_tag() {
    let out = tempfile::tempdir().unwrap();
    assemble_image(
        &plan(),
        &entries(),
        out.path(),
        &OciLayerConfig::default(),
        None,
    )
    .unwrap();
    let tag = "almacen:5000/paiml/aprender-build:spike";
    let idx = read_json(&out.path().join("index.json"));
    let ann = &idx["manifests"][0]["annotations"];
    assert_eq!(ann["org.opencontainers.image.ref.name"], tag);
    assert_eq!(ann["io.containerd.image.name"], tag);
    let dm = read_json(&out.path().join("manifest.json"));
    assert_eq!(dm[0]["RepoTags"][0], tag);
}

#[test]
fn img001_base_layers_come_first_and_their_blobs_travel_with_the_image() {
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let b = base(src.path());
    let img = assemble_image_with_base(
        &plan(),
        &entries(),
        out.path(),
        &OciLayerConfig::default(),
        None,
        Some(&BaseLayout {
            layout_dir: src.path().to_path_buf(),
            layers: b,
        }),
    )
    .unwrap();
    assert_eq!(img.manifest.layers.len(), 2, "base layer + built layer");
    assert_eq!(
        img.manifest.layers[0].digest,
        format!("sha256:{BASE_LAYER_HEX}")
    );
    assert_eq!(img.config.rootfs.diff_ids.len(), 2);
    assert_eq!(img.config.rootfs.diff_ids[0], BASE_DIFF_ID);
    assert!(out
        .path()
        .join(format!("blobs/sha256/{BASE_LAYER_HEX}"))
        .exists());
    assert_eq!(img.config.config.env, vec!["PATH=/usr/local/sbin:/usr/bin"]);
    let dm = read_json(&out.path().join("manifest.json"));
    assert_eq!(dm[0]["Layers"].as_array().unwrap().len(), 2);
    assert_eq!(dm[0]["Layers"][0], format!("blobs/sha256/{BASE_LAYER_HEX}"));
}
