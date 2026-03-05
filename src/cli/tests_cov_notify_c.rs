//! Coverage tests: dispatch_notify channels, secrets, check, doctor.
use super::check::*;
use super::doctor::*;
#[cfg(feature = "encryption")]
use super::secrets::*;
#[cfg(feature = "encryption")]
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    // secrets.rs
    #[cfg(feature = "encryption")]
    #[test]
    fn test_find_enc_markers() {
        assert!(find_enc_markers("").is_empty());
        assert!(find_enc_markers("no markers").is_empty());
        assert_eq!(find_enc_markers("ENC[age,abc]").len(), 1);
        assert_eq!(find_enc_markers("a: ENC[age,x] b: ENC[age,y]").len(), 2);
        assert!(find_enc_markers("ENC[age,no_closing").is_empty());
        assert_eq!(find_enc_markers("ENC[age,a]ENC[age,b]").len(), 2);
    }
    #[cfg(feature = "encryption")]
    #[test]
    fn test_secrets_keygen() {
        assert!(cmd_secrets_keygen().is_ok());
    }
    #[cfg(feature = "encryption")]
    #[test]
    fn test_secrets_view() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("p.yaml");
        std::fs::write(&f, "k: v\n").unwrap();
        assert!(cmd_secrets_view(&f, None).is_ok());
        assert!(cmd_secrets_view(Path::new("/tmp/ne-12345.yaml"), None).is_err());
        let f2 = d.path().join("e.yaml");
        std::fs::write(&f2, "pw: ENC[age,fake]\n").unwrap();
        let _ = cmd_secrets_view(&f2, None);
    }
    #[cfg(feature = "encryption")]
    #[test]
    fn test_secrets_encrypt() {
        assert!(cmd_secrets_encrypt("val", &["bad".into()]).is_err());
        assert!(cmd_secrets_encrypt("val", &[]).is_err());
    }
    #[cfg(feature = "encryption")]
    #[test]
    fn test_secrets_decrypt() {
        assert!(cmd_secrets_decrypt("not_valid", None).is_err());
        assert!(cmd_secrets_decrypt("ENC[age,!!!]", None).is_err());
    }
    #[cfg(feature = "encryption")]
    #[test]
    fn test_secrets_rekey() {
        assert!(cmd_secrets_rekey(Path::new("/tmp/ne-12345.yaml"), None, &["a".into()]).is_err());
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("p.yaml");
        std::fs::write(&f, "k: v\n").unwrap();
        assert!(cmd_secrets_rekey(&f, None, &["a".into()]).is_ok());
    }
    #[cfg(feature = "encryption")]
    #[test]
    fn test_secrets_rotate() {
        let d = tempfile::tempdir().unwrap();
        let sd = d.path().join("state");
        let f = d.path().join("r.yaml");
        std::fs::write(&f, "v: ENC[age,d]\n").unwrap();
        assert!(cmd_secrets_rotate(&f, None, &["a".into()], false, &sd).is_err());
        assert!(cmd_secrets_rotate(
            Path::new("/tmp/ne-12345.yaml"),
            None,
            &["a".into()],
            true,
            &sd
        )
        .is_err());
        let f2 = d.path().join("p.yaml");
        std::fs::write(&f2, "k: v\n").unwrap();
        assert!(cmd_secrets_rotate(&f2, None, &["a".into()], true, &sd).is_ok());
    }

    // check.rs
    fn check_cfg(d: &std::path::Path) -> std::path::PathBuf {
        let f = d.join("forjar.yaml");
        std::fs::write(
            &f,
            r#"version: "1.0"
name: ck
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  pkg1:
    type: package
    machine: local
    name: coreutils
    tags: [base, system]
  cfg1:
    type: file
    machine: local
    path: /tmp/forjar-ck-test.txt
    content: "hello"
    tags: [config]
"#,
        )
        .unwrap();
        f
    }
    #[test]
    fn test_check_filters() {
        let d = tempfile::tempdir().unwrap();
        let c = check_cfg(d.path());
        let _ = cmd_check(&c, None, None, Some("config"), false, false);
        let _ = cmd_check(&c, None, None, Some("config"), true, false);
        let _ = cmd_check(&c, None, Some("pkg1"), None, false, false);
        let _ = cmd_check(&c, None, Some("nonexistent"), None, false, false);
        let _ = cmd_check(&c, Some("local"), None, None, false, false);
        let _ = cmd_check(&c, Some("nonexistent"), None, None, false, false);
        let _ = cmd_check(&c, None, None, None, false, true);
        let _ = cmd_check(&c, None, None, None, true, false);
        let _ = cmd_check(&c, Some("local"), Some("pkg1"), Some("base"), false, false);
    }
    #[test]
    fn test_check_invalid() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("bad.yaml");
        std::fs::write(&f, "invalid: [[[").unwrap();
        assert!(cmd_check(&f, None, None, None, false, false).is_err());
    }
    #[test]
    fn test_cmd_test_variants() {
        let d = tempfile::tempdir().unwrap();
        let c = check_cfg(d.path());
        let _ = cmd_test(&c, None, None, None, None, false, false);
        let _ = cmd_test(&c, None, None, None, None, true, false);
        let _ = cmd_test(&c, None, None, None, None, false, true);
        let _ = cmd_test(&c, None, None, Some("base"), None, false, false);
        let _ = cmd_test(&c, None, Some("cfg1"), None, None, false, false);
        let _ = cmd_test(&c, Some("local"), None, None, None, false, false);
        let _ = cmd_test(&c, None, None, None, Some("web"), false, false);
        let _ = cmd_test(&c, None, None, None, Some("web"), true, false);
        let _ = cmd_test(&c, None, None, None, None, true, true);
    }
    #[test]
    fn test_cmd_test_invalid() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("bad.yaml");
        std::fs::write(&f, "invalid: [[[").unwrap();
        assert!(cmd_test(&f, None, None, None, None, false, false).is_err());
    }

    // doctor.rs
    #[test]
    fn test_doctor_fix() {
        assert!(cmd_doctor(None, false, true).is_ok());
        assert!(cmd_doctor(None, true, true).is_ok());
    }
    #[test]
    fn test_doctor_enc_markers() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(
            &f,
            r#"version: "1.0"
name: enc
machines:
  local: { hostname: localhost, addr: 127.0.0.1 }
resources:
  s: { type: file, machine: local, path: /tmp/s.txt, content: "ENC[age,fake]" }
"#,
        )
        .unwrap();
        let _ = cmd_doctor(Some(&f), false, false);
        let _ = cmd_doctor(Some(&f), true, false);
    }
    #[test]
    fn test_doctor_container() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(&f, r#"version: "1.0"
name: ctr
machines:
  ctr: { hostname: ctr, addr: container, transport: container, container: { image: alpine:3.18, runtime: podman } }
resources:
  f: { type: file, machine: ctr, path: /tmp/t, content: "x" }
"#).unwrap();
        let _ = cmd_doctor(Some(&f), true, false);
        let f2 = d.path().join("forjar2.yaml");
        std::fs::write(
            &f2,
            r#"version: "1.0"
name: ctr
machines:
  ctr: { hostname: ctr, addr: container, transport: container }
resources:
  f: { type: file, machine: ctr, path: /tmp/t, content: "x" }
"#,
        )
        .unwrap();
        let _ = cmd_doctor(Some(&f2), false, false);
    }
    #[test]
    fn test_doctor_network() {
        let _ = cmd_doctor_network(None, false);
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(
            &f,
            r#"version: "1.0"
name: n
machines:
  lo: { hostname: lo, addr: 127.0.0.1 }
resources:
  f: { type: file, machine: lo, path: /tmp/t, content: "x" }
"#,
        )
        .unwrap();
        assert!(cmd_doctor_network(Some(&f), false).is_ok());
        assert!(cmd_doctor_network(Some(&f), true).is_ok());
    }
    #[test]
    fn test_doctor_network_ssh_key() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(
            &f,
            r#"version: "1.0"
name: n
machines:
  r: { hostname: r, addr: 10.0.0.99, user: deploy, ssh_key: /nonexistent/key.pem }
resources:
  f: { type: file, machine: r, path: /tmp/t, content: "x" }
"#,
        )
        .unwrap();
        assert!(cmd_doctor_network(Some(&f), false).is_ok());
        assert!(cmd_doctor_network(Some(&f), true).is_ok());
    }
    #[test]
    fn test_doctor_network_localhost() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(
            &f,
            r#"version: "1.0"
name: n
machines:
  lo: { hostname: lo, addr: localhost }
resources:
  f: { type: file, machine: lo, path: /tmp/t, content: "x" }
"#,
        )
        .unwrap();
        assert!(cmd_doctor_network(Some(&f), false).is_ok());
    }
    #[test]
    fn test_doctor_network_invalid() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("forjar.yaml");
        std::fs::write(&f, "invalid: [[[").unwrap();
        assert!(cmd_doctor_network(Some(&f), false).is_err());
    }
}
