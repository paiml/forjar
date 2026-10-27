//! forjar#613: a pinned binary whose live `--version` is not its pin is drift.

use super::version_pin::{
    declared_pin, evaluate, parse_probe, parse_semver, Live, Outcome, Pin, Semver, Upstream,
};
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

#[test]
fn fj613_parse_probe_splits_live_and_upstream() {
    let (live, up) = parse_probe(
        "@@forjar-live\nollama version is 0.34.2\n@@forjar-upstream\n\"tag_name\": \"v0.35.0\"\n",
        true,
    );
    assert_eq!(live, Live::Version(v(0, 34, 2)));
    assert_eq!(up, Upstream::Version(v(0, 35, 0)));

    let (live, up) = parse_probe(
        "@@forjar-missing\n@@forjar-upstream\nerror: curl: (6) Could not resolve host: api.github.com\n",
        true,
    );
    assert_eq!(live, Live::Missing);
    assert!(matches!(up, Upstream::Unmeasured(ref e) if e.contains("Could not resolve host")));

    let (_, up) = parse_probe("@@forjar-live\nx 1.0.0\n@@forjar-upstream\n", true);
    assert!(
        matches!(up, Upstream::Unmeasured(_)),
        "empty answer is not a version"
    );

    let (_, up) = parse_probe("@@forjar-live\nx 1.0.0\n", false);
    assert_eq!(up, Upstream::NotChecked);
}

// ---------------------------------------------------------------------------
// Verdicts ("latest CRUX tooling only").
// ---------------------------------------------------------------------------

#[test]
fn fj613_pin_behind_upstream_is_drift() {
    let out = evaluate(
        "ollama",
        "ollama/ollama",
        &Pin::Version(v(0, 33, 2)),
        &Live::Version(v(0, 33, 2)),
        &Upstream::Version(v(0, 34, 2)),
    );
    assert_eq!(out.len(), 1, "{out:?}");
    assert!(
        matches!(&out[0], Outcome::Drift { expected, actual, .. } if expected == "latest 0.34.2" && actual == "pin 0.33.2"),
        "{out:?}"
    );
}

#[test]
fn fj613_pin_at_upstream_and_live_is_clean() {
    let out = evaluate(
        "ollama",
        "ollama/ollama",
        &Pin::Version(v(0, 34, 2)),
        &Live::Version(v(0, 34, 2)),
        &Upstream::Version(v(0, 34, 2)),
    );
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn fj613_upstream_unanswered_is_unmeasured() {
    let out = evaluate(
        "ollama",
        "ollama/ollama",
        &Pin::Version(v(0, 34, 2)),
        &Live::Version(v(0, 34, 2)),
        &Upstream::Unmeasured("curl: (22) 403".into()),
    );
    assert!(
        matches!(&out[..], [Outcome::Unmeasured(e)] if e.contains("403")),
        "{out:?}"
    );
}

#[test]
fn fj613_tag_latest_compares_live_with_upstream() {
    let out = evaluate(
        "ollama",
        "ollama/ollama",
        &Pin::Latest,
        &Live::Version(v(0, 33, 2)),
        &Upstream::Version(v(0, 34, 2)),
    );
    assert!(
        matches!(&out[..], [Outcome::Drift { detail, .. }] if detail.contains("upgrade")),
        "{out:?}"
    );

    let offline = evaluate(
        "ollama",
        "ollama/ollama",
        &Pin::Latest,
        &Live::Version(v(0, 33, 2)),
        &Upstream::NotChecked,
    );
    assert!(
        matches!(&offline[..], [Outcome::NotChecked(_)]),
        "{offline:?}"
    );
}

#[test]
fn fj613_nightly_is_named_not_passed() {
    let out = evaluate(
        "apr",
        "paiml/aprender",
        &Pin::Unversioned("nightly".into()),
        &Live::Missing,
        &Upstream::NotChecked,
    );
    assert!(
        matches!(&out[..], [Outcome::NotChecked(w)] if w.contains("nightly")),
        "{out:?}"
    );
}
