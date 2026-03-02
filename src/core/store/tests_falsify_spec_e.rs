//! Spec falsification tests: Phases G–H (FAR archive, Convert)
//!
//! Split from tests_falsify_spec_b.rs to stay under 500-line limit.
#![allow(unused_imports)]

use super::convert::{
    analyze_conversion, ChangeType, ConversionChange, ConversionReport, ConversionSignals,
};
use super::far::{
    decode_far_manifest, encode_far, ChunkEntry, FarFileEntry, FarManifest, FarProvenance,
    FAR_MAGIC,
};
use super::purity::PurityLevel;

// ═══════════════════════════════════════════════════════════════════
// Phase G: FAR Archive
// ═══════════════════════════════════════════════════════════════════

/// G-01: FAR magic is 12 bytes.
#[test]
fn falsify_g01_far_magic_size() {
    assert_eq!(FAR_MAGIC.len(), 12, "FAR magic must be exactly 12 bytes");
}

/// G-02: FAR magic starts with "FORJAR-FAR".
#[test]
fn falsify_g02_far_magic_content() {
    assert!(
        FAR_MAGIC.starts_with(b"FORJAR-FAR"),
        "FAR magic must start with 'FORJAR-FAR'"
    );
}

/// G-03: encode_far → decode_far_manifest roundtrip.
#[test]
fn falsify_g03_far_roundtrip() {
    let manifest = FarManifest {
        name: "test".to_string(),
        version: "1.0.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:abc".to_string(),
        tree_hash: "blake3:def".to_string(),
        file_count: 1,
        total_size: 100,
        files: vec![FarFileEntry {
            path: "bin/test".to_string(),
            size: 100,
            blake3: "blake3:eee".to_string(),
        }],
        provenance: FarProvenance {
            origin_provider: "cargo".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 1.0.0".to_string(),
        },
        kernel_contracts: None,
    };
    let chunk_hash = [0u8; 32];
    let chunk_data = vec![1, 2, 3, 4, 5];
    let mut buf = Vec::new();
    encode_far(&manifest, &[(chunk_hash, chunk_data)], &mut buf).expect("encode");
    let (decoded, chunks) = decode_far_manifest(buf.as_slice()).expect("decode");
    assert_eq!(decoded.name, "test");
    assert_eq!(decoded.version, "1.0.0");
    assert_eq!(chunks.len(), 1);
}

/// G-04: decode_far_manifest rejects invalid magic.
#[test]
fn falsify_g04_far_invalid_magic() {
    let bad = b"NOT-A-FAR\x00\x00\x00";
    let result = decode_far_manifest(bad.as_slice());
    assert!(result.is_err(), "invalid magic must be rejected");
}

/// G-05: FarManifest has required fields.
#[test]
fn falsify_g05_far_manifest_fields() {
    let m = FarManifest {
        name: "ripgrep".to_string(),
        version: "14.1.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:abc".to_string(),
        tree_hash: "blake3:def".to_string(),
        file_count: 1,
        total_size: 5242880,
        files: vec![],
        provenance: FarProvenance {
            origin_provider: "cargo".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2026-03-02T10:00:00Z".to_string(),
            generator: "forjar 1.0.0".to_string(),
        },
        kernel_contracts: None,
    };
    assert_eq!(m.name, "ripgrep");
    assert_eq!(m.version, "14.1.0");
    assert_eq!(m.arch, "x86_64");
}

/// G-06: ChunkEntry has hash, offset, length fields.
#[test]
fn falsify_g06_chunk_entry_fields() {
    let entry = ChunkEntry {
        hash: [0xAA; 32],
        offset: 0,
        length: 65536,
    };
    assert_eq!(entry.hash.len(), 32);
    assert_eq!(entry.length, 65536);
}

/// G-07: FAR uses zstd compression (verified via roundtrip).
#[test]
fn falsify_g07_far_zstd_compression() {
    let manifest = FarManifest {
        name: "t".to_string(),
        version: "1.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:a".to_string(),
        tree_hash: "blake3:b".to_string(),
        file_count: 0,
        total_size: 0,
        files: vec![],
        provenance: FarProvenance {
            origin_provider: "apt".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2026-01-01".to_string(),
            generator: "test".to_string(),
        },
        kernel_contracts: None,
    };
    let mut buf = Vec::new();
    encode_far(&manifest, &[], &mut buf).expect("encode");
    assert!(buf.len() > 20, "encoded FAR must contain compressed data");
    let (m, _) = decode_far_manifest(buf.as_slice()).expect("decode");
    assert_eq!(m.name, "t");
}

// ═══════════════════════════════════════════════════════════════════
// Phase H: Convert
// ═══════════════════════════════════════════════════════════════════

/// H-01: 5-step conversion ladder — auto steps 1-3, manual steps 4-5.
#[test]
fn falsify_h01_conversion_auto_steps() {
    let signals = vec![ConversionSignals {
        name: "nginx".to_string(),
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        provider: "apt".to_string(),
        current_version: None,
    }];
    let report = analyze_conversion(&signals);
    let auto_types: Vec<_> = report.resources[0]
        .auto_changes
        .iter()
        .map(|c| c.change_type.clone())
        .collect();
    assert!(auto_types.contains(&ChangeType::AddVersionPin));
    assert!(auto_types.contains(&ChangeType::EnableStore));
    assert!(auto_types.contains(&ChangeType::GenerateLockPin));
}

/// H-02: Manual steps reported for sandbox (step 4).
#[test]
fn falsify_h02_conversion_manual_sandbox() {
    let signals = vec![ConversionSignals {
        name: "nginx".to_string(),
        has_version: true,
        has_store: true,
        has_sandbox: false,
        has_curl_pipe: false,
        provider: "apt".to_string(),
        current_version: Some("1.24.0".to_string()),
    }];
    let report = analyze_conversion(&signals);
    assert!(!report.resources[0].manual_changes.is_empty());
}

/// H-03: curl|bash produces Impure + manual step 5 report.
#[test]
fn falsify_h03_conversion_curl_pipe() {
    let signals = vec![ConversionSignals {
        name: "rustup".to_string(),
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: true,
        provider: "apt".to_string(),
        current_version: None,
    }];
    let report = analyze_conversion(&signals);
    assert_eq!(report.resources[0].current_purity, PurityLevel::Impure);
    assert!(report.resources[0]
        .manual_changes
        .iter()
        .any(|m| m.contains("curl|bash")));
}

/// H-04: ConversionReport tracks change counts.
#[test]
fn falsify_h04_conversion_report_counts() {
    let signals = vec![
        ConversionSignals {
            name: "a".to_string(),
            has_version: false,
            has_store: false,
            has_sandbox: false,
            has_curl_pipe: false,
            provider: "apt".to_string(),
            current_version: None,
        },
        ConversionSignals {
            name: "b".to_string(),
            has_version: true,
            has_store: true,
            has_sandbox: true,
            has_curl_pipe: false,
            provider: "cargo".to_string(),
            current_version: Some("1.0".to_string()),
        },
    ];
    let report = analyze_conversion(&signals);
    assert!(report.auto_change_count > 0);
    assert_eq!(report.resources.len(), 2);
}

/// H-05: Already-pure resource produces no auto changes.
#[test]
fn falsify_h05_pure_no_auto_changes() {
    let signals = vec![ConversionSignals {
        name: "rg".to_string(),
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        provider: "cargo".to_string(),
        current_version: Some("14.1.0".to_string()),
    }];
    let report = analyze_conversion(&signals);
    assert!(report.resources[0].auto_changes.is_empty());
}
