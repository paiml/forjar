//! forjar#660 round 5: a data value is OPAQUE text, whatever braces it holds,
//! and every loader hands a reader the value as written.
//!
//!   1. "{", "}", "{{" and adjacency with config text come out verbatim, both
//!      from the loaders (what a reader sees) and from `show` (what a render sees).
//!   2. `load_resolved_config` and `load_resolved_config_lenient` return decoded
//!      text: no stand-in character ever reaches a reader.
//!   3. A reserved stand-in character is an error where it could be mistaken
//!      for a brace, and passes through untouched where no data is in play.
//!   4. A `{{data.x}}` in a mapping KEY is rejected by every loader.

mod common_660;

use common_660::*;
use std::path::Path;

fn write(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(name), body).unwrap();
}

fn manifest(data_cmd: &str, content: &str) -> String {
    format!(
        "version: \"1.0\"\nname: r5-660\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nparams:\n  p: PVAL\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/r5-660.txt\n    content: \"{content}\"\n    tags: [\"{content}\"]\ndata:\n  a:\n    type: command\n    value: \"{data_cmd}\"\n"
    )
}

/// (value the source prints, content template, text the reader must see)
const CASES: &[(&str, &str, &str)] = &[
    ("{", "{{data.a}}", "{"),
    ("}", "{{data.a}}", "}"),
    ("{{", "{{data.a}}", "{{"),
    ("}}", "{{data.a}}", "}}"),
    ("{", "{{data.a}}{params.p}}", "{{params.p}}"),
    ("{{", "{{data.a}}params.p}}", "{{params.p}}"),
    ("{", "{{data.a}}{{data.a}}params.p}}", "{{params.p}}"),
    ("}", "{{data.a}}{{data.a}}", "}}"),
    ("{{build}}", "x-{{data.a}}-y", "x-{{build}}-y"),
    ("{{params.p}}", "{{data.a}}", "{{params.p}}"),
];

fn fixture_for(value: &str, content: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let cmd = format!("printf '%s' '{value}'");
    write(d.path(), "f.yaml", &manifest(&cmd, content));
    d
}

fn assert_clean(what: &str, s: &str) {
    assert!(
        !s.contains('\u{E001}') && !s.contains('\u{E002}'),
        "{what}: a stand-in character reached a reader: {s:?}"
    );
}

#[test]
fn a_reader_sees_the_value_verbatim_from_both_loaders() {
    for (value, content, want) in CASES {
        let d = fixture_for(value, content);
        let f = d.path().join("f.yaml");
        let strict = forjar::core::resolver::load_resolved_config(&f).unwrap();
        let lenient = forjar::core::resolver::load_resolved_config_lenient(&f).unwrap();
        for (name, cfg) in [("load_resolved_config", strict), ("lenient", lenient)] {
            let r = &cfg.resources["cfg"];
            assert_eq!(
                r.content.as_deref(),
                Some(*want),
                "{name}: {value:?} in {content:?}"
            );
            assert_eq!(
                r.tags,
                vec![want.to_string()],
                "{name} tags: {value:?} in {content:?}"
            );
            assert_clean(name, &serde_json::to_string(&cfg).unwrap());
        }
    }
}

#[test]
fn a_render_sees_the_value_verbatim_and_never_re_expands_it() {
    for (value, content, want) in CASES {
        let d = fixture_for(value, content);
        let out = run(d.path(), &["show", "-f", "f.yaml", "--json"]);
        let body = text(&out);
        assert!(out.status.success(), "{value:?} in {content:?}: {body}");
        let v: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout))
            .unwrap_or_else(|e| panic!("show --json: {e}: {body}"));
        let got = v["resources"]["cfg"]["content"].as_str().unwrap_or("");
        assert_eq!(got, *want, "show: {value:?} in {content:?}");
        assert_clean("show", &body);
    }
}

#[test]
fn the_pin_command_reads_decoded_text_too() {
    let d = fixture_for("{{", "{{data.a}}x");
    let out = run(d.path(), &["sbom", "-f", "f.yaml"]);
    assert_clean("sbom", &text(&out));
}

#[test]
fn a_stand_in_character_in_a_data_value_is_an_error() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "f.yaml",
        &manifest("printf '\\\\356\\\\200\\\\201'", "{{data.a}}"),
    );
    let out = run(d.path(), &["show", "-f", "f.yaml"]);
    let body = text(&out);
    assert!(
        !out.status.success(),
        "accepted a reserved character: {body}"
    );
    assert!(body.contains("reserved"), "unhelpful error: {body}");
}

#[test]
fn a_stand_in_character_in_config_text_is_untouched_when_no_data_is_declared() {
    let d = tempfile::tempdir().unwrap();
    let y = "version: \"1.0\"\nname: r5-660\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/r5-660.txt\n    content: \"a\u{E001}b\"\n";
    write(d.path(), "f.yaml", y);
    let out = run(d.path(), &["show", "-f", "f.yaml", "--json"]);
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    assert!(body.contains("a\u{E001}b"), "rewrote authored text: {body}");
}

// ---- 4. mapping keys -------------------------------------------------------

fn keyed(section: &str) -> String {
    let base = manifest("echo hello", "ok");
    match section {
        "params" => base.replace("  p: PVAL\n", "  p: PVAL\n  \"{{data.a}}\": v\n"),
        _ => base.replace("machines:\n", "x-labels:\n  \"{{data.a}}\": v\nmachines:\n"),
    }
}

#[test]
fn a_data_reference_in_a_mapping_key_is_rejected_by_every_loader() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "f.yaml", &keyed("params"));
    let f = d.path().join("f.yaml");
    assert!(forjar::core::resolver::load_resolved_config(&f).is_err());
    assert!(forjar::core::resolver::load_resolved_config_lenient(&f).is_err());
    for args in [
        vec!["validate", "-f", "f.yaml"],
        vec!["validate", "-f", "f.yaml", "--exhaustive"],
        vec!["show", "-f", "f.yaml"],
        vec!["plan", "-f", "f.yaml"],
    ] {
        let out = run(d.path(), &args);
        let body = text(&out);
        assert!(
            !out.status.success(),
            "{args:?} accepted a key reference: {body}"
        );
        assert!(
            body.contains("mapping key") && body.contains("data"),
            "{args:?}: the error does not name the key: {body}"
        );
    }
}

#[test]
fn a_key_that_is_not_a_data_reference_still_loads() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "f.yaml",
        &manifest("echo hello", "ok").replace("  p: PVAL\n", "  p: PVAL\n  \"{{params.p}}\": v\n"),
    );
    let out = run(d.path(), &["show", "-f", "f.yaml"]);
    assert!(!text(&out).contains("mapping key"), "{}", text(&out));
}
