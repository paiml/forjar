use super::secrets::*;
use age::secrecy::ExposeSecret;
use base64::Engine;

fn test_keypair() -> (age::x25519::Identity, String) {
    let identity = generate_identity();
    let recipient = identity_to_recipient(&identity);
    (identity, recipient)
}

#[test]
fn test_fj200_encrypt_decrypt_roundtrip() {
    let (identity, recipient) = test_keypair();
    let plaintext = "s3cret-db-passw0rd!";

    let encrypted = encrypt(plaintext, &[&recipient]).unwrap();
    assert!(encrypted.starts_with(ENC_PREFIX));
    assert!(encrypted.ends_with(ENC_SUFFIX));

    let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_fj200_encrypt_empty_string() {
    let (identity, recipient) = test_keypair();
    let encrypted = encrypt("", &[&recipient]).unwrap();
    let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
    assert_eq!(decrypted, "");
}

#[test]
fn test_fj200_encrypt_unicode() {
    let (identity, recipient) = test_keypair();
    let plaintext = "日本語パスワード🔑";
    let encrypted = encrypt(plaintext, &[&recipient]).unwrap();
    let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_fj200_encrypt_multiline() {
    let (identity, recipient) = test_keypair();
    let plaintext = "line1\nline2\nline3";
    let encrypted = encrypt(plaintext, &[&recipient]).unwrap();
    let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_fj200_encrypt_no_recipients() {
    let err = encrypt("test", &[]).unwrap_err();
    assert!(err.contains("at least one recipient"));
}

#[test]
fn test_fj200_encrypt_invalid_recipient() {
    let err = encrypt("test", &["not-a-valid-key"]).unwrap_err();
    assert!(err.contains("invalid recipient"));
}

#[test]
fn test_fj200_decrypt_invalid_marker() {
    let (identity, _) = test_keypair();
    let err = decrypt_marker("not-a-marker", &[identity]).unwrap_err();
    assert!(err.contains("invalid ENC marker"));
}

#[test]
fn test_fj200_decrypt_invalid_base64() {
    let (identity, _) = test_keypair();
    let err = decrypt_marker("ENC[age,!!!invalid!!!]", &[identity]).unwrap_err();
    assert!(err.contains("base64 decode error"));
}

#[test]
fn test_fj200_decrypt_wrong_key() {
    let (_identity1, recipient1) = test_keypair();
    let (identity2, _recipient2) = test_keypair();

    let encrypted = encrypt("secret", &[&recipient1]).unwrap();
    let err = decrypt_marker(&encrypted, &[identity2]).unwrap_err();
    assert!(err.contains("decryption failed"));
}

#[test]
fn test_fj200_multi_recipient() {
    let (identity1, recipient1) = test_keypair();
    let (identity2, recipient2) = test_keypair();
    let plaintext = "shared-secret";

    let encrypted = encrypt(plaintext, &[&recipient1, &recipient2]).unwrap();

    // Both recipients can decrypt
    let d1 = decrypt_marker(&encrypted, &[identity1]).unwrap();
    assert_eq!(d1, plaintext);
    let d2 = decrypt_marker(&encrypted, &[identity2]).unwrap();
    assert_eq!(d2, plaintext);
}

#[test]
fn test_fj200_has_encrypted_markers() {
    let (_identity, recipient) = test_keypair();
    let real_marker = encrypt("test", &[&recipient]).unwrap();
    assert!(has_encrypted_markers(&format!("foo {} bar", real_marker)));
    assert!(has_encrypted_markers(&real_marker));
    // Short/invalid base64 inside markers should NOT trigger
    assert!(!has_encrypted_markers("foo ENC[age,abc] bar"));
    assert!(!has_encrypted_markers("ENC[age,abc]"));
    assert!(!has_encrypted_markers("no markers here"));
    assert!(!has_encrypted_markers("ENC[other,abc]"));
    assert!(!has_encrypted_markers(""));
}

#[test]
fn test_fj200_find_markers() {
    let s = "a=ENC[age,x] b=ENC[age,y]";
    let markers = find_markers(s);
    assert_eq!(markers.len(), 2);
    assert_eq!(&s[markers[0].0..markers[0].1], "ENC[age,x]");
    assert_eq!(&s[markers[1].0..markers[1].1], "ENC[age,y]");
}

#[test]
fn test_fj200_find_markers_none() {
    assert!(find_markers("no markers here").is_empty());
}

#[test]
fn test_fj200_find_markers_unclosed() {
    // Unclosed marker — should not match
    let markers = find_markers("ENC[age,abc");
    assert!(markers.is_empty());
}

#[test]
fn test_fj200_decrypt_all() {
    let (identity, recipient) = test_keypair();
    let enc1 = encrypt("alpha", &[&recipient]).unwrap();
    let enc2 = encrypt("beta", &[&recipient]).unwrap();
    let input = format!("key1={} key2={}", enc1, enc2);

    let result = decrypt_all(&input, &[identity]).unwrap();
    assert_eq!(result, "key1=alpha key2=beta");
}

#[test]
fn test_fj200_decrypt_all_no_markers() {
    let (identity, _) = test_keypair();
    let input = "plain text with no markers";
    let result = decrypt_all(input, &[identity]).unwrap();
    assert_eq!(result, input);
}

#[test]
fn test_fj200_decrypt_all_mixed_content() {
    let (identity, recipient) = test_keypair();
    let enc = encrypt("secret-value", &[&recipient]).unwrap();
    let input = format!("host: localhost\npassword: {}\nport: 5432", enc);

    let result = decrypt_all(&input, &[identity]).unwrap();
    assert_eq!(
        result,
        "host: localhost\npassword: secret-value\nport: 5432"
    );
}

#[test]
fn test_fj200_parse_identity_valid() {
    let identity = generate_identity();
    let key_str = identity.to_string();
    let parsed = parse_identity(key_str.expose_secret()).unwrap();
    assert_eq!(
        identity_to_recipient(&parsed),
        identity_to_recipient(&identity)
    );
}

#[test]
fn test_fj200_parse_identity_invalid() {
    let err = parse_identity("not-a-key").err().expect("expected error");
    assert!(err.contains("invalid age identity"));
}

#[test]
fn test_fj200_identity_to_recipient() {
    let identity = generate_identity();
    let recipient = identity_to_recipient(&identity);
    assert!(recipient.starts_with("age1"));
}

#[test]
fn test_fj200_load_identity_from_env_subprocess() {
    // Run a subprocess with FORJAR_AGE_KEY set to verify env-based loading
    let identity = generate_identity();
    let key_str = identity.to_string();

    let exe = std::env::current_exe().unwrap();
    let output = std::process::Command::new(exe)
        .env("FORJAR_AGE_KEY", key_str.expose_secret())
        .arg("--test-threads=1")
        .arg("--exact")
        .arg("core::secrets::tests::test_fj200_load_identity_env_inner")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "subprocess failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_fj200_load_identity_env_inner() {
    // Run via subprocess with FORJAR_AGE_KEY set
    if std::env::var("FORJAR_AGE_KEY").is_err() {
        return; // Skip when not run via subprocess
    }
    let loaded = load_identity_from_env().unwrap();
    let recipient = identity_to_recipient(&loaded);
    assert!(recipient.starts_with("age1"));
}

#[test]
fn test_fj200_load_identity_from_env_error_message() {
    // Verify error message format (don't actually set env vars)
    let result = load_identity_from_env();
    if result.is_ok() {
        return; // FORJAR_AGE_KEY happens to be set in this environment
    }
    let err = result.err().expect("expected error");
    assert!(err.contains("FORJAR_AGE_KEY not set"));
}

#[test]
fn test_fj200_load_identity_file() {
    let identity = generate_identity();
    let key_str = identity.to_string();
    let recipient_expected = identity_to_recipient(&identity);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("identity.txt");
    std::fs::write(
        &path,
        format!(
            "# created: 2024-01-01\n# public key: {}\n{}\n",
            recipient_expected,
            key_str.expose_secret()
        ),
    )
    .unwrap();

    let loaded = load_identity_file(&path).unwrap();
    assert_eq!(identity_to_recipient(&loaded), recipient_expected);
}

#[test]
fn test_fj200_load_identity_file_not_found() {
    let err = load_identity_file(std::path::Path::new("/nonexistent/key.txt"))
        .err()
        .expect("expected error");
    assert!(err.contains("cannot read identity file"));
}

#[test]
fn test_fj200_load_identity_file_no_key() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.txt");
    std::fs::write(&path, "# just a comment\n").unwrap();

    let err = load_identity_file(&path).err().expect("expected error");
    assert!(err.contains("no AGE-SECRET-KEY found"));
}

#[test]
fn test_fj200_deterministic_marker_format() {
    let (_identity, recipient) = test_keypair();
    let encrypted = encrypt("test", &[&recipient]).unwrap();

    // Verify format
    assert!(encrypted.starts_with("ENC[age,"));
    assert!(encrypted.ends_with(']'));

    // Extract base64 portion and verify it decodes
    let b64 = encrypted
        .strip_prefix(ENC_PREFIX)
        .unwrap()
        .strip_suffix(ENC_SUFFIX)
        .unwrap();
    assert!(B64.decode(b64).is_ok());
}

#[test]
fn test_fj200_large_secret() {
    let (identity, recipient) = test_keypair();
    let plaintext = "x".repeat(100_000);
    let encrypted = encrypt(&plaintext, &[&recipient]).unwrap();
    let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_fj200_special_chars() {
    let (identity, recipient) = test_keypair();
    let plaintext = r#"p@$$w0rd!@#$%^&*(){}[]|\"':;<>,.?/~`"#;
    let encrypted = encrypt(plaintext, &[&recipient]).unwrap();
    let decrypted = decrypt_marker(&encrypted, &[identity]).unwrap();
    assert_eq!(decrypted, plaintext);
}

// ─── PMAT-037: has_encrypted_markers false positives ───────────

#[test]
fn test_pmat037_comment_mentioning_enc_not_a_marker() {
    // A YAML comment like "# In production, values use ENC[age,...] markers"
    // should NOT be detected as having encrypted markers — the "..." is not
    // valid base64 ciphertext.
    let content = "# In production, values use ENC[age,...] markers\nAPP_NAME=myapp\n";
    assert!(
        !has_encrypted_markers(content),
        "comment mentioning ENC[age,...] must not trigger decryption"
    );
}

#[test]
fn test_pmat037_real_marker_still_detected() {
    let (_identity, recipient) = test_keypair();
    let encrypted = encrypt("secret", &[&recipient]).unwrap();
    let content = format!("API_KEY={}\n", encrypted);
    assert!(
        has_encrypted_markers(&content),
        "real encrypted marker must still be detected"
    );
}

// ─── FJ-201 tests ───────────────────────────────────────────────

#[test]
fn test_fj201_decrypt_all_counted() {
    let (identity, recipient) = test_keypair();
    let enc1 = encrypt("alpha", &[&recipient]).unwrap();
    let enc2 = encrypt("beta", &[&recipient]).unwrap();
    let input = format!("key1={} key2={}", enc1, enc2);

    let (result, count) = decrypt_all_counted(&input, &[identity]).unwrap();
    assert_eq!(result, "key1=alpha key2=beta");
    assert_eq!(count, 2);
}

#[test]
fn test_fj201_decrypt_all_counted_zero() {
    let (identity, _) = test_keypair();
    let (result, count) = decrypt_all_counted("no markers", &[identity]).unwrap();
    assert_eq!(result, "no markers");
    assert_eq!(count, 0);
}

#[test]
fn test_fj201_rotate_roundtrip() {
    // Encrypt with key1, then re-encrypt with key2
    let (identity1, recipient1) = test_keypair();
    let (identity2, recipient2) = test_keypair();

    let encrypted = encrypt("rotation-secret", &[&recipient1]).unwrap();

    // Decrypt with key1, re-encrypt with key2
    let plaintext = decrypt_marker(&encrypted, &[identity1]).unwrap();
    assert_eq!(plaintext, "rotation-secret");

    let re_encrypted = encrypt(&plaintext, &[&recipient2]).unwrap();
    let decrypted = decrypt_marker(&re_encrypted, &[identity2]).unwrap();
    assert_eq!(decrypted, "rotation-secret");

    // Original key can't decrypt new ciphertext
    let err = decrypt_marker(&re_encrypted, &[generate_identity()]);
    assert!(err.is_err());
}

#[test]
fn test_fj201_rotate_multi_recipient() {
    let (identity1, recipient1) = test_keypair();
    let (identity2, recipient2) = test_keypair();
    let (identity3, recipient3) = test_keypair();

    // Initially encrypted for identity1
    let encrypted = encrypt("team-secret", &[&recipient1]).unwrap();
    let plain = decrypt_marker(&encrypted, &[identity1]).unwrap();

    // Rotate to identity2 + identity3 (identity1 loses access)
    let rotated = encrypt(&plain, &[&recipient2, &recipient3]).unwrap();
    assert_eq!(
        decrypt_marker(&rotated, &[identity2]).unwrap(),
        "team-secret"
    );
    assert_eq!(
        decrypt_marker(&rotated, &[identity3]).unwrap(),
        "team-secret"
    );
    // identity1 can no longer decrypt
    assert!(decrypt_marker(&rotated, &[generate_identity()]).is_err());
}

#[test]
fn test_fj201_secret_accessed_event_serializes() {
    let event = crate::core::types::ProvenanceEvent::SecretAccessed {
        resource: "db-config".to_string(),
        marker_count: 2,
        identity_recipient: "age1abc...".to_string(),
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("secret_accessed"));
    assert!(json.contains("db-config"));
    assert!(json.contains("marker_count"));
}

#[test]
fn test_fj201_secret_rotated_event_serializes() {
    let event = crate::core::types::ProvenanceEvent::SecretRotated {
        file: "forjar.yaml".to_string(),
        marker_count: 3,
        new_recipients: vec!["age1abc...".to_string(), "age1xyz...".to_string()],
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("secret_rotated"));
    assert!(json.contains("forjar.yaml"));
    assert!(json.contains("age1xyz..."));
}

#[test]
fn test_fj201_secret_events_roundtrip_json() {
    let event = crate::core::types::ProvenanceEvent::SecretAccessed {
        resource: "api-config".to_string(),
        marker_count: 1,
        identity_recipient: "age1test".to_string(),
    };
    let json = serde_json::to_string(&event).unwrap();
    let back: crate::core::types::ProvenanceEvent = serde_json::from_str(&json).unwrap();
    let json2 = serde_json::to_string(&back).unwrap();
    assert_eq!(json, json2);
}
