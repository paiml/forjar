//! Tests for [`crate::core::webhook_sig`].
//!
//! # Why these are known-answer tests
//!
//! The predecessor of this module was documented and named HMAC-SHA256 but
//! computed a keyed BLAKE3 hash, and shipped that way for months with ~6 passing
//! signature tests. Every one of them derived its expected value by calling the
//! function under test:
//!
//! ```ignore
//! let sig = compute_hmac_hex(secret, body);          // f(x)
//! req.headers.insert("x-forjar-signature", sig);
//! assert!(validate_request(&config, &req).is_valid()); // == f(x)
//! ```
//!
//! `f(x) == f(x)` holds for ANY function, so no amount of that could detect the
//! wrong algorithm. `hmac_deterministic` asserted only `h1 == h2` and
//! `h1.len() == 64` — and BLAKE3-256 hex is also 64 chars, so even the length
//! could not discriminate.
//!
//! Every expected value below therefore comes from **outside this crate**: RFC
//! 4231's published vectors, and digests generated with
//! `openssl dgst -sha256 -hmac`. If someone swaps the primitive again, these fail.

use super::webhook_sig::*;

// ── Known-answer vectors (RFC 4231) ──────────────────────────────────────────

/// RFC 4231 test case 1: key = 20×0x0b, data = "Hi There".
#[test]
fn rfc4231_tc1() {
    let key = [0x0bu8; 20];
    assert_eq!(
        compute_hmac_hex(&key, b"Hi There"),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}

/// RFC 4231 test case 2: key = "Jefe".
///
/// This is the vector that exposes the substituted primitive: the old
/// keyed-BLAKE3 implementation returned `30f3b0f1f2b72e19eefe2a08fc3af2bc…`.
/// Independently reproducible:
///   printf 'what do ya want for nothing?' | openssl dgst -sha256 -hmac 'Jefe'
#[test]
fn rfc4231_tc2() {
    assert_eq!(
        compute_hmac_hex(b"Jefe", b"what do ya want for nothing?"),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}

/// RFC 4231 test case 3: key = 20×0xaa, data = 50×0xdd.
#[test]
fn rfc4231_tc3() {
    let key = [0xaau8; 20];
    let data = [0xddu8; 50];
    assert_eq!(
        compute_hmac_hex(&key, &data),
        "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"
    );
}

/// A key longer than the 64-byte block must be hashed first (RFC 4231 TC6).
#[test]
fn rfc4231_tc6_key_longer_than_block() {
    let key = [0xaau8; 131];
    assert_eq!(
        compute_hmac_hex(
            &key,
            b"Test Using Larger Than Block-Size Key - Hash Key First"
        ),
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
    );
}

/// Distinct DATA must give a distinct MAC. The old suite varied only the key,
/// so a function ignoring its data argument entirely would have passed.
#[test]
fn hmac_different_data() {
    let a = compute_hmac_hex(b"key", b"data-one");
    let b = compute_hmac_hex(b"key", b"data-two");
    assert_ne!(a, b);
}

/// Empty key and empty data are still well-defined, not a panic.
#[test]
fn hmac_handles_empty_inputs() {
    // openssl dgst -sha256 -hmac '' </dev/null
    assert_eq!(
        compute_hmac_hex(b"", b""),
        "b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad"
    );
}

// ── Verification ─────────────────────────────────────────────────────────────

/// Verification accepts a signature produced by an INDEPENDENT implementation,
/// not one this module just computed.
#[test]
fn verify_accepts_an_openssl_generated_signature() {
    // printf 'what do ya want for nothing?' | openssl dgst -sha256 -hmac 'Jefe'
    let sig = "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843";
    assert!(verify_hex(b"Jefe", b"what do ya want for nothing?", sig));
}

/// Hex case must not matter — the digest is decoded before comparison. The old
/// `sig != &expected` String compare rejected a correct uppercase signature.
#[test]
fn verify_is_hex_case_insensitive() {
    let upper = "5BDCC146BF60754E6A042426089575C75A003F089D2739839DEC58B964EC3843";
    assert!(verify_hex(b"Jefe", b"what do ya want for nothing?", upper));
}

#[test]
fn verify_rejects_wrong_secret_and_wrong_data() {
    let sig = "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843";
    assert!(!verify_hex(b"wrong", b"what do ya want for nothing?", sig));
    assert!(!verify_hex(b"Jefe", b"tampered", sig));
}

/// Malformed input is a clean `false`, never a panic or a truncated compare.
#[test]
fn verify_rejects_malformed_hex() {
    for bad in [
        "",
        "abc",
        "zz2f679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad",
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec38",
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843ff",
    ] {
        assert!(!verify_hex(b"Jefe", b"data", bad), "accepted {bad:?}");
    }
}

/// The MAC covers raw bytes, including sequences that are not valid UTF-8. The
/// old `&str` signature could not express this: the server had already replaced
/// such bytes with U+FFFD before hashing, so a correctly-signed binary-ish body
/// could never verify.
#[test]
fn verify_covers_non_utf8_bytes() {
    // Assembled at runtime, not written as a literal: clippy const-evaluates
    // `from_utf8` on a byte-array literal and rejects the call outright
    // ("with an invalid literal always return an error"), so the invalid byte has
    // to be pushed in rather than spelled out.
    let mut body: Vec<u8> = br#"{"a":"#.to_vec();
    body.push(0xff);
    body.push(b'}');
    assert!(
        std::str::from_utf8(&body).is_err(),
        "fixture must be non-UTF-8"
    );
    let sig = compute_hmac_hex(b"s3cret", &body);
    assert!(verify_hex(b"s3cret", &body, &sig));

    // And the lossy form must NOT verify, which is what the old code compared.
    let lossy = String::from_utf8_lossy(&body).into_owned();
    assert!(!verify_hex(b"s3cret", lossy.as_bytes(), &sig));
}

// ── Canonical payload ────────────────────────────────────────────────────────

#[test]
fn canonical_payload_is_newline_separated() {
    let p = canonical_payload(1785350000, "POST", "/webhook", b"{\"a\":1}");
    assert_eq!(
        String::from_utf8(p).unwrap(),
        "t=1785350000\nPOST\n/webhook\n{\"a\":1}"
    );
}

/// The whole point of binding the path: a signature minted for one allowed path
/// must not verify at another. With a body-only MAC it did.
#[test]
fn signature_does_not_transfer_between_paths() {
    let secret = b"s3cret";
    let body = b"{\"action\":\"go\"}";
    let t = 1785350000;

    let deploy = compute_hmac_hex(secret, &canonical_payload(t, "POST", "/hooks/deploy", body));
    let destroy_payload = canonical_payload(t, "POST", "/hooks/destroy", body);

    assert!(!verify_hex(secret, &destroy_payload, &deploy));
}

/// Likewise the method, so a signed POST cannot be replayed as another verb.
#[test]
fn signature_does_not_transfer_between_methods() {
    let secret = b"s3cret";
    let body = b"{}";
    let t = 1785350000;
    let post = compute_hmac_hex(secret, &canonical_payload(t, "POST", "/webhook", body));
    let put_payload = canonical_payload(t, "PUT", "/webhook", body);
    assert!(!verify_hex(secret, &put_payload, &post));
}

/// A `.` in the path must not be able to shift the field boundary — the reason
/// for newline separators rather than Stripe's `t.payload`.
#[test]
fn dotted_path_cannot_shift_the_boundary() {
    let a = canonical_payload(1, "POST", "/a.b", b"x");
    let b = canonical_payload(1, "POST", "/a", b".b\nx");
    assert_ne!(a, b);
}

// ── Header parsing ───────────────────────────────────────────────────────────

#[test]
fn parse_signature_header_extracts_t_and_v1() {
    let h = parse_forjar_signature("t=1785350000,v1=deadbeef");
    assert_eq!(h.timestamp, Some(1785350000));
    assert_eq!(h.v1, vec!["deadbeef".to_string()]);
    assert!(h.has_v1());
}

/// Multiple v1 elements support secret rotation without a flag day.
#[test]
fn parse_signature_header_keeps_every_v1() {
    let h = parse_forjar_signature("t=1,v1=aaa,v1=bbb");
    assert_eq!(h.v1, vec!["aaa".to_string(), "bbb".to_string()]);
}

/// Unknown elements are ignored so the scheme can grow, but a header carrying no
/// v1 at all must be visible as such — NOT silently treated as unsigned.
#[test]
fn parse_signature_header_without_v1_is_not_signed() {
    let h = parse_forjar_signature("t=1,v2=future,junk");
    assert!(!h.has_v1());
    assert_eq!(h.timestamp, Some(1));
}

#[test]
fn parse_signature_header_tolerates_whitespace_and_bad_t() {
    let h = parse_forjar_signature(" t = 12 , v1 = abc ");
    assert_eq!(h.timestamp, Some(12));
    assert_eq!(h.v1, vec!["abc".to_string()]);
    assert_eq!(parse_forjar_signature("t=notanumber,v1=a").timestamp, None);
}

#[test]
fn parse_github_signature_strips_the_prefix() {
    assert_eq!(
        parse_github_signature("sha256=abc123").as_deref(),
        Some("abc123")
    );
    assert!(parse_github_signature("sha1=abc123").is_none());
    assert!(parse_github_signature("abc123").is_none());
}

// ── Freshness ────────────────────────────────────────────────────────────────

#[test]
fn timestamp_freshness_window() {
    let now = 1_000_000;
    assert!(timestamp_is_fresh(now, now, 300));
    assert!(timestamp_is_fresh(now - 300, now, 300));
    assert!(!timestamp_is_fresh(now - 301, now, 300));
}

/// A future-dated timestamp is rejected too. Otherwise a sender with a fast
/// clock — or an attacker choosing `t` — could extend a captured request's life.
#[test]
fn timestamp_in_the_future_is_rejected() {
    let now = 1_000_000;
    assert!(timestamp_is_fresh(now + 300, now, 300));
    assert!(!timestamp_is_fresh(now + 301, now, 300));
}

// ── Replay guard ─────────────────────────────────────────────────────────────

#[test]
fn replay_guard_admits_once() {
    let mut g = ReplayGuard::new(300, 16);
    assert!(g.admit("sig-a", 1000));
    assert!(!g.admit("sig-a", 1000), "second admit must be refused");
    assert!(g.admit("sig-b", 1000));
    assert_eq!(g.len(), 2);
}

#[test]
fn replay_guard_expires_outside_the_window() {
    let mut g = ReplayGuard::new(300, 16);
    assert!(g.admit("sig-a", 1000));
    // 400s later the entry is outside the 300s window and is dropped, so the
    // same digest is admissible again — by then the freshness check rejects it
    // anyway, which is why persistence buys nothing.
    assert!(g.admit("sig-b", 1400));
    assert!(!g.seen_contains("sig-a"));
}

/// Bounded: an attacker sending distinct signatures must not grow memory without
/// limit.
#[test]
fn replay_guard_is_bounded() {
    let mut g = ReplayGuard::new(300, 8);
    for i in 0..100 {
        assert!(g.admit(&format!("sig-{i}"), 1000));
    }
    assert!(g.len() <= 8, "grew to {}", g.len());
}

#[test]
fn replay_guard_starts_empty() {
    let g = ReplayGuard::new(300, 4);
    assert!(g.is_empty());
    assert_eq!(g.len(), 0);
}
