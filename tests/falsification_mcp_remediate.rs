//! `forjar_remediate` must read the value it writes from the POLICY, and must
//! say so when it cannot fix something (paiml/forjar#356, F-MCP-004).
//!
//! WHAT THIS FEATURE IS FOR. An agent asked to "fix the permissions on this
//! config" guesses two things: which value is correct, and where in the file to
//! write it. The first guess is the dangerous one — `0644` is a fact about
//! someone's policy, not a constant. A remediation tool that hardcodes it has
//! not removed the hallucination, it has relocated it into the tool and given
//! it forjar's authority. So the property under test is narrow and load-bearing:
//!
//!     a remediation's target value is READ FROM THE POLICY RULE,
//!     never chosen by forjar.
//!
//! Its immediate consequence is the second property here: **only `assert` rules
//! are auto-fixable.** `deny`/`warn` name a value that is FORBIDDEN with no
//! replacement; `require` names a field with no value; `limit` bounds a list.
//! `deny mode == "0777"` is the most natural way to write the motivating
//! example and it is NOT fixable — reporting that, with the reason, is the most
//! valuable output this verb produces, and test 2 is what keeps it honest.
//!
//! WHY THEY DRIVE `verb::find(...).invoke(...)`. That is the transport-neutral
//! entry point every surface routes through, so one assertion covers the CLI
//! leaf, the MCP tool and the HTTP verb at once — a verb cannot pass here and
//! be wrong on one transport.
//!
//! HOW EACH IS MADE TO FAIL. Each test kills a specific plausible-but-wrong
//! implementation rather than merely exercising the happy path:
//!
//! - hardcode `"0644"` in `fixes::derive` and accept `Deny` -> 1 and 2 red.
//! - replace the byte-range splice with `serde_yaml_ng::to_string(&config)`,
//!   i.e. copy the `lint --fix` implementation this epic had to fix first ->
//!   3 red (comments gone, include inlined, every null field emitted) and
//!   4 red (the first pass reformats, so the second is not a fixpoint).
//! - build `remaining_violations` by deleting the fixed entries from the
//!   original list instead of re-evaluating -> 5 red (VERIFIED: the earlier
//!   version of test 5, without the `deny` rule, survived this mutation —
//!   every entry it checked was either fixed-and-deleted or never fixed. A
//!   rule satisfied as a SIDE EFFECT is what separates the two designs).
//! - flip the registry row to `Effects::Mutating` and add a `fs::write` ->
//!   6 red.
//! - drop the flow-style guard and fall back to first-match substitution ->
//!   7 red: the flow-style case is corrupted rather than refused.

use serde_json::json;
use std::path::Path;

fn invoke(params: serde_json::Value) -> serde_json::Value {
    let verb = forjar::verb::find("remediate").expect("the remediate verb is registered");
    (verb.invoke)(params).expect("remediate returned a result")
}

fn write(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).expect("fixture written");
    path
}

/// A config with a comment, an include, a block scalar and a 0777 file mode.
fn fixture(policies: &str) -> String {
    format!(
        r#"version: "1.0"
name: remediate-fixture
# This comment carries the reason the mode matters. It must survive.
includes:
  - extra.yaml
machines:
  box:
    hostname: box
    addr: 127.0.0.1
resources:
  web-conf:
    type: file
    machine: box
    path: /etc/web.conf
    mode: "0777"
    content: |
      listen 80;
policies:
{policies}"#
    )
}

const EXTRA: &str = r#"version: "1.0"
name: extra
resources:
  included-conf:
    type: file
    machine: box
    path: /etc/included.conf
    mode: "0777"
"#;

fn assert_mode(value: &str) -> String {
    format!(
        "  - type: assert\n    id: SEC-MODE\n    message: files must be {value}\n\
         \x20   resource_type: file\n    condition_field: mode\n    condition_value: \"{value}\"\n"
    )
}

const DENY_0777: &str = "  - type: deny\n    id: SEC-NO-0777\n    message: 0777 is forbidden\n\
                         \x20   resource_type: file\n    condition_field: mode\n\
                         \x20   condition_value: \"0777\"\n";

const REQUIRE_OWNER: &str = "  - type: require\n    id: OWN-001\n\
                             \x20   message: files must declare an owner\n\
                             \x20   resource_type: file\n    field: owner\n";

/// A project whose only fixable violation is on `web-conf`.
fn project(policies: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "extra.yaml", EXTRA);
    let path = write(dir.path(), "forjar.yaml", &fixture(policies));
    (dir, path)
}

fn run(path: &Path) -> serde_json::Value {
    invoke(json!({ "path": path.to_string_lossy() }))
}

fn updated(out: &serde_json::Value) -> &str {
    out["updated_yaml_content"]
        .as_str()
        .expect("updated_yaml_content is a string")
}

