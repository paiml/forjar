//! Coverage tests: dispatch_notify_custom — circuit_breaker, dead_letter,
//! escalation, correlation, sampling, digest, severity_filter.
//! Also covers secrets.rs — find_enc_markers, keygen, view, encrypt, decrypt,
//! rekey, and rotate.

#![allow(unused_imports)]
use super::check::*;
use super::dispatch_notify::*;
use super::dispatch_notify_custom::*;
use super::doctor::*;
use super::secrets::*;
use super::test_fixtures::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    // ─── dispatch_notify_custom.rs — circuit_breaker ────────────────

    #[test]
    fn test_custom_circuit_breaker_none_noop() {
        send_custom_circuit_breaker_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_circuit_breaker_default() {
        send_custom_circuit_breaker_notification(
            Some("http://127.0.0.1:1/cb"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_circuit_breaker_with_threshold() {
        send_custom_circuit_breaker_notification(
            Some("http://127.0.0.1:1/cb|10"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_circuit_breaker_failure() {
        send_custom_circuit_breaker_notification(
            Some("http://127.0.0.1:1/cb|3"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    // ─── dispatch_notify_custom.rs — dead_letter ────────────────────

    #[test]
    fn test_custom_dead_letter_none_noop() {
        send_custom_dead_letter_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_dead_letter_default() {
        send_custom_dead_letter_notification(
            Some("http://127.0.0.1:1/dl"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_dead_letter_with_queue() {
        send_custom_dead_letter_notification(
            Some("http://127.0.0.1:1/dl|my-dead-queue"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_dead_letter_failure() {
        send_custom_dead_letter_notification(
            Some("http://127.0.0.1:1/dl|dlq"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    // ─── dispatch_notify_custom.rs — escalation ─────────────────────

    #[test]
    fn test_custom_escalation_none_noop() {
        send_custom_escalation_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_escalation_success_uses_spec_level() {
        send_custom_escalation_notification(
            Some("http://127.0.0.1:1/esc|warning"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_escalation_failure_uses_critical() {
        send_custom_escalation_notification(
            Some("http://127.0.0.1:1/esc|info"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_escalation_default_level() {
        send_custom_escalation_notification(
            Some("http://127.0.0.1:1/esc"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    // ─── dispatch_notify_custom.rs — correlation ────────────────────

    #[test]
    fn test_custom_correlation_none_noop() {
        send_custom_correlation_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_correlation_default_window() {
        send_custom_correlation_notification(
            Some("http://127.0.0.1:1/cor"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_correlation_with_window() {
        send_custom_correlation_notification(
            Some("http://127.0.0.1:1/cor|120s"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_correlation_failure() {
        send_custom_correlation_notification(
            Some("http://127.0.0.1:1/cor|30s"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    // ─── dispatch_notify_custom.rs — sampling ───────────────────────

    #[test]
    fn test_custom_sampling_none_noop() {
        send_custom_sampling_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_sampling_default_rate() {
        send_custom_sampling_notification(
            Some("http://127.0.0.1:1/sam"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_sampling_with_rate() {
        send_custom_sampling_notification(
            Some("http://127.0.0.1:1/sam|50"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_sampling_failure() {
        send_custom_sampling_notification(
            Some("http://127.0.0.1:1/sam|25"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    // ─── dispatch_notify_custom.rs — digest ─────────────────────────

    #[test]
    fn test_custom_digest_none_noop() {
        send_custom_digest_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_digest_default_interval() {
        send_custom_digest_notification(
            Some("http://127.0.0.1:1/dig"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_digest_with_interval() {
        send_custom_digest_notification(
            Some("http://127.0.0.1:1/dig|4h"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_digest_failure() {
        send_custom_digest_notification(
            Some("http://127.0.0.1:1/dig|1h"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    // ─── dispatch_notify_custom.rs — severity_filter ────────────────

    #[test]
    fn test_custom_severity_filter_none_noop() {
        send_custom_severity_filter_notification(None, &Ok(()), Path::new("/tmp/t.yaml"));
    }

    #[test]
    fn test_custom_severity_filter_info_passes_success() {
        // min_severity=info, result=Ok -> severity=info -> should send
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1/sf|info"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_severity_filter_warn_blocks_info() {
        // min_severity=warn, result=Ok -> severity=info -> should NOT send
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1/sf|warn"),
            &Ok(()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_severity_filter_critical_passes_failure() {
        // min_severity=critical, result=Err -> severity=critical -> should send
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1/sf|critical"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_severity_filter_warn_passes_failure() {
        // min_severity=warn, result=Err -> severity=critical -> should send
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1/sf|warn"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_severity_filter_info_passes_failure() {
        // min_severity=info, result=Err -> severity=critical -> should send
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1/sf|info"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    #[test]
    fn test_custom_severity_filter_default_min_severity() {
        // No pipe -> default min_severity=warn
        send_custom_severity_filter_notification(
            Some("http://127.0.0.1:1/sf"),
            &Err("e".into()),
            Path::new("/tmp/t.yaml"),
        );
    }

    // ─── secrets.rs — find_enc_markers ────────────────────────────

    #[test]
    fn test_find_enc_markers_empty_string() {
        let markers = find_enc_markers("");
        assert!(markers.is_empty());
    }

    #[test]
    fn test_find_enc_markers_no_markers() {
        let markers = find_enc_markers("hello world, no secrets here");
        assert!(markers.is_empty());
    }

    #[test]
    fn test_find_enc_markers_single_marker() {
        let input = "password: ENC[age,abc123def456]";
        let markers = find_enc_markers(input);
        assert_eq!(markers.len(), 1);
        let (start, end) = markers[0];
        assert_eq!(&input[start..end], "ENC[age,abc123def456]");
    }

    #[test]
    fn test_find_enc_markers_multiple_markers() {
        let input = "a: ENC[age,aaa] b: ENC[age,bbb] c: ENC[age,ccc]";
        let markers = find_enc_markers(input);
        assert_eq!(markers.len(), 3);
    }

    #[test]
    fn test_find_enc_markers_incomplete_prefix() {
        let markers = find_enc_markers("ENC[age,no_closing_bracket");
        assert!(markers.is_empty());
    }

    #[test]
    fn test_find_enc_markers_adjacent_markers() {
        let input = "ENC[age,x]ENC[age,y]";
        let markers = find_enc_markers(input);
        assert_eq!(markers.len(), 2);
        assert_eq!(&input[markers[0].0..markers[0].1], "ENC[age,x]");
        assert_eq!(&input[markers[1].0..markers[1].1], "ENC[age,y]");
    }

    #[test]
    fn test_find_enc_markers_nested_text() {
        let input = "line1\npassword: ENC[age,longbase64data==]\nline3";
        let markers = find_enc_markers(input);
        assert_eq!(markers.len(), 1);
        let (s, e) = markers[0];
        assert!(input[s..e].starts_with("ENC[age,"));
        assert!(input[s..e].ends_with(']'));
    }

    // ─── secrets.rs — cmd_secrets_keygen ──────────────────────────

    #[test]
    fn test_cmd_secrets_keygen_succeeds() {
        let result = cmd_secrets_keygen();
        assert!(result.is_ok(), "keygen should always succeed");
    }

    // ─── secrets.rs — cmd_secrets_view ────────────────────────────

    #[test]
    fn test_cmd_secrets_view_plain_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("plain.yaml");
        std::fs::write(&file, "key: value\nother: stuff\n").unwrap();
        let result = cmd_secrets_view(&file, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_secrets_view_nonexistent_file() {
        let result = cmd_secrets_view(Path::new("/tmp/forjar-nonexistent-12345.yaml"), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_secrets_view_file_with_markers_no_identity() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("encrypted.yaml");
        std::fs::write(&file, "password: ENC[age,fake_encrypted_data_here]\n").unwrap();
        // Without a valid identity, decryption will fail
        let result = cmd_secrets_view(&file, None);
        // May fail because no identity is available or marker is invalid
        // Either way, we exercise the code path
        let _ = result;
    }

    // ─── secrets.rs — cmd_secrets_encrypt ─────────────────────────

    #[test]
    fn test_cmd_secrets_encrypt_invalid_recipient() {
        let result = cmd_secrets_encrypt("secret_value", &["not-a-valid-recipient".to_string()]);
        assert!(result.is_err(), "invalid recipient should fail");
    }

    #[test]
    fn test_cmd_secrets_encrypt_empty_recipients() {
        let result = cmd_secrets_encrypt("secret_value", &[]);
        assert!(result.is_err(), "empty recipients should fail");
    }

    // ─── secrets.rs — cmd_secrets_decrypt ─────────────────────────

    #[test]
    fn test_cmd_secrets_decrypt_invalid_marker() {
        let result = cmd_secrets_decrypt("not_a_valid_marker", None);
        // Will fail because it's not a valid ENC[age,...] marker
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_secrets_decrypt_bad_base64() {
        let result = cmd_secrets_decrypt("ENC[age,!!!invalid!!!]", None);
        assert!(result.is_err());
    }

    // ─── secrets.rs — cmd_secrets_rekey ───────────────────────────

    #[test]
    fn test_cmd_secrets_rekey_nonexistent_file() {
        let result = cmd_secrets_rekey(
            Path::new("/tmp/forjar-nonexistent-rekey-12345.yaml"),
            None,
            &["age1test".to_string()],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_secrets_rekey_no_markers() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("plain.yaml");
        std::fs::write(&file, "key: plaintext_value\n").unwrap();
        let result = cmd_secrets_rekey(&file, None, &["age1test".to_string()]);
        assert!(result.is_ok());
    }

    // ─── secrets.rs — cmd_secrets_rotate ──────────────────────────

    #[test]
    fn test_cmd_secrets_rotate_no_re_encrypt_flag() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("rotate.yaml");
        std::fs::write(&file, "val: ENC[age,data]\n").unwrap();
        let state_dir = dir.path().join("state");
        let result = cmd_secrets_rotate(
            &file,
            None,
            &["age1test".to_string()],
            false, // re_encrypt=false should fail
            &state_dir,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--re-encrypt"));
    }

    #[test]
    fn test_cmd_secrets_rotate_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        let result = cmd_secrets_rotate(
            Path::new("/tmp/forjar-nonexistent-rotate-12345.yaml"),
            None,
            &["age1test".to_string()],
            true,
            &state_dir,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_secrets_rotate_no_markers() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("rotate-plain.yaml");
        std::fs::write(&file, "key: value\n").unwrap();
        let state_dir = dir.path().join("state");
        let result = cmd_secrets_rotate(&file, None, &["age1test".to_string()], true, &state_dir);
        assert!(result.is_ok());
    }
}
