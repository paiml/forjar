//! FJ-3300: Ephemeral values and state integrity example.
//!
//! Demonstrates:
//! 1. Ephemeral value redaction (BLAKE3 hash-and-discard)
//! 2. Drift detection on ephemeral values
//! 3. BLAKE3 keyed hash (HMAC) for encrypted state integrity

fn main() {
    use forjar::core::state::ephemeral;

    println!("=== FJ-3300: Ephemeral Values ===\n");

    // 1. Redact a secret to its BLAKE3 hash
    let secret = "my-database-password-2026";
    let marker = ephemeral::redact_to_hash(secret);
    println!("Original:  {secret}");
    println!("Redacted:  {marker}");
    println!("Is marker: {}\n", ephemeral::is_ephemeral_marker(&marker));

    // 2. Drift detection
    println!("--- Drift Detection ---");
    let same = ephemeral::verify_drift(secret, &marker);
    println!("Same secret:    drift={}", !same);

    let changed = ephemeral::verify_drift("new-password-2027", &marker);
    println!("Changed secret: drift={}\n", !changed);

    // 3. Redact an output map
    println!("--- Output Redaction ---");
    let mut outputs = indexmap::IndexMap::new();
    outputs.insert("data_dir".to_string(), "/var/data".to_string());
    outputs.insert("db_password".to_string(), "s3cret!".to_string());
    outputs.insert("api_token".to_string(), "tok-abc123".to_string());
    outputs.insert("app_port".to_string(), "8080".to_string());

    // Heuristic mode: only secret-looking keys are redacted
    let heuristic = ephemeral::redact_outputs(&outputs, false);
    println!("Heuristic redaction:");
    for (k, v) in &heuristic {
        let redacted = if ephemeral::is_ephemeral_marker(v) {
            "REDACTED"
        } else {
            "cleartext"
        };
        println!("  {k}: [{redacted}] {}", &v[..v.len().min(40)]);
    }

    // Force-all mode: everything is redacted (secrets.ephemeral: true)
    println!("\nForce-all redaction:");
    let forced = ephemeral::redact_outputs(&outputs, true);
    for (k, v) in &forced {
        println!("  {k}: {}", &v[..v.len().min(40)]);
    }

    // 4. BLAKE3 keyed hash for encrypted state integrity
    println!("\n--- Encrypted State Integrity ---");
    let passphrase = "my-encryption-passphrase";
    let key = ephemeral::derive_key(passphrase);
    let state_data = b"encrypted state ciphertext bytes here";

    let hmac = ephemeral::keyed_hash(state_data, &key);
    println!("HMAC:     {hmac}");
    println!(
        "Verify:   {}",
        ephemeral::verify_keyed_hash(state_data, &key, &hmac)
    );
    println!(
        "Tampered: {}",
        ephemeral::verify_keyed_hash(b"tampered", &key, &hmac)
    );
}