fn reasons(out: &serde_json::Value) -> Vec<String> {
    out["remaining_violations"]
        .as_array()
        .expect("remaining_violations is an array")
        .iter()
        .map(|v| v["reason"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// 1. Two byte-identical projects that differ only in the policy's
///    `condition_value` must produce two different documents.
#[test]
fn the_target_value_is_read_from_the_policy_not_from_a_builtin() {
    for want in ["0644", "0600"] {
        let (_dir, path) = project(&assert_mode(want));
        let out = run(&path);
        assert!(
            updated(&out).contains(&format!("mode: \"{want}\"")),
            "policy asked for {want} but the document says otherwise:\n{}",
            updated(&out)
        );
        let applied = out["remediations_applied"].as_array().expect("array");
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0]["to"], want);
        assert_eq!(applied[0]["from"], "0777");
    }
}

/// 2. A `deny` rule names what to avoid, not what to write. Guessing a
///    replacement here is the defect the whole feature exists to prevent.
#[test]
fn a_deny_rule_is_reported_unfixable_never_guessed() {
    let (_dir, path) = project(DENY_0777);
    let source = std::fs::read_to_string(&path).expect("read");
    let out = run(&path);
    assert_eq!(
        out["remediations_applied"].as_array().expect("array").len(),
        0,
        "forjar invented a replacement for a deny rule"
    );
    assert_eq!(updated(&out), source, "the document was edited anyway");
    assert_eq!(out["changed"], false);
    assert!(
        reasons(&out).iter().any(|r| r.contains("FORBIDDEN")),
        "the reason does not say why a deny rule has no fix: {:?}",
        reasons(&out)
    );
}

/// 3. Exactly one line may differ, and the comment, the include and the block
///    scalar must all come back verbatim.
#[test]
fn every_untouched_line_survives_byte_for_byte() {
    let (_dir, path) = project(&assert_mode("0644"));
    let source = std::fs::read_to_string(&path).expect("read");
    let out = run(&path);
    let after = updated(&out);

    assert_eq!(
        after.lines().count(),
        source.lines().count(),
        "the line count changed — the document was re-serialised, not edited"
    );
    let differing: Vec<usize> = source
        .lines()
        .zip(after.lines())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(differing.len(), 1, "more than one line changed");
    assert!(source
        .lines()
        .nth(differing[0])
        .expect("line")
        .contains("mode:"));

    assert!(after.contains("# This comment carries the reason"));
    assert!(after.contains("  - extra.yaml"));
    assert!(after.contains("    content: |"));
    assert!(after.contains("      listen 80;"));
    assert!(
        !after.contains("included-conf"),
        "the included file was inlined into the document"
    );
}

/// 4. Feeding the output back in must be a fixpoint.
#[test]
fn remediation_is_idempotent() {
    let (dir, path) = project(&assert_mode("0644"));
    let first = updated(&run(&path)).to_string();

    let second_path = write(dir.path(), "forjar.yaml", &first);
    let out = run(&second_path);
    assert_eq!(out["changed"], false);
    assert_eq!(
        out["remediations_applied"].as_array().expect("array").len(),
        0
    );
    assert_eq!(updated(&out), first);
    assert_eq!(
        out["config_hash_before"], out["config_hash_after"],
        "nothing changed, so the hashes must match"
    );
}

/// 5. `remaining_violations` must come from re-running the rules, not from
///    deleting entries someone believed were fixed. The unfixable `require`
///    rule in the same file has to still be there.
///
///    The `deny` rule is what makes this test kill the bookkeeping mutant.
///    Nothing "fixed" SEC-NO-0777 — it was satisfied as a SIDE EFFECT of
///    writing 0644 — so an implementation that removes the entries it believes
///    it fixed still reports it. Only a fresh evaluation makes it disappear,
///    and only on the resource that actually changed.
#[test]
fn remaining_violations_is_re_evaluated_not_asserted() {
    let policies = format!("{}{REQUIRE_OWNER}{DENY_0777}", assert_mode("0644"));
    let (_dir, path) = project(&policies);
    let out = run(&path);

    let pairs: Vec<(&str, &str)> = out["remaining_violations"]
        .as_array()
        .expect("array")
        .iter()
        .map(|v| {
            (
                v["policy_id"].as_str().unwrap_or_default(),
                v["resource_id"].as_str().unwrap_or_default(),
            )
        })
        .collect();
    assert!(
        !pairs.contains(&("SEC-MODE", "web-conf")),
        "SEC-MODE was fixed on web-conf but is still reported: {pairs:?}"
    );
    // It IS still reported on the resource that lives in the include, which is
    // the point: the list is what the rules say NOW, per resource.
    assert!(
        pairs.contains(&("SEC-MODE", "included-conf")),
        "SEC-MODE vanished from a resource it was never fixed on: {pairs:?}"
    );
    assert!(
        pairs.contains(&("OWN-001", "web-conf")),
        "the unfixable require rule vanished from remaining_violations: {pairs:?}"
    );
    assert!(
        !pairs.contains(&("SEC-NO-0777", "web-conf")),
        "a rule satisfied as a side effect of the correction is still reported — \
         remaining_violations was bookkept, not re-evaluated: {pairs:?}"
    );
    assert!(
        pairs.contains(&("SEC-NO-0777", "included-conf")),
        "the same rule must still hold on the resource that did not change: {pairs:?}"
    );
    let owner = out["remaining_violations"]
        .as_array()
        .expect("array")
        .iter()
        .find(|v| v["policy_id"] == "OWN-001" && v["resource_id"] == "web-conf")
        .expect("OWN-001 present on web-conf");
    assert!(
        owner["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("not the value"),
        "a require rule's reason must say a field name is not a value"
    );
}

/// 6. The verb writes nothing, and every surface says so.
#[test]
fn the_verb_writes_nothing_and_says_so() {
    let (_dir, path) = project(&assert_mode("0644"));
    let before = std::fs::read(&path).expect("read");
    let out = run(&path);
    assert!(
        out["changed"].as_bool().expect("bool"),
        "nothing was computed"
    );
    let after = std::fs::read(&path).expect("read");
    assert_eq!(
        blake3::hash(&before),
        blake3::hash(&after),
        "remediate wrote to the file on disk"
    );

    let verb = forjar::verb::find("remediate").expect("registered");
    assert!(
        verb.effects.read_only(),
        "the verb declares itself mutating"
    );

    let schema = forjar::mcp::export_schema();
    let tool = schema["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|t| t["name"] == "forjar_remediate")
        .expect("forjar_remediate is published");
    assert_eq!(
        tool["annotations"]["readOnlyHint"], true,
        "an agent is told this tool may mutate"
    );
}

/// 7. Every shape the anchor cannot prove must fail CLOSED: the document comes
///    back byte-identical and the reason names the specific cause.
#[test]
fn an_unanchorable_value_fails_closed() {
    // (a) flow style — a first-match text substitution would corrupt this line.
    let dir = tempfile::tempdir().expect("tempdir");
    let flow = format!(
        "version: \"1.0\"\nname: flow\nmachines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\n\
         resources:\n  web-conf: {{type: file, machine: box, path: /etc/w, mode: \"0777\"}}\n\
         policies:\n{}",
        assert_mode("0644")
    );
    let path = write(dir.path(), "forjar.yaml", &flow);
    let out = run(&path);
    assert_eq!(updated(&out), flow, "a flow-style mapping was edited");
    assert!(
        reasons(&out).iter().any(|r| r.contains("flow style")),
        "{:?}",
        reasons(&out)
    );

    // (b) the resource lives in the included file, not in this document.
    let (_dir2, path2) = project(&assert_mode("0644"));
    let out2 = run(&path2);
    let included: Vec<String> = out2["remaining_violations"]
        .as_array()
        .expect("array")
        .iter()
        .filter(|v| v["resource_id"] == "included-conf")
        .map(|v| v["reason"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(included.len(), 1, "the included resource was not reported");
    assert!(
        included[0].contains("extra.yaml"),
        "the reason does not name the include file: {}",
        included[0]
    );
    assert!(
        !updated(&out2).contains("included-conf"),
        "a resource from an include was written into this document"
    );

    // (c) the id is produced by `count:` expansion, so no such key exists in
    //     the source text.
    let counted = format!(
        "version: \"1.0\"\nname: counted\nmachines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\n\
         resources:\n  web-conf:\n    type: file\n    machine: box\n    path: /etc/w{{{{index}}}}\n\
         \x20   mode: \"0777\"\n    count: 2\npolicies:\n{}",
        assert_mode("0644")
    );
    let path3 = write(dir.path(), "counted.yaml", &counted);
    let out3 = run(&path3);
    assert_eq!(updated(&out3), counted, "an expanded resource was edited");
    assert!(
        reasons(&out3).iter().all(|r| !r.is_empty()),
        "an expanded resource was reported with no reason"
    );
}

/// 8. The registry, the schema and the published count moved together.
#[test]
fn the_verb_surface_moved_to_ten() {
    assert_eq!(forjar::verb::verbs().len(), 10);
    let schema = forjar::mcp::export_schema();
    assert_eq!(schema["tool_count"], 10);
    let names: Vec<&str> = schema["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().unwrap_or_default())
        .collect();
    assert!(names.contains(&"forjar_remediate"), "{names:?}");
}
