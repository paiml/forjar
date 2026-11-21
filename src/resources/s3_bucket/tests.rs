//! `s3_bucket` falsifiers: spec validation, check, apply and state query against the
//! stateful fake `aws` in `tests_fake`.

use super::tests_fake::{mutating, res, Fake, BUCKET};
use super::*;
use std::os::unix::fs::PermissionsExt;

/// One drift case: label, marker the check must print, and how to break the fake.
pub(super) type Case = (&'static str, &'static str, Box<dyn Fn(&Fake)>);
type SpecCase = (&'static str, Box<dyn Fn(&mut Resource)>);

// ---- spec_of ----------------------------------------------------------------

#[test]
fn a_valid_declaration_is_accepted_with_defaults() {
    let mut r = res();
    r.s3.private = None;
    r.s3.sse = None;
    r.s3.tls_only = None;
    r.s3.versioning = None;
    r.s3.noncurrent_expire_days = None;
    let s = spec_of(&r).unwrap();
    assert!(s.tls_only && s.versioning && s.noncurrent_expire_days.is_none());
}

#[test]
fn bad_declarations_are_refused() {
    let cases: Vec<SpecCase> = vec![
        ("no bucket", Box::new(|r| r.s3.bucket = None)),
        (
            "empty bucket",
            Box::new(|r| r.s3.bucket = Some(String::new())),
        ),
        (
            "upper case",
            Box::new(|r| r.s3.bucket = Some("Bad-Name".into())),
        ),
        ("too short", Box::new(|r| r.s3.bucket = Some("ab".into()))),
        (
            "leading dash",
            Box::new(|r| r.s3.bucket = Some("-abc".into())),
        ),
        (
            "trailing dash",
            Box::new(|r| r.s3.bucket = Some("abc-".into())),
        ),
        (
            "double dot",
            Box::new(|r| r.s3.bucket = Some("a..b".into())),
        ),
        (
            "quote injection",
            Box::new(|r| r.s3.bucket = Some("a';rm -rf /;'b".into())),
        ),
        ("no region", Box::new(|r| r.s3.region = None)),
        (
            "bad region",
            Box::new(|r| r.s3.region = Some("useast1".into())),
        ),
        (
            "region injection",
            Box::new(|r| r.s3.region = Some("us-east-1;id".into())),
        ),
        ("public", Box::new(|r| r.s3.private = Some(false))),
        ("kms", Box::new(|r| r.s3.sse = Some("aws:kms".into()))),
        (
            "zero days",
            Box::new(|r| r.s3.noncurrent_expire_days = Some(0)),
        ),
        (
            "huge days",
            Box::new(|r| r.s3.noncurrent_expire_days = Some(36_501)),
        ),
        ("absent", Box::new(|r| r.state = Some("absent".into()))),
    ];
    for (label, f) in cases {
        let mut r = res();
        f(&mut r);
        assert!(spec_of(&r).is_err(), "{label} must be refused");
        for s in [check_script(&r), apply_script(&r), state_query_script(&r)] {
            assert!(s.contains("REFUSED"), "{label}: {s}");
            assert!(
                !s.contains("aws s3api"),
                "{label}: refusal must not reach aws"
            );
        }
    }
}

#[test]
fn name_and_region_allow_lists() {
    for ok in ["abc", "paiml-metrics-preview", "a.b-c", "a1b"] {
        assert!(is_bucket_name(ok), "{ok}");
    }
    for ok in [
        "us-east-1",
        "eu-south-2",
        "us-gov-west-1",
        "ap-southeast-12",
    ] {
        assert!(is_region(ok), "{ok}");
    }
    for bad in [
        "",
        "us-east",
        "us_east_1",
        "US-EAST-1",
        "us-east-x",
        "a-b-c-d-1",
    ] {
        assert!(!is_region(bad), "{bad}");
    }
}

// ---- check: asks AWS --------------------------------------------------------

#[test]
fn check_is_green_on_a_converged_bucket_and_never_writes() {
    let f = Fake::converged();
    f.clear_log();
    let (rc, out, err) = f.run(&check_script(&res()));
    assert_eq!(rc, 0, "{out}\n{err}");
    for m in [
        "s3-bucket-present",
        "s3-sse-aes256",
        "s3-public-access-blocked",
        "s3-versioning-enabled",
        "s3-lifecycle-present",
        "s3-tls-only-policy",
    ] {
        assert!(out.contains(&format!("{m}:{BUCKET}")), "{m}: {out}");
    }
    assert!(mutating(&f.log()).is_empty(), "check wrote: {}", f.log());
    for op in [
        "head-bucket",
        "get-bucket-encryption",
        "get-public-access-block",
        "get-bucket-versioning",
        "get-bucket-lifecycle-configuration",
        "get-bucket-policy",
    ] {
        assert!(f.log().contains(op), "check never asked AWS {op}");
    }
}

#[test]
fn check_never_reads_a_lock() {
    let f = Fake::converged();
    let s = check_script(&res());
    assert!(
        !s.contains("forjar.lock") && !s.contains("state/") && !s.contains(".lock"),
        "{s}"
    );
    // AWS changes behind forjar's back: the check must see it with no lock involved.
    f.set("enc", "aws:kms\n");
    assert_ne!(f.run(&s).0, 0);
}

/// One broken property per case: the check must go red and name exactly that marker.
pub(super) fn cases() -> Vec<Case> {
    vec![
        (
            "bucket",
            "s3-bucket-absent",
            Box::new(|f| f.unset("exists")),
        ),
        (
            "sse",
            "s3-sse-missing",
            Box::new(|f| f.set("enc", "aws:kms\n")),
        ),
        ("sse-unset", "s3-sse-missing", Box::new(|f| f.unset("enc"))),
        (
            "pab",
            "s3-public-access-open",
            Box::new(|f| f.set("pab", "True\tFalse\tTrue\tTrue\n")),
        ),
        (
            "pab-unset",
            "s3-public-access-open",
            Box::new(|f| f.unset("pab")),
        ),
        (
            "versioning",
            "s3-versioning-off",
            Box::new(|f| f.set("ver", "Suspended\n")),
        ),
        (
            "versioning-unset",
            "s3-versioning-off",
            Box::new(|f| f.unset("ver")),
        ),
        (
            "lifecycle-days",
            "s3-lifecycle-missing",
            Box::new(|f| f.set("lc", "Enabled\t7\n")),
        ),
        (
            "lifecycle-disabled",
            "s3-lifecycle-missing",
            Box::new(|f| f.set("lc", "Disabled\t30\n")),
        ),
        (
            "lifecycle-unset",
            "s3-lifecycle-missing",
            Box::new(|f| f.unset("lc")),
        ),
        (
            "policy-unset",
            "s3-tls-only-policy-missing",
            Box::new(|f| f.unset("pol")),
        ),
        (
            "policy-allow",
            "s3-tls-only-policy-missing",
            Box::new(|f| {
                let p = std::fs::read_to_string(f.file("pol")).unwrap();
                f.set("pol", &p.replace("\"Deny\"", "\"Allow\""));
            }),
        ),
        (
            "policy-secure-transport-true",
            "s3-tls-only-policy-missing",
            Box::new(|f| {
                let p = std::fs::read_to_string(f.file("pol")).unwrap();
                f.set("pol", &p.replace("\"false\"", "\"true\""));
            }),
        ),
        (
            "policy-other-bucket",
            "s3-tls-only-policy-missing",
            Box::new(|f| {
                let p = std::fs::read_to_string(f.file("pol")).unwrap();
                f.set("pol", &p.replace(BUCKET, "someone-elses-bucket"));
            }),
        ),
        (
            "policy-objects-only",
            "s3-tls-only-policy-missing",
            Box::new(|f| {
                let p = std::fs::read_to_string(f.file("pol")).unwrap();
                f.set(
                    "pol",
                    &p.replace(&format!("\"arn:aws:s3:::{BUCKET}\","), ""),
                );
            }),
        ),
    ]
}

#[test]
fn every_property_drift_turns_the_check_red_with_its_own_marker() {
    for (label, marker, break_it) in cases() {
        let f = Fake::converged();
        break_it(&f);
        let (rc, out, err) = f.run(&check_script(&res()));
        assert_eq!(rc, 1, "{label}: should be divergent\n{out}\n{err}");
        assert!(
            out.contains(&format!("{marker}:{BUCKET}")),
            "{label}: {out}"
        );
    }
}

#[test]
fn a_failed_head_bucket_is_an_error_not_absent() {
    for msg in [
        "An error occurred (403) when calling the HeadBucket operation: Forbidden",
        "An error occurred (301) when calling the HeadBucket operation: Moved Permanently",
        "Unable to locate credentials",
        "An error occurred (ExpiredToken) when calling the HeadBucket operation",
    ] {
        let f = Fake::converged();
        f.set("head.err", msg);
        let (rc, out, err) = f.run(&check_script(&res()));
        assert_eq!(rc, 1, "{msg}");
        assert!(err.contains("bucket-lookup-failed"), "{msg}: {err}");
        assert!(
            !out.contains("s3-bucket-absent"),
            "{msg}: reported absent: {out}"
        );
        // ...and apply must not try to create a bucket it cannot see.
        f.clear_log();
        let (rc, _, err) = f.run(&apply_script(&res()));
        assert_eq!(rc, 1, "{msg}: {err}");
        assert!(mutating(&f.log()).is_empty(), "{msg}: wrote {}", f.log());
    }
}

#[test]
fn optional_properties_are_only_asserted_when_declared() {
    let mut r = res();
    r.s3.versioning = Some(false);
    r.s3.tls_only = Some(false);
    r.s3.noncurrent_expire_days = None;
    let f = Fake::new();
    f.set("exists", "");
    f.set("enc", "AES256\n");
    f.set("pab", "True\tTrue\tTrue\tTrue\n");
    let (rc, out, _) = f.run(&check_script(&r));
    assert_eq!(rc, 0, "{out}");
    for m in ["versioning", "lifecycle", "tls-only"] {
        assert!(!out.contains(m), "{m} asserted though undeclared: {out}");
    }
    let a = apply_script(&r);
    for op in [
        "put-bucket-versioning",
        "put-bucket-lifecycle",
        "put-bucket-policy",
    ] {
        assert!(!a.contains(op), "{op} written though undeclared");
    }
}

// ---- apply ------------------------------------------------------------------

#[test]
fn apply_creates_and_configures_an_empty_account_in_order() {
    let f = Fake::new();
    let (rc, out, err) = f.run(&apply_script(&res()));
    assert_eq!(rc, 0, "{out}\n{err}");
    let w: Vec<String> = mutating(&f.log())
        .iter()
        .map(|l| l.split_whitespace().nth(1).unwrap().to_string())
        .collect();
    assert_eq!(
        w,
        [
            "create-bucket",
            "put-public-access-block",
            "put-bucket-encryption",
            "put-bucket-versioning",
            "put-bucket-lifecycle-configuration",
            "put-bucket-policy"
        ]
    );
    assert_eq!(f.run(&check_script(&res())).0, 0);
}

#[test]
fn us_east_1_needs_no_location_constraint_other_regions_do() {
    let f = Fake::new();
    f.run(&apply_script(&res()));
    assert!(!f.log().contains("LocationConstraint"), "{}", f.log());
    let mut r = res();
    r.s3.region = Some("eu-south-2".into());
    let f = Fake::new();
    f.run(&apply_script(&r));
    let create = f
        .log()
        .lines()
        .find(|l| l.contains("create-bucket"))
        .unwrap()
        .to_string();
    assert!(
        create.contains("--create-bucket-configuration LocationConstraint=eu-south-2"),
        "{create}"
    );
}

#[test]
fn apply_on_an_existing_bucket_does_not_create_and_is_idempotent() {
    let f = Fake::converged();
    let before: Vec<String> = ["enc", "pab", "ver", "lc", "pol"]
        .iter()
        .map(|n| std::fs::read_to_string(f.file(n)).unwrap())
        .collect();
    f.clear_log();
    let (rc, out, err) = f.run(&apply_script(&res()));
    assert_eq!(rc, 0, "{out}\n{err}");
    assert!(!f.log().contains("create-bucket"), "{}", f.log());
    let after: Vec<String> = ["enc", "pab", "ver", "lc", "pol"]
        .iter()
        .map(|n| std::fs::read_to_string(f.file(n)).unwrap())
        .collect();
    assert_eq!(before, after, "second apply changed AWS state");
}

#[test]
fn apply_repairs_every_drift_case() {
    for (label, _, break_it) in cases() {
        let f = Fake::converged();
        break_it(&f);
        let (rc, out, err) = f.run(&apply_script(&res()));
        assert_eq!(rc, 0, "{label}: {out}\n{err}");
        assert_eq!(f.run(&check_script(&res())).0, 0, "{label}");
    }
}

#[test]
fn apply_fails_when_aws_does_not_take_the_write() {
    // A put that exits 0 but does not stick (the stub's state file is read-only
    // after the first put): the read-back must catch it.
    let f = Fake::new();
    let bad = r#"#!/bin/sh
case "$2" in put-bucket-encryption) exit 0;; esac
exec "$REAL_AWS" "$@"
"#;
    let real = f.path().join("bin/aws");
    let moved = f.path().join("bin/aws-real");
    std::fs::rename(&real, &moved).unwrap();
    std::fs::write(&real, bad).unwrap();
    std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o755)).unwrap();
    let script = format!(
        "REAL_AWS={}; export REAL_AWS\n{}",
        moved.display(),
        apply_script(&res())
    );
    let (rc, out, err) = f.run(&script);
    assert_eq!(rc, 1, "{out}");
    assert!(out.contains("s3-sse-missing"), "{out}");
    assert!(err.contains("does not match the declaration"), "{err}");
}

