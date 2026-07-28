//! FJ-2723 (PMAT-199): a selector matching nothing is an error, not a no-op.
//!
//! `forjar apply -r <typo>` printed `0 converged, 0 unchanged` and exited 0.
//! Every signal said success while nothing had been applied. In CI the exit
//! code is often the only thing read, so a typo'd targeted apply looked like a
//! completed deploy.

use std::path::Path;

fn cfg(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(
        &p,
        r#"
version: "1.0"
name: sel
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  real-one:
    type: file
    machine: local
    path: /tmp/forjar-selector-guard.txt
    content: hi
    tags: [live]
    resource_group: alpha
"#,
    )
    .unwrap();
    p
}

fn check(
    file: &Path,
    resource: Option<&str>,
    tag: Option<&str>,
    group: Option<&str>,
) -> Result<(), String> {
    let config = super::helpers::parse_and_validate(file).expect("fixture parses");
    super::apply_selection::reject_empty_selection(&config, resource, tag, group)
}

#[test]
fn unknown_resource_selector_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    let f = cfg(d.path());
    let err = check(&f, Some("no-such-resource"), None, None)
        .expect_err("a typo'd -r must not report success");
    assert!(err.contains("no-such-resource"), "{err}");
    assert!(
        err.contains("real-one"),
        "the error should name what IS available: {err}"
    );
}

#[test]
fn unknown_tag_selector_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    let f = cfg(d.path());
    let err = check(&f, None, Some("no-such-tag"), None).expect_err("typo'd -t must fail");
    assert!(err.contains("no-such-tag"), "{err}");
}

#[test]
fn unknown_group_selector_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    let f = cfg(d.path());
    let err = check(&f, None, None, Some("no-such-group")).expect_err("typo'd -g must fail");
    assert!(err.contains("no-such-group"), "{err}");
}

#[test]
fn selectors_that_match_are_accepted() {
    // The guard must not reject valid invocations — that would be worse than
    // the bug it fixes.
    let d = tempfile::tempdir().unwrap();
    let f = cfg(d.path());
    for (r, t, g) in [
        (Some("real-one"), None, None),
        (None, Some("live"), None),
        (None, None, Some("alpha")),
        (None, None, None),
    ] {
        let out = check(&f, r, t, g);
        assert!(
            out.is_ok(),
            "valid selector ({r:?}, {t:?}, {g:?}) was rejected: {out:?}"
        );
    }
}
