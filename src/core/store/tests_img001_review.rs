//! IMG-001 (forjar#756) review findings carried into epic #768: symlinks survive into the layer.

use super::layer_builder::{build_layer, LayerEntry};
use crate::core::types::OciLayerConfig;
use std::io::Read;

fn untar(compressed: &[u8]) -> Vec<(String, tar::EntryType, Option<String>)> {
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(compressed)
        .read_to_end(&mut raw)
        .unwrap();
    let mut ar = tar::Archive::new(raw.as_slice());
    ar.entries()
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            let link = e
                .link_name()
                .unwrap()
                .map(|l| l.to_string_lossy().to_string());
            (
                e.path().unwrap().to_string_lossy().to_string(),
                e.header().entry_type(),
                link,
            )
        })
        .collect()
}

#[test]
fn symlink_entry_is_a_symlink_in_the_layer_tar() {
    let entries = vec![LayerEntry::symlink(
        "usr/bin/cc",
        "/etc/alternatives/cc",
        0o777,
    )];
    let (_, blob) = build_layer(&entries, &OciLayerConfig::default()).unwrap();
    let got = untar(&blob);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].1, tar::EntryType::Symlink);
    assert_eq!(got[0].2.as_deref(), Some("/etc/alternatives/cc"));
}

#[test]
fn image_totals_count_the_base_layers_the_manifest_carries() {
    use super::image_assembler::{assemble_image_with_base, BaseLayout};
    use super::tests_image_assembler_img001 as t;
    use crate::core::types::{ImageBuildPlan, LayerStrategy};
    let src = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    t::write_base_layout(src.path());
    let base = super::base_image::extract_base_layers(src.path()).unwrap();
    let base_size: u64 = base.layers.iter().map(|l| l.size).sum();
    let plan = ImageBuildPlan {
        tag: "t:1".into(),
        base_image: None,
        layers: vec![LayerStrategy::Files {
            paths: vec!["/x".into()],
        }],
        labels: vec![],
        entrypoint: None,
    };
    let img = assemble_image_with_base(
        &plan,
        &[vec![LayerEntry::file("x", b"x", 0o644)]],
        out.path(),
        &OciLayerConfig::default(),
        None,
        Some(&BaseLayout {
            layout_dir: src.path().to_path_buf(),
            layers: base,
        }),
    )
    .unwrap();
    assert_eq!(img.manifest.layers.len(), 2);
    assert_eq!(
        img.layer_count(),
        img.manifest.layers.len(),
        "layer_count != manifest"
    );
    let manifest_total: u64 = img.manifest.layers.iter().map(|l| l.size).sum();
    assert_eq!(
        img.total_size, manifest_total,
        "total_size omits base layers"
    );
    assert!(img.total_size >= base_size + img.layers[0].compressed_size);
}
