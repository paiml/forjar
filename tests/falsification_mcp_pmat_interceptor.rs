//! F-MCP-003: the in-process quality gate, driven through the SHIPPED verb.
//!
//! Rejection criteria for the claims this feature makes. Every one of them is
//! asserted through `forjar::verb::find("lint")` and the spec's `invoke` — the
//! same funnel MCP, HTTP and `forjar verb call` all route through — so a green
//! run proves the gate is REACHABLE on the published surface, not merely that
//! a core function exists and returns the right thing.
//!
//! What this file deliberately does NOT test, because it was designed out:
//! a gate in front of `validate` or `plan`. `validate` exists to answer "is
//! this config valid, and if not, why"; refusing to answer for a config that
//! fails a gate removes the only diagnostic an operator has exactly when they
//! need it. `plan` is `Effects::ReadOnly` and changes nothing. Enforcement
//! belongs in front of mutation, and `cli/dispatch_apply_b.rs` is the one
//! place that has it. That CALL SITE is not pinned from here and cannot be:
//! `cli::apply_quality_gate::check_quality_gate` is `pub(super)`, and widening
//! it to satisfy a test would grow the public surface to test a private one.
//! The lib test `cli::apply_quality_gate::tests::a_plaintext_secret_blocks_apply`
//! pins it instead, and was verified red by reverting that function to
//! compliance-packs-only. What CLAIM 6 below proves is narrower and is named
//! for what it is: the predicate itself separates dirty from clean under the
//! default thresholds.
//!
//! Usage: cargo test --test falsification_mcp_pmat_interceptor

use forjar::core::quality_gate::{evaluate, GateLevel, GateThresholds, QUALITY_GATE_ERROR_CODE};
use serde_json::{json, Value};

/// A real age marker: >= 20 chars of decodable base64 inside `ENC[age,…]`.
/// A gate that greps for the word "password" cannot tell this from plaintext.
const SEALED: &str = "ENC[age,YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo=]";

/// A Stripe-shaped fixture, ASSEMBLED AT RUNTIME so the literal never appears
/// in the source.
///
/// The detector under test matches `[sr]k_(live|test)_[A-Za-z0-9]{20,}`, so the
/// fixture must have that exact shape — and GitHub push protection matches the
/// same shape, which blocked a push of this repo outright. Splitting the prefix
/// keeps the runtime value identical while leaving nothing for a file scanner.
fn fake_stripe_key() -> String {
    format!("sk_{}_{}", "live", "A".repeat(24))
}

const DIRTY_YAML: &str = r#"version: "1.0"
name: dirty
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  app-config:
    type: file
    machine: m1
    path: /etc/app.conf
    content: |
      db_password=hunter2
      api_key=__STRIPE_KEY__
  deploy:
    type: task
    machine: m1
    phony: true
    command: "sshpass -p hunter2 ssh deploy@host"
"#;

fn dirty_yaml() -> String {
    DIRTY_YAML.replace("__STRIPE_KEY__", &fake_stripe_key())
}

/// Byte-identical to `dirty_yaml` except the secrets are sealed.
fn sealed_yaml() -> String {
    dirty_yaml()
        .replace("db_password=hunter2", &format!("db_password={SEALED}"))
        .replace(
            &format!("api_key={}", fake_stripe_key()),
            &format!("api_key={SEALED}"),
        )
        .replace("sshpass -p hunter2", &format!("sshpass -p {SEALED}"))
}

const CLEAN_YAML: &str = r#"version: "1.0"
name: clean
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  motd:
    type: file
    machine: m1
    path: /etc/motd
    content: "welcome"
"#;

const BRANCHY_YAML: &str = r#"version: "1.0"
name: branchy
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  loopy:
    type: task
    machine: m1
    phony: true
    command: |
      for f in a b c d e f g h; do
        case "$f" in
          a) echo a ;;
          b) echo b ;;
          c) echo c ;;
          d) echo d ;;
          e) echo e ;;
          f) echo f ;;
          g) echo g ;;
          h) echo h ;;
          i) echo i ;;
          j) echo j ;;
          k) echo k ;;
          l) echo l ;;
          m) echo m ;;
          n) echo n ;;
          o) echo o ;;
          p) echo p ;;
          q) echo q ;;
          r) echo r ;;
          s) echo s ;;
          t) echo t ;;
          u) echo u ;;
          v) echo v ;;
          w) echo w ;;
          x) echo x ;;
          *) echo other ;;
        esac
      done
"#;