#[test]
fn state_query_reports_what_aws_says_and_never_writes() {
    let f = Fake::converged();
    f.clear_log();
    let (rc, out, _) = f.run(&state_query_script(&res()));
    assert_eq!(rc, 0);
    assert!(
        out.contains(&format!("bucket={BUCKET} present=yes sse=AES256 pab=True True True True versioning=Enabled lifecycle=Enabled 30 tls_only=yes")),
        "{out}"
    );
    assert!(mutating(&f.log()).is_empty());
    f.set("enc", "aws:kms\n");
    assert!(f.run(&state_query_script(&res())).1.contains("sse=aws:kms"));
    f.unset("exists");
    assert!(f.run(&state_query_script(&res())).1.contains("present=no"));
}

#[test]
fn every_script_passes_the_i8_gate() {
    use crate::core::purifier::validate_script;
    let mut variants = vec![res()];
    let mut r = res();
    r.s3.region = Some("eu-south-2".into());
    r.s3.versioning = Some(false);
    r.s3.tls_only = Some(false);
    r.s3.noncurrent_expire_days = None;
    variants.push(r);
    for r in &variants {
        for (which, s) in [
            ("check", check_script(r)),
            ("apply", apply_script(r)),
            ("state", state_query_script(r)),
        ] {
            if let Err(e) = validate_script(&s) {
                panic!("I8 gate rejects the {which} script:\n{e}\n---\n{s}");
            }
        }
    }
}
