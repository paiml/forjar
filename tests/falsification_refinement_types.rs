//! FJ-043: Refinement type validation — Port, FileMode, SemVer, Hostname, AbsPath, ResourceName.
//!
//! Popperian rejection criteria for:
//! - FJ-043: Port (valid range, zero rejected, copy semantics)
//! - FJ-043: FileMode (valid, overflow, from_str, octal_string)
//! - FJ-043: SemVer (parse, display, zero version, invalid formats)
//! - FJ-043: Hostname (RFC 1123, length, labels, dashes, invalid chars)
//! - FJ-043: AbsPath (absolute, root, relative rejected, null bytes)
//! - FJ-043: ResourceName (valid, underscore, empty, too long, special chars)
//!
//! Usage: cargo test --test falsification_refinement_types

use forjar::core::types::refinement::{AbsPath, FileMode, Hostname, Port, ResourceName, SemVer};

// ============================================================================
// Port
// ============================================================================

#[test]
fn port_valid_range() {
    let p = Port::new(1).unwrap();
    assert_eq!(p.value(), 1);
    let p = Port::new(80).unwrap();
    assert_eq!(p.value(), 80);
    let p = Port::new(65535).unwrap();
    assert_eq!(p.value(), 65535);
}

#[test]
fn port_zero_rejected() {
    let err = Port::new(0).unwrap_err();
    assert!(err.contains("port"));
    assert!(err.contains("0"));
}

#[test]
fn port_copy_semantics() {
    let p = Port::new(443).unwrap();
    let p2 = p;
    assert_eq!(p.value(), p2.value());
}

// ============================================================================
// FileMode
// ============================================================================

#[test]
fn filemode_valid() {
    assert_eq!(FileMode::new(0o644).unwrap().value(), 0o644);
    assert_eq!(FileMode::new(0o755).unwrap().value(), 0o755);
    assert_eq!(FileMode::new(0o000).unwrap().value(), 0o000);
    assert_eq!(FileMode::new(0o777).unwrap().value(), 0o777);
}

#[test]
fn filemode_overflow_rejected() {
    let err = FileMode::new(0o1000).unwrap_err();
    assert!(err.contains("mode"));
}

#[test]
fn filemode_from_str_valid() {
    assert_eq!(FileMode::from_str("644").unwrap().value(), 0o644);
    assert_eq!(FileMode::from_str("755").unwrap().value(), 0o755);
    assert_eq!(FileMode::from_str("000").unwrap().value(), 0);
}

#[test]
fn filemode_from_str_invalid() {
    assert!(FileMode::from_str("999").is_err());
    assert!(FileMode::from_str("abc").is_err());
    assert!(FileMode::from_str("").is_err());
}

#[test]
fn filemode_octal_string() {
    assert_eq!(FileMode::new(0o644).unwrap().as_octal_string(), "0644");
    assert_eq!(FileMode::new(0o000).unwrap().as_octal_string(), "0000");
    assert_eq!(FileMode::new(0o777).unwrap().as_octal_string(), "0777");
    assert_eq!(FileMode::new(0o100).unwrap().as_octal_string(), "0100");
}

// ============================================================================
// SemVer
// ============================================================================

#[test]
fn semver_parse_valid() {
    let v = SemVer::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
}

#[test]
fn semver_display() {
    let v = SemVer::parse("10.20.30").unwrap();
    assert_eq!(v.to_string(), "10.20.30");
}

#[test]
fn semver_zero_version() {
    let v = SemVer::parse("0.0.0").unwrap();
    assert_eq!(v.major, 0);
    assert_eq!(v.to_string(), "0.0.0");
}

#[test]
fn semver_invalid_too_few_parts() {
    let err = SemVer::parse("1.2").unwrap_err();
    assert!(err.contains("X.Y.Z"));
}

#[test]
fn semver_invalid_too_many_parts() {
    assert!(SemVer::parse("1.2.3.4").is_err());
}

#[test]
fn semver_invalid_non_numeric() {
    assert!(SemVer::parse("a.b.c").is_err());
    assert!(SemVer::parse("1.2.x").is_err());
}

// ============================================================================
// Hostname
// ============================================================================

#[test]
fn hostname_valid_simple() {
    let h = Hostname::new("example.com").unwrap();
    assert_eq!(h.as_str(), "example.com");
}

#[test]
fn hostname_valid_with_dashes() {
    let h = Hostname::new("web-01.example.com").unwrap();
    assert_eq!(h.as_str(), "web-01.example.com");
}

#[test]
fn hostname_valid_single_label() {
    let h = Hostname::new("localhost").unwrap();
    assert_eq!(h.as_str(), "localhost");
}

#[test]
fn hostname_empty_rejected() {
    let err = Hostname::new("").unwrap_err();
    assert!(err.contains("length"));
}

#[test]
fn hostname_too_long_rejected() {
    let long = "a".repeat(254);
    let err = Hostname::new(&long).unwrap_err();
    assert!(err.contains("253"));
}

#[test]
fn hostname_leading_dash_rejected() {
    let err = Hostname::new("-invalid.com").unwrap_err();
    assert!(err.contains("dash"));
}

#[test]
fn hostname_trailing_dash_rejected() {
    let err = Hostname::new("invalid-.com").unwrap_err();
    assert!(err.contains("dash"));
}

#[test]
fn hostname_invalid_chars_rejected() {
    assert!(Hostname::new("under_score.com").is_err());
    assert!(Hostname::new("space here.com").is_err());
}

#[test]
fn hostname_empty_label_rejected() {
    let err = Hostname::new("example..com").unwrap_err();
    assert!(err.contains("label"));
}

#[test]
fn hostname_label_too_long_rejected() {
    let long_label = "a".repeat(64);
    let host = format!("{long_label}.com");
    let err = Hostname::new(&host).unwrap_err();
    assert!(err.contains("63"));
}

// ============================================================================
// AbsPath
// ============================================================================

#[test]
fn abspath_valid() {
    let p = AbsPath::new("/etc/nginx/nginx.conf").unwrap();
    assert_eq!(p.as_str(), "/etc/nginx/nginx.conf");
}

#[test]
fn abspath_root() {
    let p = AbsPath::new("/").unwrap();
    assert_eq!(p.as_str(), "/");
}

#[test]
fn abspath_relative_rejected() {
    let err = AbsPath::new("relative/path").unwrap_err();
    assert!(err.contains("absolute"));
}

#[test]
fn abspath_null_byte_rejected() {
    let err = AbsPath::new("/etc/bad\0path").unwrap_err();
    assert!(err.contains("null"));
}

// ============================================================================
// ResourceName
// ============================================================================

#[test]
fn resource_name_valid() {
    let n = ResourceName::new("pkg-nginx").unwrap();
    assert_eq!(n.as_str(), "pkg-nginx");
}

#[test]
fn resource_name_underscore() {
    let n = ResourceName::new("my_resource_01").unwrap();
    assert_eq!(n.as_str(), "my_resource_01");
}

#[test]
fn resource_name_empty_rejected() {
    let err = ResourceName::new("").unwrap_err();
    assert!(err.contains("length"));
}

#[test]
fn resource_name_too_long_rejected() {
    let long = "a".repeat(129);
    let err = ResourceName::new(&long).unwrap_err();
    assert!(err.contains("128"));
}

#[test]
fn resource_name_spaces_rejected() {
    assert!(ResourceName::new("has space").is_err());
}

#[test]
fn resource_name_special_chars_rejected() {
    assert!(ResourceName::new("bad!name").is_err());
    assert!(ResourceName::new("foo.bar").is_err());
    assert!(ResourceName::new("foo/bar").is_err());
}
