use super::*;

#[test]
fn test_fj242_compute_signatures_single_block() {
    let data = vec![0u8; 100]; // Less than BLOCK_SIZE
    let sigs = compute_signatures(&data);
    assert_eq!(sigs.len(), 1);
    assert_eq!(sigs[0].index, 0);
    assert!(!sigs[0].hash.is_empty());
}

#[test]
fn test_fj242_compute_signatures_multiple_blocks() {
    let data = vec![0u8; BLOCK_SIZE * 3 + 100]; // 3 full + 1 partial
    let sigs = compute_signatures(&data);
    assert_eq!(sigs.len(), 4);
    for (i, sig) in sigs.iter().enumerate() {
        assert_eq!(sig.index, i);
    }
    // First 3 blocks are identical (all zeros), so same hash
    assert_eq!(sigs[0].hash, sigs[1].hash);
    assert_eq!(sigs[1].hash, sigs[2].hash);
    // Last block is shorter, different hash
    assert_ne!(sigs[2].hash, sigs[3].hash);
}

#[test]
fn test_fj242_compute_signatures_deterministic() {
    let data = b"hello world copia delta sync test data";
    let sigs1 = compute_signatures(data);
    let sigs2 = compute_signatures(data);
    assert_eq!(sigs1, sigs2);
}

#[test]
fn test_fj242_compute_delta_identical() {
    let data = vec![42u8; BLOCK_SIZE * 5];
    let sigs = compute_signatures(&data);
    let delta = compute_delta(&data, &sigs);
    assert_eq!(delta.len(), 5);
    assert_eq!(literal_count(&delta), 0); // All copies, no changes
}

#[test]
fn test_fj242_compute_delta_all_different() {
    let old_data = vec![0u8; BLOCK_SIZE * 3];
    let new_data = vec![1u8; BLOCK_SIZE * 3];
    let sigs = compute_signatures(&old_data);
    let delta = compute_delta(&new_data, &sigs);
    assert_eq!(delta.len(), 3);
    assert_eq!(literal_count(&delta), 3); // All literals, nothing matches
}

#[test]
fn test_fj242_compute_delta_partial_change() {
    // 4 blocks, change only block 1 and 3
    let old_data = vec![0u8; BLOCK_SIZE * 4];
    let sigs = compute_signatures(&old_data);

    let mut new_data = old_data.clone();
    // Modify block 1
    new_data[BLOCK_SIZE] = 0xFF;
    // Modify block 3
    new_data[BLOCK_SIZE * 3] = 0xFF;

    let delta = compute_delta(&new_data, &sigs);
    assert_eq!(delta.len(), 4);
    assert_eq!(literal_count(&delta), 2); // Only 2 blocks changed
    assert!(matches!(delta[0], DeltaOp::Copy { index: 0 }));
    assert!(matches!(delta[1], DeltaOp::Literal { .. }));
    assert!(matches!(delta[2], DeltaOp::Copy { index: 2 }));
    assert!(matches!(delta[3], DeltaOp::Literal { .. }));
}

#[test]
fn test_fj242_compute_delta_new_file_longer() {
    // Old file: 2 blocks, new file: 4 blocks (2 new blocks appended)
    let old_data = vec![0u8; BLOCK_SIZE * 2];
    let sigs = compute_signatures(&old_data);

    let mut new_data = vec![0u8; BLOCK_SIZE * 4];
    new_data[BLOCK_SIZE * 2..].fill(1); // New blocks are different

    let delta = compute_delta(&new_data, &sigs);
    assert_eq!(delta.len(), 4);
    assert!(matches!(delta[0], DeltaOp::Copy { index: 0 }));
    assert!(matches!(delta[1], DeltaOp::Copy { index: 1 }));
    assert!(matches!(delta[2], DeltaOp::Literal { .. })); // New block
    assert!(matches!(delta[3], DeltaOp::Literal { .. })); // New block
    assert_eq!(literal_count(&delta), 2);
}

#[test]
fn test_fj242_literal_bytes_count() {
    let ops = vec![
        DeltaOp::Copy { index: 0 },
        DeltaOp::Literal {
            data: vec![0u8; 100],
        },
        DeltaOp::Literal {
            data: vec![0u8; 200],
        },
        DeltaOp::Copy { index: 3 },
    ];
    assert_eq!(literal_bytes(&ops), 300);
    assert_eq!(literal_count(&ops), 2);
}

#[test]
fn test_fj242_signature_script_generates_valid_shell() {
    let script = signature_script("/opt/models/llama.gguf");
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("/opt/models/llama.gguf"));
    assert!(script.contains("NEW_FILE"));
    assert!(script.contains("b3sum"));
    assert!(script.contains("sha256sum")); // fallback
    assert!(script.contains(&BLOCK_SIZE.to_string()));
}

