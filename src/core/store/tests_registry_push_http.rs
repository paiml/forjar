//! Tests for push HTTP status handling (Refs #210).
//!
//! These cover the parsing the push now gates on. RED on the pre-fix code:
//! there was no status parsing at all — the presence of a `Location:` header
//! was the entire success criterion, which is why a 301 from `docker.io` to
//! its marketing site was accepted as an upload session.

use super::registry_push_http::{
    describe_status, header_value, parse_status_code, registry_url, resolve_location,
    with_digest_query,
};

#[test]
fn parses_status_from_http1_and_http2() {
    assert_eq!(
        parse_status_code("HTTP/1.1 202 Accepted\r\nLocation: /v2/x\r\n\r\n"),
        Some(202)
    );
    assert_eq!(parse_status_code("HTTP/2 401 \r\n\r\n"), Some(401));
    assert_eq!(parse_status_code("HTTP/2 301\r\n"), Some(301));
}

#[test]
fn continue_preamble_does_not_mask_the_real_status() {
    let headers = "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 401 Unauthorized\r\n\r\n";
    assert_eq!(parse_status_code(headers), Some(401));
}

#[test]
fn no_response_has_no_status() {
    assert_eq!(parse_status_code(""), None);
    assert_eq!(parse_status_code("curl: (7) Failed to connect"), None);
}

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
fn header_lookup_is_case_insensitive() {
    let headers = "HTTP/2 200\r\nDocker-Content-Digest: sha256:abc\r\n\r\n";
    assert_eq!(
        header_value(headers, "docker-content-digest"),
        Some("sha256:abc".to_string())
    );
    assert_eq!(header_value(headers, "etag"), None);
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
