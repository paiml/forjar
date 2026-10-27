//! forjar#613: a pinned binary whose live `--version` is not its pin is drift.

use super::version_pin::{declared_pin, parse_semver, Pin, Semver};
use crate::core::types::{Resource, ResourceType};

// ---------------------------------------------------------------------------
// Parsing.
// ---------------------------------------------------------------------------

fn v(major: u64, minor: u64, patch: u64) -> Semver {
    Semver {
        major,
        minor,
        patch,
    }
}

#[test]
fn fj613_parse_semver_reads_the_shapes_in_the_wild() {
    assert_eq!(parse_semver("ollama version is 0.34.2"), Some(v(0, 34, 2)));
    assert_eq!(parse_semver("forjar 1.32.0"), Some(v(1, 32, 0)));
    assert_eq!(parse_semver("v0.34.2"), Some(v(0, 34, 2)));
    assert_eq!(
        parse_semver("rclone v1.68.1\n- os/version: ubuntu 24.04"),
        Some(v(1, 68, 1))
    );
    assert_eq!(
        parse_semver(
            "Warning: could not connect to a running Ollama instance\nWarning: client version is 0.34.2"
        ),
        Some(v(0, 34, 2))
    );
    assert_eq!(parse_semver("tool 2.0.0-rc.1 (abc123)"), Some(v(2, 0, 0)));
}

#[test]
fn fj613_parse_semver_refuses_what_is_not_a_version() {
    assert_eq!(parse_semver(""), None);
    assert_eq!(parse_semver("usage: ollama [command]"), None);
    assert_eq!(parse_semver("ollama version 0.34"), None);
    assert_eq!(parse_semver("listening on 10.42.0.11"), None);
    assert_eq!(parse_semver("abc1.2.3"), None);
}

#[test]
fn fj613_declared_pin_from_tag() {
    let mut r = Resource {
        resource_type: ResourceType::GithubRelease,
        ..Default::default()
    };
    assert_eq!(declared_pin(&r), Pin::Latest);
    r.tag = Some("latest".into());
    assert_eq!(declared_pin(&r), Pin::Latest);
    r.tag = Some("v0.33.2".into());
    assert_eq!(declared_pin(&r), Pin::Version(v(0, 33, 2)));
    r.tag = Some("nightly".into());
    assert_eq!(declared_pin(&r), Pin::Unversioned("nightly".into()));
}