#[test]
fn test_fj242_parse_signatures_new_file() {
    let output = "NEW_FILE\n";
    let result = parse_signatures(output).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_fj242_parse_signatures_valid() {
    let output = "SIZE:12288\n0 abc123\n1 def456\n2 ghi789\n";
    let sigs = parse_signatures(output).unwrap().unwrap();
    assert_eq!(sigs.len(), 3);
    assert_eq!(sigs[0].index, 0);
    assert_eq!(sigs[0].hash, "abc123");
    assert_eq!(sigs[1].index, 1);
    assert_eq!(sigs[1].hash, "def456");
    assert_eq!(sigs[2].index, 2);
    assert_eq!(sigs[2].hash, "ghi789");
}

#[test]
fn test_fj242_parse_signatures_empty_lines() {
    let output = "\nSIZE:4096\n\n0 hash0\n\n";
    let sigs = parse_signatures(output).unwrap().unwrap();
    assert_eq!(sigs.len(), 1);
}

#[test]
fn test_fj242_parse_signatures_invalid_index() {
    let output = "SIZE:4096\nabc hash0\n";
    let result = parse_signatures(output);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid block index"));
}

#[test]
fn test_fj242_parse_signatures_invalid_line() {
    let output = "SIZE:4096\njustahash\n";
    let result = parse_signatures(output);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid signature line"));
}

#[test]
fn test_fj242_patch_script_copy_and_literal() {
    let ops = vec![
        DeltaOp::Copy { index: 0 },
        DeltaOp::Literal {
            data: b"hello".to_vec(),
        },
        DeltaOp::Copy { index: 2 },
    ];
    let script = patch_script("/opt/model.gguf", &ops, Some("noah"), None, Some("0644"));
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("dd if='/opt/model.gguf'"));
    assert!(script.contains("skip=0 count=1"));
    assert!(script.contains("base64 -d"));
    assert!(script.contains("skip=2 count=1"));
    assert!(script.contains("mv \"$TMPFILE\" '/opt/model.gguf'"));
    assert!(script.contains("chown 'noah' '/opt/model.gguf'"));
    assert!(script.contains("chmod '0644' '/opt/model.gguf'"));
}

#[test]
fn test_fj242_patch_script_owner_and_group() {
    let ops = vec![DeltaOp::Copy { index: 0 }];
    let script = patch_script("/etc/data", &ops, Some("app"), Some("www-data"), None);
    assert!(script.contains("chown 'app:www-data' '/etc/data'"));
    assert!(!script.contains("chmod"));
}

#[test]
fn test_fj242_patch_script_no_ownership() {
    let ops = vec![DeltaOp::Literal {
        data: b"data".to_vec(),
    }];
    let script = patch_script("/tmp/test", &ops, None, None, None);
    assert!(!script.contains("chown"));
    assert!(!script.contains("chmod"));
}

#[test]
fn test_fj242_is_eligible_nonexistent() {
    assert!(!is_eligible("/nonexistent/path/that/does/not/exist"));
}

#[test]
fn test_fj242_is_eligible_small_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("small.txt");
    std::fs::write(&path, "hello").unwrap();
    assert!(!is_eligible(path.to_str().unwrap()));
}

#[test]
fn test_fj242_is_eligible_large_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.bin");
    let data = vec![0u8; SIZE_THRESHOLD as usize + 1];
    std::fs::write(&path, &data).unwrap();
    assert!(is_eligible(path.to_str().unwrap()));
}

#[test]
fn test_fj242_is_eligible_exact_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("exact.bin");
    let data = vec![0u8; SIZE_THRESHOLD as usize];
    std::fs::write(&path, &data).unwrap();
    assert!(!is_eligible(path.to_str().unwrap())); // Must be > threshold, not >=
}

#[test]
fn test_fj242_full_transfer_script_missing_source() {
    let result = full_transfer_script("/opt/target", "/nonexistent/source", None, None, None);
    assert!(result.is_err());
}

#[test]
fn test_fj242_full_transfer_script_valid() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source.bin");
    std::fs::write(&source, b"test data for transfer").unwrap();
    let script = full_transfer_script(
        "/opt/target",
        source.to_str().unwrap(),
        Some("root"),
        Some("root"),
        Some("0644"),
    )
    .unwrap();
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("base64 -d"));
    assert!(script.contains("/opt/target"));
    assert!(script.contains("chown 'root:root'"));
    assert!(script.contains("chmod '0644'"));
}

#[test]
fn test_fj242_roundtrip_delta_reconstruction() {
    // Simulate: old file on remote, new file locally, verify delta correctness
    let mut old_data = vec![0u8; BLOCK_SIZE * 10]; // 10 blocks
                                                   // Make blocks unique
    for i in 0..10 {
        old_data[i * BLOCK_SIZE] = i as u8;
    }

    let remote_sigs = compute_signatures(&old_data);

    // New data: change blocks 3 and 7
    let mut new_data = old_data.clone();
    new_data[3 * BLOCK_SIZE] = 0xFF;
    new_data[7 * BLOCK_SIZE] = 0xFE;

    let delta = compute_delta(&new_data, &remote_sigs);
    assert_eq!(delta.len(), 10);
    assert_eq!(literal_count(&delta), 2);
    assert_eq!(literal_bytes(&delta), BLOCK_SIZE * 2);

    // Verify Copy blocks reference correct indices
    for (i, op) in delta.iter().enumerate() {
        match op {
            DeltaOp::Copy { index } => {
                assert_eq!(*index, i);
                assert_ne!(i, 3);
                assert_ne!(i, 7);
            }
            DeltaOp::Literal { data } => {
                assert_eq!(data.len(), BLOCK_SIZE);
                assert!(i == 3 || i == 7);
            }
        }
    }
}

#[test]
fn test_fj242_empty_data() {
    let sigs = compute_signatures(&[]);
    assert!(sigs.is_empty());
    let delta = compute_delta(&[], &sigs);
    assert!(delta.is_empty());
}

#[test]
fn test_fj242_size_threshold_constant() {
    assert_eq!(SIZE_THRESHOLD, 1_048_576);
    assert_eq!(BLOCK_SIZE, 4096);
}
