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
