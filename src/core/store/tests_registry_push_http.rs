//! Tests for push HTTP status handling (Refs #210).
//!
//! These cover the URL and status handling the push gates on. RED on the
//! pre-#210 code: there was no status handling at all — the presence of a
//! `Location:` header was the entire success criterion, which is why a 301
//! from `docker.io` to its marketing site was accepted as an upload session.
//!
//! GH-228: the raw-header-dump parsers (`parse_status_code`, `header_value`)
//! their siblings used to test are gone; ureq supplies a typed status and
//! typed headers, so there is no dump to parse.

use super::registry_push_http::{
    describe_status, registry_url, resolve_location, with_digest_query,
};

#[test]
fn location_resolution_handles_absolute_and_relative() {
    assert_eq!(
        resolve_location("ghcr.io", "https://blob.example/upload/1"),
        "https://blob.example/upload/1"
    );
    assert_eq!(
        resolve_location("ghcr.io", "/v2/foo/blobs/uploads/abc"),
        "https://ghcr.io/v2/foo/blobs/uploads/abc"
    );
    assert_eq!(
        resolve_location("ghcr.io", "v2/foo/blobs/uploads/abc"),
        "https://ghcr.io/v2/foo/blobs/uploads/abc"
    );
}

#[test]
fn auth_failure_names_the_missing_capability() {
    for code in [401, 403] {
        let msg = describe_status("POST /v2/app/blobs/uploads/", code);
        assert!(msg.contains("requires authentication"), "{msg}");
        assert!(
            msg.contains("does not implement registry credentials"),
            "the operator must be told forjar cannot do this, not that it was skipped: {msg}"
        );
    }
}

#[test]
fn a_redirect_is_reported_as_the_wrong_host_not_as_a_session() {
    let msg = describe_status("POST https://docker.io/v2/app/blobs/uploads/", 301);
    assert!(msg.contains("redirect"), "{msg}");
    assert!(msg.contains("registry-1.docker.io"), "{msg}");
}

#[test]
fn digest_query_respects_an_existing_query_string() {
    // RED before the fix: `?_state=x?digest=…` — distribution/registry:2
    // answers BLOB_UPLOAD_INVALID, so no monolithic upload ever completed.
    assert_eq!(
        with_digest_query("http://reg/v2/a/blobs/uploads/u1?_state=xyz", "sha256:ab"),
        "http://reg/v2/a/blobs/uploads/u1?_state=xyz&digest=sha256:ab"
    );
    assert_eq!(
        with_digest_query("http://reg/v2/a/blobs/uploads/u1", "sha256:ab"),
        "http://reg/v2/a/blobs/uploads/u1?digest=sha256:ab"
    );
}

#[test]
fn loopback_registries_are_addressed_over_http() {
    assert_eq!(
        registry_url("127.0.0.1:5000", "v2/team/app/tags/list"),
        "http://127.0.0.1:5000/v2/team/app/tags/list"
    );
    assert_eq!(registry_url("localhost", "v2/"), "http://localhost/v2/");
    assert_eq!(registry_url("ghcr.io", "v2/"), "https://ghcr.io/v2/");
    // A host that merely starts with "localhost" is not loopback.
    assert_eq!(
        registry_url("localhost.example.com", "v2/"),
        "https://localhost.example.com/v2/"
    );
}
