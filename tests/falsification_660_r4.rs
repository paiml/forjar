//! forjar#660 round 4: the class behind the five reproduced defects.
//!
//!   1. plan / apply / lock read through the ONE loader (tags, groups, params).
//!   2. Resolution walks EVERY string of the config, not an enumerated list.
//!   3. A data value is substituted once, as literal text: `{{build}}` stays.
//!   4. `config-merge` merges `data:` and names both files on a conflict.
//!
//! The structural half (no `cmd_*` bypasses the loader) is
//! `falsification_660_one_loader.rs`. Fixtures mirror `r4/me/rep/`.

mod common_660;

use common_660::*;
use std::path::Path;

fn write(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(name), body).unwrap();
}

const HEAD: &str =
    "version: \"1.0\"\nname: r4-660\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\n";
const GREETING: &str = "data:\n  greeting:\n    type: command\n    value: \"echo hello\"\n";

fn ok(dir: &Path, args: &[&str]) -> String {
    let out = run(dir, args);
    let body = text(&out);
    assert!(out.status.success(), "forjar {args:?} failed: {body}");
    body
}

// ---- 1. plan / apply / lock use the loader -------------------------------

fn tagged() -> String {
    format!(
        "{HEAD}resources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/r4-660.txt\n    content: ok\n    resource_group: \"{{{{data.greeting}}}}\"\n    tags: [\"{{{{data.greeting}}}}\"]\n{GREETING}"
    )
}

#[test]
fn plan_and_apply_filter_on_a_resolved_tag_and_group() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "f.yaml", &tagged());
    for args in [
        vec!["plan", "-f", "f.yaml", "-t", "hello"],
        vec!["plan", "-f", "f.yaml", "-g", "hello"],
        vec!["apply", "--dry-run", "-f", "f.yaml", "-t", "hello"],
    ] {
        let body = ok(d.path(), &args);
        assert!(body.contains("cfg"), "{args:?} planned nothing: {body}");
    }
}

#[test]
fn plan_resolves_a_data_reference_that_sits_in_a_param() {
    let d = tempfile::tempdir().unwrap();
    let y = format!(
        "{HEAD}params:\n  p: \"v={{{{data.greeting}}}}\"\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/r4-660.txt\n    content: \"{{{{params.p}}}}\"\n{GREETING}"
    );
    write(d.path(), "f.yaml", &y);
    let body = ok(d.path(), &["plan", "-f", "f.yaml"]);
    assert!(body.contains("v=hello"), "plan: {body}");
    assert!(!body.contains("{{data."), "plan leaked a reference: {body}");
}

// ---- 2. resolution walks the whole tree ----------------------------------

fn env_manifest() -> String {
    format!(
        "{HEAD}resources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/r4-660.txt\n    content: ok\ndescription: \"{{{{data.greeting}}}}\"\nenvironments:\n  dev:\n    description: \"{{{{data.greeting}}}}\"\n    params:\n      p: \"{{{{data.greeting}}}}\"\n{GREETING}"
    )
    .replace("hostname: m1", "hostname: \"{{data.greeting}}\"")
}

#[test]
fn machines_environments_and_description_resolve_data() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "f.yaml", &env_manifest());
    let inv = ok(d.path(), &["inventory", "-f", "f.yaml"]);
    assert!(inv.contains("m1 (hello)"), "inventory: {inv}");
    let envs = ok(d.path(), &["environments", "list", "-f", "f.yaml"]);
    assert!(envs.contains("dev — hello"), "environments: {envs}");
}

/// An undeclared `{{data.missing}}` is rejected wherever it sits: the places
/// are DATA, each row a different top-level section of the manifest.
#[test]
fn exhaustive_validate_rejects_an_undeclared_reference_in_every_section() {
    let places: &[(&str, &str)] = &[
        ("machine hostname", "hostname: \"{{data.missing}}\"\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/x\n    content: ok\n"),
        ("machine addr", "hostname: m1\n    addr: \"{{data.missing}}\"\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/x\n    content: ok\n"),
        ("description", "hostname: m1\n    addr: 127.0.0.1\ndescription: \"{{data.missing}}\"\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/x\n    content: ok\n"),
        ("environment", "hostname: m1\n    addr: 127.0.0.1\nenvironments:\n  dev:\n    description: \"{{data.missing}}\"\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/x\n    content: ok\n"),
        ("check", "hostname: m1\n    addr: 127.0.0.1\nchecks:\n  c1:\n    machine: m1\n    command: \"echo {{data.missing}}\"\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/x\n    content: ok\n"),
        ("tag", "hostname: m1\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/x\n    content: ok\n    tags: [\"{{data.missing}}\"]\n"),
        ("resource_group", "hostname: m1\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/x\n    content: ok\n    resource_group: \"{{data.missing}}\"\n"),
        ("output", "hostname: m1\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: /tmp/x\n    content: ok\noutputs:\n  o:\n    value: \"{{data.missing}}\"\n"),
    ];
    let d = tempfile::tempdir().unwrap();
    for (place, tail) in places {
        let y = format!("version: \"1.0\"\nname: r4-660\nmachines:\n  m1:\n    {tail}");
        write(d.path(), "f.yaml", &y);
        let out = run(d.path(), &["validate", "--exhaustive", "-f", "f.yaml"]);
        let body = text(&out);
        assert!(!out.status.success(), "{place}: validate passed: {body}");
        assert!(
            body.contains("unknown data source 'missing'"),
            "{place}: {body}"
        );
    }
}

