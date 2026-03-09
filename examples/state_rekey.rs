//! Example: State file rekey (FJ-3309)
//!
//! Demonstrates encrypting state files and re-keying them with
//! a new passphrase, preserving data integrity throughout.
//!
//! ```bash
//! cargo run --example state_rekey
//! ```

use forjar::core::state_encryption;

fn main() {
    println!("=== State File Rekey (FJ-3309) ===\n");

    // Create a sample state file
    let dir = tempfile::tempdir().unwrap();
    let lock_file = dir.path().join("m1.lock.yaml");
    let original = "schema: \"1.0\"\nmachine: m1\nresources:\n  nginx:\n    status: converged\n";
    std::fs::write(&lock_file, original).unwrap();

    println!("1. Original state:");
    println!("  File: {}", lock_file.display());
    println!(
        "  Hash: {}",
        state_encryption::hash_data(original.as_bytes())
    );
    println!(
        "  Encrypted: {}",
        state_encryption::is_encrypted(&lock_file)
    );

    // Encrypt with initial key
    let key1 = state_encryption::derive_key("team-password-2024");
    let plaintext = std::fs::read(&lock_file).unwrap();
    let ciphertext = xor_mask(&plaintext, &key1);
    std::fs::write(&lock_file, &ciphertext).unwrap();
    let meta = state_encryption::create_metadata(&plaintext, &ciphertext, &key1);
    state_encryption::write_metadata(&lock_file, &meta).unwrap();

    println!("\n2. After encryption:");
    println!(
        "  Encrypted: {}",
        state_encryption::is_encrypted(&lock_file)
    );
    println!("  Plaintext hash: {}", meta.plaintext_hash);
    println!("  Ciphertext HMAC: {}...", &meta.ciphertext_hmac[..16]);
    println!("  Version: {}", meta.version);

    // Rekey: decrypt with old key, re-encrypt with new key
    let key2 = state_encryption::derive_key("team-password-2025");
    let old_ciphertext = std::fs::read(&lock_file).unwrap();
    let old_meta = state_encryption::read_metadata(&lock_file).unwrap();

    // Verify integrity
    assert!(state_encryption::verify_metadata(
        &old_meta,
        &old_ciphertext,
        &key1
    ));

    // Decrypt
    let decrypted = xor_mask(&old_ciphertext, &key1);
    assert_eq!(
        state_encryption::hash_data(&decrypted),
        old_meta.plaintext_hash
    );

    // Re-encrypt with new key
    let new_ciphertext = xor_mask(&decrypted, &key2);
    std::fs::write(&lock_file, &new_ciphertext).unwrap();
    let new_meta = state_encryption::create_metadata(&decrypted, &new_ciphertext, &key2);
    state_encryption::write_metadata(&lock_file, &new_meta).unwrap();

    println!("\n3. After rekey:");
    println!("  Plaintext hash: {} (same)", new_meta.plaintext_hash);
    println!(
        "  Ciphertext HMAC: {}... (changed)",
        &new_meta.ciphertext_hmac[..16]
    );
    println!(
        "  Plaintext preserved: {}",
        new_meta.plaintext_hash == old_meta.plaintext_hash
    );

    // Verify with new key
    let final_ct = std::fs::read(&lock_file).unwrap();
    assert!(state_encryption::verify_metadata(
        &new_meta, &final_ct, &key2
    ));

    // Cannot verify with old key
    assert!(!state_encryption::verify_metadata(
        &new_meta, &final_ct, &key1
    ));

    println!("\n4. Key verification:");
    println!("  New key verifies: true");
    println!("  Old key verifies: false");

    // Decrypt with new key and verify data
    let final_plain = xor_mask(&final_ct, &key2);
    assert_eq!(std::str::from_utf8(&final_plain).unwrap(), original);
    println!("\n5. Full roundtrip:");
    println!("  Data matches original: true");

    println!("\nDone.");
}

/// XOR mask (same as in state_encrypt.rs)
fn xor_mask(data: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    for (block_idx, chunk) in (0_u64..).zip(data.chunks(32)) {
        let block_key = blake3::keyed_hash(key, &block_idx.to_le_bytes());
        let stream = block_key.as_bytes();
        for (i, &byte) in chunk.iter().enumerate() {
            result.push(byte ^ stream[i]);
        }
    }
    result
}