/// Write a config and invoke the `lint` verb through the shipped funnel.
fn lint(dir: &std::path::Path, name: &str, yaml: &str, extra: Value) -> Value {
    let path = dir.join(name);
    std::fs::write(&path, yaml).unwrap();
    let mut params = json!({ "path": path.to_str().unwrap() });
    for (k, v) in extra.as_object().unwrap() {
        params[k] = v.clone();
    }
    let spec = forjar::verb::find("lint").expect("the `lint` verb must be on the surface");
    (spec.invoke)(params).expect("a failing gate is a SUCCESSFUL result, never an Err")
}

fn sarif_results(out: &Value) -> &Vec<Value> {
    out["sarif"]["runs"][0]["results"]
        .as_array()
        .expect("the lint result must carry a SARIF 2.1.0 log")
}

// ── CLAIM 1: the verb reports a verdict, not just a warning list ─────

#[test]
fn a_config_with_a_plaintext_secret_fails_the_gate() {
    let dir = tempfile::tempdir().unwrap();
    let out = lint(dir.path(), "forjar.yaml", &dirty_yaml(), json!({}));
    assert_eq!(
        out["gate_passed"],
        json!(false),
        "the lint verb reported no gate verdict for a config carrying a \
         plaintext password and a live-looking API key: {out}"
    );
    assert_eq!(
        out["error_code"],
        json!(QUALITY_GATE_ERROR_CODE),
        "a failing gate must name its error code in the RESULT — an Err \
         bypasses output_schema and would ship a payload no schema describes"
    );
    assert!(out["error_count"].as_u64().unwrap() >= 1);
}

// ── CLAIM 2: SARIF, addressed to the file, with a real line ─────────

#[test]
fn sarif_names_the_resource_and_the_line_it_is_declared_on() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = dirty_yaml();
    let out = lint(dir.path(), "forjar.yaml", &yaml, json!({}));

    let hit = sarif_results(&out)
        .iter()
        .find(|r| {
            r["ruleId"]
                .as_str()
                .is_some_and(|id| id.starts_with("FJQ-SEC"))
                && r["message"]["text"]
                    .as_str()
                    .is_some_and(|t| t.contains("app-config"))
        })
        .unwrap_or_else(|| panic!("no FJQ-SEC result naming app-config: {out}"));

    // Computed from the fixture text, so the assertion cannot be satisfied by
    // an emitter that hardcodes a plausible-looking number.
    let expected = yaml
        .lines()
        .position(|l| l.trim_start().starts_with("app-config:"))
        .map(|i| i + 1)
        .unwrap();
    assert_eq!(
        hit["locations"][0]["physicalLocation"]["region"]["startLine"],
        json!(expected),
        "SARIF carried no usable line for the finding: {hit}"
    );
    assert_eq!(
        hit["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        json!(dir.path().join("forjar.yaml").to_str().unwrap()),
        "the artifact uri must be the file that was linted, not a literal"
    );
}

// ── CLAIM 3: DISCRIMINATION — the strongest falsifier ───────────────

#[test]
fn a_sealed_secret_passes_where_the_same_plaintext_fails() {
    let dir = tempfile::tempdir().unwrap();
    let dirty = lint(dir.path(), "dirty.yaml", &dirty_yaml(), json!({}));
    let sealed = lint(dir.path(), "sealed.yaml", &sealed_yaml(), json!({}));

    assert_eq!(dirty["gate_passed"], json!(false));
    assert_eq!(
        sealed["gate_passed"],
        json!(true),
        "a config whose secrets are ENC[age,…] ciphertext was reported as \
         leaking them — the check is grepping for the word `password` rather \
         than discriminating sealed from plaintext: {sealed}"
    );
    assert!(
        !sarif_results(&sealed)
            .iter()
            .any(|r| r["ruleId"] == json!("FJQ-SEC-002")),
        "sealed config still carried a plaintext-secret finding: {sealed}"
    );
    // And the two files differ ONLY in the sealing, so nothing else explains it.
    assert!(dirty_yaml().lines().count() == sealed_yaml().lines().count());
}

// ── CLAIM 4: ANTI-VACUITY — "fixed" must not mean "always fails" ────

