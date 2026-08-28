//! GH-228: the agent configuration is the transport's correctness, not a detail.
//!
//! Three settings on `agent_for_url` carry behaviour that used to be argv flags
//! or curl defaults. Each is asserted here because each has a default that is
//! wrong for a registry client, and a silent revert to any of them reopens a
//! shipped defect.

use super::registry_http::{agent_for_url, authority, is_success, CONNECT_TIMEOUT};

#[test]
fn the_upload_post_never_follows_a_redirect() {
    // THE #210 GATE. ureq follows up to 10 redirects by default. `docker.io`
    // answers the upload POST with a 301 to its marketing site; following it
    // yields a 2xx from a web page, which the 202-check then reads as an upload
    // session. tests/falsification_registry_push_needs_no_curl.rs proves the
    // end-to-end consequence; this pins the setting that prevents it.
    let agent = agent_for_url("https://registry-1.docker.io/v2/a/blobs/uploads/");
    assert_eq!(agent.config().max_redirects(), 0);
}

#[test]
fn a_4xx_arrives_as_a_response_not_as_a_discarded_error() {
    // ureq's default turns a 4xx into `Err` with the body dropped. The body is
    // the registry's own diagnostic — the thing `--fail-with-body` was added
    // for in #154 — so the caller must be the one that decides what a status
    // means.
    let agent = agent_for_url("https://ghcr.io/v2/a/manifests/v1");
    assert!(!agent.config().http_status_as_error());
}

#[test]
fn only_the_connect_phase_is_bounded() {
    // A 64 MB layer over a slow link must not be killed by a clock, but an
    // unroutable registry must not hang either.
    let agent = agent_for_url("https://ghcr.io/v2/a/blobs/uploads/");
    let timeouts = agent.config().timeouts();
    assert_eq!(timeouts.connect, Some(CONNECT_TIMEOUT));
    assert_eq!(timeouts.global, None);
}

#[test]
fn a_loopback_registry_is_never_proxied() {
    // curl honored an ambient HTTP(S)_PROXY even for 127.0.0.1, with no way to
    // say otherwise short of mutating process-global env — which is why all
    // four in-process-registry tests were `#[ignore]`d. The proxy decision is
    // per agent now.
    let agent = agent_for_url("http://127.0.0.1:5000/v2/team/app/blobs/uploads/");
    assert!(agent.config().proxy().is_none());
}

#[test]
fn the_authority_is_extracted_from_any_shape_of_url() {
    assert_eq!(
        authority("http://127.0.0.1:5000/v2/a/blobs/uploads/"),
        "127.0.0.1:5000"
    );
    assert_eq!(
        authority("https://ghcr.io/v2/a?digest=sha256:ab"),
        "ghcr.io"
    );
    assert_eq!(authority("https://localhost"), "localhost");
    // A session URL that is not absolute still has to yield something usable.
    assert_eq!(authority("/v2/a/blobs/uploads/u1"), "");
}

#[test]
fn only_2xx_is_acceptance() {
    assert!(is_success(200));
    assert!(is_success(201));
    assert!(is_success(202));
    assert!(
        !is_success(301),
        "a redirect is not an accepted upload — Refs #210"
    );
    assert!(!is_success(401));
    assert!(!is_success(500), "Refs #154: a 500 stored nothing");
}
