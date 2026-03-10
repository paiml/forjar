//! Example: State file rekey (FJ-3309)
//!
//! Demonstrates encrypting state files and re-keying them with
//! a new passphrase using age encryption, preserving data integrity throughout.
//!
//! ```bash
//! cargo run --features encryption --example state_rekey
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

    // Encrypt with initial passphrase
    let passphrase1 = "team-password-2024";
    let meta = state_encryption::encrypt_state_file(&lock_file, passphrase1)
        .expect("encryption failed — build with --features encryption");

    println!("\n2. After encryption:");
    println!(
        "  Encrypted: {}",
        state_encryption::is_encrypted(&lock_file)
    );
    println!("  Plaintext hash: {}", meta.plaintext_hash);
    println!("  Ciphertext HMAC: {}...", &meta.ciphertext_hmac[..16]);
    println!("  Version: {}", meta.version);

    // Rekey: decrypt with old passphrase, re-encrypt with new passphrase
    let passphrase2 = "team-password-2025";
    let old_ciphertext = std::fs::read(&lock_file).unwrap();
    let old_meta = state_encryption::read_metadata(&lock_file).unwrap();
    let key1 = state_encryption::derive_key(passphrase1);

    // Verify integrity with old key
    assert!(state_encryption::verify_metadata(
        &old_meta,
        &old_ciphertext,
        &key1
    ));

    // Decrypt with old passphrase
    let decrypted = state_encryption::decrypt_data(&old_ciphertext, passphrase1).unwrap();
    assert_eq!(
        state_encryption::hash_data(&decrypted),
        old_meta.plaintext_hash
    );

    // Re-encrypt with new passphrase
    let new_ciphertext = state_encryption::encrypt_data(&decrypted, passphrase2).unwrap();
    std::fs::write(&lock_file, &new_ciphertext).unwrap();
    let key2 = state_encryption::derive_key(passphrase2);
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

    // Decrypt with new passphrase and verify data
    let final_plain = state_encryption::decrypt_data(&final_ct, passphrase2).unwrap();
    assert_eq!(std::str::from_utf8(&final_plain).unwrap(), original);
    println!("\n5. Full roundtrip:");
    println!("  Data matches original: true");

    println!("\nDone.");
}