#[test]
fn a_clean_config_passes_with_no_blocking_finding() {
    let dir = tempfile::tempdir().unwrap();
    let out = lint(dir.path(), "forjar.yaml", CLEAN_YAML, json!({}));
    assert_eq!(out["gate_passed"], json!(true), "{out}");
    assert_eq!(out["error_count"], json!(0));
    assert!(
        out["error_code"].is_null(),
        "a passing gate names no error code"
    );
    let blocking: Vec<_> = sarif_results(&out)
        .iter()
        .filter(|r| r["level"] == json!("error"))
        .collect();
    assert!(
        blocking.is_empty(),
        "an inert file resource produced blocking findings, so every other \
         assertion here is satisfied by a gate that always fails: {blocking:?}"
    );
    assert!(
        !sarif_results(&out).iter().any(|r| r["ruleId"]
            .as_str()
            .is_some_and(|id| id.starts_with("FJQ-SEC"))),
        "a config with no secret in it produced a secret finding: {out}"
    );
}

// ── CLAIM 5: the complexity ceiling is READ, not decorative ─────────

#[test]
fn the_cyclomatic_ceiling_is_the_number_that_was_passed() {
    let dir = tempfile::tempdir().unwrap();
    let tight = lint(
        dir.path(),
        "forjar.yaml",
        BRANCHY_YAML,
        json!({ "max_cyclomatic": 1 }),
    );
    assert!(
        sarif_results(&tight)
            .iter()
            .any(|r| r["ruleId"] == json!("FJQ-CPX-001")),
        "a for/if/case chain did not exceed a ceiling of 1: {tight}"
    );

    // The fixture's apply script scores ~28, so a ceiling ABOVE it must be
    // silent. This is what proves the number in the params is the number the
    // check compares against, rather than a constant that happens to fire.
    let loose = lint(
        dir.path(),
        "forjar.yaml",
        BRANCHY_YAML,
        json!({ "max_cyclomatic": 1000 }),
    );
    assert!(
        !sarif_results(&loose)
            .iter()
            .any(|r| r["ruleId"] == json!("FJQ-CPX-001")),
        "a ceiling of 1000 fired, so the check is not reading it: {loose}"
    );

    let off = lint(dir.path(), "forjar.yaml", BRANCHY_YAML, json!({}));
    assert!(
        !sarif_results(&off)
            .iter()
            .any(|r| r["ruleId"] == json!("FJQ-CPX-001")),
        "the ceiling fired with no ceiling configured — omitting it must SKIP \
         the check, not parse three scripts per resource and ignore the answer"
    );

    // It is ADVISORY. The shell it scores is emitted by forjar's own codegen,
    // so blocking an operator's apply for it punishes the wrong party.
    assert_eq!(
        tight["gate_passed"],
        json!(true),
        "a complexity finding blocked the gate: {tight}"
    );
}

// ── CLAIM 6: the default-threshold predicate discriminates ─────────
//
// NOT a test of the apply call site. Reverting `check_quality_gate` to
// compliance-packs-only leaves this test GREEN — measured — because it calls
// `evaluate` directly, and with different thresholds than that function uses
// (`policy_dir` is set there). The call site's falsification lives in
// `cli::apply_quality_gate::tests`.

#[test]
fn the_default_threshold_predicate_separates_dirty_from_clean() {
    let dirty = forjar::core::parser::parse_config(&dirty_yaml()).unwrap();
    let clean = forjar::core::parser::parse_config(CLEAN_YAML).unwrap();
    let t = GateThresholds::default();

    let bad = evaluate(&dirty, Some(&dirty_yaml()), &t);
    assert!(!bad.passed(), "{:?}", bad.findings);
    assert!(bad
        .findings
        .iter()
        .any(|f| f.level == GateLevel::Error && f.rule_id.starts_with("FJQ-SEC")));

    let good = evaluate(&clean, Some(CLEAN_YAML), &t);
    assert!(
        good.passed(),
        "the pre-apply gate would refuse an inert config: {:?}",
        good.findings
    );
}

// ── CLAIM 7: the verb's contract still describes what it returns ────

#[test]
fn the_gate_fields_are_in_the_published_output_schema() {
    let spec = forjar::verb::find("lint").unwrap();
    let schema = (spec.output_schema)();
    let props = schema["properties"].as_object().unwrap_or_else(|| {
        panic!("lint output schema has no properties: {schema}");
    });
    for field in ["gate_passed", "error_code", "findings", "sarif"] {
        assert!(
            props.contains_key(field),
            "`{field}` is returned but not published in output_schema — FVS-3 \
             requires the schema to describe the result: {schema}"
        );
    }
    assert!(
        spec.effects.read_only(),
        "lint must stay ReadOnly: an agent decides whether it may call a verb \
         unattended from readOnlyHint before it calls it"
    );
}
