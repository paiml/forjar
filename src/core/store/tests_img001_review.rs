//! IMG-001 (forjar#756) review findings carried into epic #768: symlinks and
//! empty directories survive into the layer, the layer reports base layers.

use super::layer_builder::{build_layer, LayerEntry};
use super::overlay_export::{merge_overlay_entries, scan_overlay_upper};
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
fn overlay_scan_keeps_an_absolute_symlink_as_a_link_not_host_content() {
    let tmp = tempfile::tempdir().unwrap();
    let up = tmp.path().join("up");
    std::fs::create_dir_all(up.join("usr/bin")).unwrap();
    // Absolute target that EXISTS on the host: following it leaks host bytes.
    std::os::unix::fs::symlink("/etc/hostname", up.join("usr/bin/cc")).unwrap();
    // Dangling link: following it would drop the entry.
    std::os::unix::fs::symlink("/no/such/target", up.join("usr/bin/dangling")).unwrap();
    // Link to a directory: following it would recurse into the host tree.
    std::os::unix::fs::symlink("/etc", up.join("usr/bin/etcdir")).unwrap();

    let scan = scan_overlay_upper(&up, &up).unwrap();
    let all = merge_overlay_entries(&scan);
    for name in ["usr/bin/cc", "usr/bin/dangling", "usr/bin/etcdir"] {
        let e = all
            .iter()
            .find(|e| e.path == name)
            .unwrap_or_else(|| panic!("{name} missing from {all:?}"));
        assert!(e.link_target.is_some(), "{name} not a link");
        assert!(e.content.is_empty(), "{name} carries host bytes");
    }
    assert!(
        !all.iter().any(|e| e.path.starts_with("usr/bin/etcdir/")),
        "descended through a symlinked dir"
    );
}

#[test]
fn overlay_scan_keeps_an_empty_directory_and_its_mode() {
    let tmp = tempfile::tempdir().unwrap();
    let up = tmp.path().join("up");
    std::fs::create_dir_all(up.join("var/cache/apt/archives/partial")).unwrap();
    let scan = scan_overlay_upper(&up, &up).unwrap();
    let all = merge_overlay_entries(&scan);
    assert!(
        all.iter()
            .any(|e| e.is_dir && e.path == "var/cache/apt/archives/partial/"),
        "empty dir lost: {all:?}"
    );
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