// ---- 3. a data value is substituted once, as literal text ----------------

fn literal() -> String {
    format!(
        "{HEAD}params:\n  p: \"{{{{data.snippet}}}}\"\nresources:\n  cfg:\n    type: package\n    provider: apt\n    machine: m1\n    packages: [curl]\n    version: \"{{{{data.snippet}}}}\"\n  file:\n    type: file\n    machine: m1\n    path: /tmp/r4-660.txt\n    content: \"c={{{{data.snippet}}}}\"\noutputs:\n  raw:\n    value: \"{{{{data.snippet}}}}\"\ndata:\n  snippet:\n    type: command\n    value: \"echo 'hello {{{{build}}}}'\"\n"
    )
}

#[test]
fn a_value_containing_braces_comes_out_verbatim_everywhere() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "f.yaml", &literal());
    let show = ok(d.path(), &["show", "-f", "f.yaml"]);
    for want in [
        "c=hello {{build}}",
        "p: hello {{build}}",
        "value: hello {{build}}",
    ] {
        assert!(show.contains(want), "show lacks `{want}`: {show}");
    }
    let pin = ok(d.path(), &["pin", "-f", "f.yaml"]);
    assert!(pin.contains("hello {{build}}"), "pin: {pin}");
    let sbom = ok(d.path(), &["sbom", "-f", "f.yaml", "--json"]);
    assert!(
        sbom.contains("\"versionInfo\": \"hello {{build}}\""),
        "sbom: {sbom}"
    );
    let plan = ok(d.path(), &["plan", "-f", "f.yaml"]);
    assert!(plan.contains("c=hello {{build}}"), "plan: {plan}");
    let out = ok(d.path(), &["output", "-f", "f.yaml", "raw"]);
    assert!(out.contains("hello {{build}}"), "output: {out}");
}

// ---- 4. config-merge merges data: ----------------------------------------

fn side(name: &str, machine: &str, key: &str, cmd: &str) -> String {
    format!(
        "version: \"1.0\"\nname: {name}\nmachines:\n  {machine}:\n    hostname: {machine}\n    addr: 127.0.0.1\nresources:\n  r-{name}:\n    type: file\n    machine: {machine}\n    path: /tmp/{name}.txt\n    content: \"{{{{data.{key}}}}}\"\ndata:\n  {key}:\n    type: command\n    value: \"{cmd}\"\n"
    )
}

#[test]
fn config_merge_keeps_both_data_blocks_and_validates() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "a.yaml", &side("a", "m1", "src_a", "echo from_a"));
    write(d.path(), "b.yaml", &side("b", "m2", "src_b", "echo from_b"));
    ok(
        d.path(),
        &["config-merge", "a.yaml", "b.yaml", "-o", "m.yaml"],
    );
    let merged = std::fs::read_to_string(d.path().join("m.yaml")).unwrap();
    assert!(
        merged.contains("src_a") && merged.contains("src_b"),
        "{merged}"
    );
    ok(d.path(), &["validate", "-f", "m.yaml"]);
}

#[test]
fn config_merge_names_both_files_on_a_data_conflict() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "a.yaml", &side("a", "m1", "shared", "echo one"));
    write(d.path(), "b.yaml", &side("b", "m2", "shared", "echo two"));
    let out = run(
        d.path(),
        &["config-merge", "a.yaml", "b.yaml", "-o", "m.yaml"],
    );
    let body = text(&out);
    assert!(!out.status.success(), "conflict merged: {body}");
    assert!(
        body.contains("shared") && body.contains("a.yaml") && body.contains("b.yaml"),
        "{body}"
    );
    assert!(!d.path().join("m.yaml").exists());
}
