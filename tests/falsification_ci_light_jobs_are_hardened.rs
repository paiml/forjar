//! The three light PR jobs (lockfile, examples-validate, file-health) read
//! nothing but the repository, so each carries `permissions: contents: read`
//! and checks out with `persist-credentials: false`.
//!
//! The test judges the parsed YAML and compares exact values. It does NOT
//! interpret shell: the two scripts that carry this PR's guarantees (the
//! examples-validate guard step, and gate's "Check required jobs" step) are
//! pinned verbatim below, byte for byte after YAML parsing, so any edit to
//! either script needs an edit to this file, which a reviewer sees.
//!
//! - job `permissions` are exactly `contents: read`; a workflow-level
//!   `permissions` is absent or no wider than read;
//! - no `continue-on-error` key on a light job, on any of its steps, on `gate`
//!   or on any `gate` step; a light job's `if` keeps its YAML value (absent
//!   stays absent; a bool or number fails); `gate`'s `if` is `always()`;
//! - every checkout step (`uses` matched case-insensitively) sets
//!   `persist-credentials` to the YAML bool false, and there is exactly one;
//! - `gate` needs both ci.yml jobs; its one step has exactly the keys
//!   name/env/run, with `env` pinned and `run` equal to `GATE_RUN`;
//! - the guard step has exactly the keys name/run and `run` equal to
//!   `GUARDS_RUN`, which contains the `--test` line for this target.
//!
//! mutation: each row of `mutants()` edits the workflow text in memory and the
//! checker must name the violation; the unmutated trees must be clean.

use serde_yaml_ng::Value;

const TARGET: &str = "falsification_ci_light_jobs_are_hardened";
const CODE_IF: &str = "needs.classify.outputs.code == 'true'";
const GUARD_NAME: &str =
    "Integration guards (examples, orphans, undo, quorum gates, nightly, version)";

fn read(rel: &str) -> String {
    let p = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p}: {e}"))
}

fn parse(text: &str) -> Value {
    serde_yaml_ng::from_str(text).expect("workflow parses")
}

/// The examples-validate guard step's `run:`, pinned verbatim.
const GUARDS_RUN: &str = r####"cargo test --locked \
--test examples_validate \
--test falsification_no_orphaned_source_files \
--test falsification_722_cache_key_arch \
--test falsification_contract_citations_resolve \
--test falsification_714_unsafe_libyaml_ignore_is_bounded \
--test falsification_chrono_0543_ignore_is_bounded \
--test falsification_drift_unmeasured_is_not_drift \
--test e2e_transport_absence_t \
--test falsification_undo_actually_undoes \
--test falsification_undo_state_dir_interlock \
--test falsification_hosted_jobs_do_not_cache_target \
--test falsification_no_tracked_file_is_gitignored \
--test falsification_quorum_gate_reads_the_pushed_ref \
--test falsification_quorum_gate_reads_the_receipt_store \
--test falsification_quorum_gate_bypasses \
--test falsification_quorum_gate_exact_identity \
--test falsification_nightly_gate_is_tag_vs_head \
--test falsification_nightly_windows_openssl_perl \
--test falsification_nightly_release_can_write_contents \
--test falsification_683_cross_mounts_under_work_root \
--test falsification_sudo_transport_closefrom_emulated \
--test falsification_version_matches_manifest \
--test falsification_pr_jobs_name_no_arm64_or_host \
--test falsification_390_one_stream_reporter \
--test falsification_release_workflow_shape \
--test falsification_untyped_error_sites_ratchet \
--test falsification_e02_controlmaster_before_the_drift_gate \
--test falsification_replace_running_binary \
--test falsification_io_hash_answers_for_its_machine \
--test falsification_budget_dry_run \
--test falsification_ci_wires_the_858_falsifiers \
--test falsification_ci_light_jobs_are_hardened
"####;

/// gate's "Check required jobs" `run:`, pinned verbatim.
const GATE_RUN: &str = r####"# PMAT-237: a job may be SKIPPED only because the change cannot reach
# it. The gate re-reads the class rather than trusting the skip: if
# the change IS code and a heavy job did not run, that is a hole, not
# a saving, and this gate refuses it by name.
# Written with explicit `if`, not `[ … ] && return 0`: an AND-list
# that fails at statement level exits the step under `bash -e` before
# the message is printed, and a gate that fails silently is half a
# gate.
# THE CLASS ITSELF MUST HAVE BEEN MEASURED (PMAT-237).
#
# If `classify` fails or is cancelled its output is EMPTY, every heavy
# job's `if` is false so they all skip, and a gate that read an empty
# class as "not code" would pass over a change nothing tested. A
# review lane found exactly that. An unmeasured class is code.
if [ "${{ needs.classify.result }}" != "success" ]; then
  echo "::error::classify did not run (${{ needs.classify.result }}), so nothing knows what this change can break"
  exit 1
fi
if [ -z "$CODE" ]; then
  echo "::error::the class is empty — treating it as code and refusing"
  exit 1
fi
# PMAT-542: the SELECTION must have been measured too. The classifier
# prints `none` and never the empty string, exactly so that "nothing
# was selected" and "nothing computed a selection" cannot look the
# same here. An empty value is the second one.
if [ -z "$GATES" ]; then
  echo "::error::the gate selection is empty — nothing measured which gates this change can move, and an unmeasured selection is not a selection"
  exit 1
fi
if [ -z "$ROADMAPS" ]; then
  echo "::error::the roadmaps class is empty - treating it as roadmaps and refusing"
  exit 1
fi

ok() {
  case "$2" in
    success) return 0 ;;
    skipped)
      if [ "$CODE" != "true" ]; then
        echo "$1: skipped — the change touches nothing it reads"
        return 0
      fi
      echo "::error::$1 was skipped on a change the classifier called code"
      return 1
      ;;
    *)
      echo "::error::$1: $2"
      return 1
      ;;
  esac
}
ok_roadmaps() {
  case "$2" in
    success) return 0 ;;
    skipped)
      if [ "$ROADMAPS" != "true" ]; then
        echo "$1: skipped - the change touches nothing it reads"
        return 0
      fi
      echo "::error::$1 was skipped on a change the classifier called roadmaps"
      return 1
      ;;
    *)
      echo "::error::$1: $2"
      return 1
      ;;
  esac
}
ok "ci" "${{ needs.ci.result }}" || exit 1
if [ "${{ needs.lockfile.result }}" != "success" ]; then
  echo "lockfile preflight failed: ${{ needs.lockfile.result }}"
  exit 1
fi
ok "examples-validate" "${{ needs.examples-validate.result }}" || exit 1
ok "no-default-features" "${{ needs.no-default-features.result }}" || exit 1
ok "doctests" "${{ needs.doctests.result }}" || exit 1
ok "dogfood-guards" "${{ needs.dogfood-guards.result }}" || exit 1
ok_roadmaps "alias-map-guard" "${{ needs.alias-map-guard.result }}" || exit 1
if [ "${{ needs.spot-labels-guard.result }}" != "success" ]; then
  echo "::error::spot-labels-guard: ${{ needs.spot-labels-guard.result }}"
  exit 1
fi
# PMAT-542: dogfood-surface skips on its OWN condition, which is
# `gates != none` rather than the class. Re-read the selection here
# for the same reason `ok` re-reads the class: a skip is a claim until
# the thing that decided it says so.
case "${{ needs.dogfood-surface.result }}" in
  success) echo "dogfood-surface: ran gates $GATES" ;;
  skipped)
    if [ "$CODE" != "true" ]; then
      echo "dogfood-surface: skipped — the change touches nothing it reads"
    elif [ "$GATES" = "none" ]; then
      echo "dogfood-surface: NOT-SELECTED — this change can move neither gate C nor gate D; both are measured in full by make dogfood-release before the tag"
    else
      echo "::error::dogfood-surface was skipped while the selection says $GATES"
      exit 1
    fi
    ;;
  *)
    echo "::error::dogfood-surface: ${{ needs.dogfood-surface.result }}"
    exit 1
    ;;
esac
echo "All required jobs passed"
"####;

fn steps(job: &Value) -> Vec<&Value> {
    job["steps"]
        .as_sequence()
        .map_or(vec![], |s| s.iter().collect())
}

fn keys(step: &Value) -> Vec<String> {
    step.as_mapping().map_or(vec![], |m| {
        m.keys()
            .filter_map(|k| k.as_str().map(String::from))
            .collect()
    })
}

fn wide_permissions(doc: &Value) -> bool {
    match doc.get("permissions") {
        None => false,
        Some(Value::String(s)) => s != "read-all",
        Some(Value::Mapping(m)) => m
            .values()
            .any(|v| !matches!(v.as_str(), Some("read" | "none"))),
        Some(_) => true,
    }
}

fn check_job(doc: &Value, name: &str, want_if: Option<&str>, out: &mut Vec<String>) {
    let job = &doc["jobs"][name];
    let mut want = serde_yaml_ng::Mapping::new();
    want.insert("contents".into(), "read".into());
    if job["permissions"] != Value::Mapping(want) {
        out.push(format!("{name}: permissions"));
    }
    if job.get("continue-on-error").is_some() {
        out.push(format!("{name}: continue-on-error (job)"));
    }
    if steps(job)
        .iter()
        .any(|s| s.get("continue-on-error").is_some())
    {
        out.push(format!("{name}: continue-on-error (step)"));
    }
    if job.get("if") != want_if.map(|s| Value::String(s.into())).as_ref() {
        out.push(format!("{name}: if"));
    }
    let checkouts: Vec<&Value> = steps(job)
        .into_iter()
        .filter(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.to_ascii_lowercase().starts_with("actions/checkout@"))
        })
        .collect();
    if checkouts.len() != 1 {
        out.push(format!("{name}: checkout count {}", checkouts.len()));
    }
    if checkouts
        .iter()
        .any(|s| s["with"]["persist-credentials"] != Value::Bool(false))
    {
        out.push(format!("{name}: persist-credentials"));
    }
}

/// The alias step must be a bare `name` + `run`: an `if:` or `continue-on-error:`
/// on it would let a red alias test report a green job.
fn check_alias_step(job: &Value, out: &mut Vec<String>) {
    let st = steps(job);
    if st.len() != 2 || keys(st[1]) != ["name", "run"] {
        out.push("alias-map-guard: step shape".into());
    }
}

fn check_guard_step(ev: &Value, out: &mut Vec<String>) {
    let guards: Vec<&Value> = steps(ev)
        .into_iter()
        .filter(|s| s["name"].as_str() == Some(GUARD_NAME))
        .collect();
    if guards.len() != 1 {
        out.push(format!(
            "examples-validate: guard step count {}",
            guards.len()
        ));
        return;
    }
    if keys(guards[0]) != ["name", "run"] {
        out.push("examples-validate: guard step keys".into());
    }
    if guards[0]["run"].as_str() != Some(GUARDS_RUN) {
        out.push("examples-validate: guard run differs from the pinned script".into());
    }
}

fn check_gate(gate: &Value, out: &mut Vec<String>) {
    let needs = gate["needs"].as_sequence().cloned().unwrap_or_default();
    for j in ["lockfile", "examples-validate", "alias-map-guard"] {
        if !needs.iter().any(|n| n.as_str() == Some(j)) {
            out.push(format!("gate: does not need {j}"));
        }
    }
    if gate.get("if") != Some(&Value::String("always()".into())) {
        out.push("gate: if".into());
    }
    if gate.get("continue-on-error").is_some() {
        out.push("gate: continue-on-error".into());
    }
    let st = steps(gate);
    if st.len() != 1 {
        out.push(format!("gate: step count {}", st.len()));
        return;
    }
    if keys(st[0]) != ["name", "env", "run"] {
        out.push("gate: step keys".into());
    }
    let mut env = serde_yaml_ng::Mapping::new();
    env.insert("CODE".into(), "${{ needs.classify.outputs.code }}".into());
    env.insert("GATES".into(), "${{ needs.classify.outputs.gates }}".into());
    env.insert(
        "ROADMAPS".into(),
        "${{ needs.classify.outputs.roadmaps }}".into(),
    );
    if st[0]["env"] != Value::Mapping(env) {
        out.push("gate: step env".into());
    }
    if st[0]["run"].as_str() != Some(GATE_RUN) {
        out.push("gate: run differs from the pinned script".into());
    }
}

fn violations(ci: &str, lint: &str) -> Vec<String> {
    let (ci, lint) = (parse(ci), parse(lint));
    let mut out = Vec::new();
    check_job(&ci, "lockfile", None, &mut out);
    check_job(&ci, "examples-validate", Some(CODE_IF), &mut out);
    check_job(&ci, "alias-map-guard", Some(ROADMAPS_IF), &mut out);
    check_alias_step(&ci["jobs"]["alias-map-guard"], &mut out);
    check_job(&lint, "file-health", None, &mut out);
    for (file, doc) in [("ci.yml", &ci), ("lint.yml", &lint)] {
        if wide_permissions(doc) {
            out.push(format!("{file}: workflow permissions"));
        }
    }
    check_guard_step(&ci["jobs"]["examples-validate"], &mut out);
    check_gate(&ci["jobs"]["gate"], &mut out);
    out
}

#[test]
fn the_light_jobs_are_hardened() {
    let v = violations(
        &read(".github/workflows/ci.yml"),
        &read(".github/workflows/lint.yml"),
    );
    assert!(v.is_empty(), "{v:#?}");
}

/// The pinned literal itself runs this target as a plain `--test` line of its
/// `cargo test` list, and reads both results with `exit 1` on failure.
#[test]
fn the_pinned_literals_say_what_this_pr_relies_on() {
    let lines: Vec<&str> = GUARDS_RUN.lines().collect();
    assert_eq!(lines[0], "cargo test --locked \\");
    assert!(
        lines.contains(&format!("--test {TARGET}").as_str()),
        "target line missing"
    );
    assert!(GATE_RUN.contains("if [ \"${{ needs.lockfile.result }}\" != \"success\" ]; then\n  echo \"lockfile preflight failed: ${{ needs.lockfile.result }}\"\n  exit 1\nfi\n"));
    assert!(GATE_RUN.contains(
        "ok \"examples-validate\" \"${{ needs.examples-validate.result }}\" || exit 1\n"
    ));
    assert!(GATE_RUN.contains(
        "ok_roadmaps \"alias-map-guard\" \"${{ needs.alias-map-guard.result }}\" || exit 1\n"
    ));
}

const SHA: &str = "3d3c42e5aac5ba805825da76410c181273ba90b1";

/// (name, file, old text, new text, replace every occurrence, expected violation)
type Mutant = (
    &'static str,
    &'static str,
    String,
    String,
    bool,
    &'static str,
);

const ROADMAPS_IF: &str = "needs.classify.outputs.roadmaps == 'true'";

fn mutants() -> Vec<Mutant> {
    let lock_name = "    name: lockfile-preflight\n";
    let verify = "      - name: Verify Cargo.lock is committed and current\n";
    let ex = format!("      - uses: Actions/checkout@{SHA}\n");
    let guards = format!("      - name: {GUARD_NAME}\n");
    let host = "          --test falsification_ci_wires_the_858_falsifiers";
    let last = format!("{host} \\\n          --test {TARGET}");
    let first = "          cargo test --locked \\\n";
    let gate_step = "      - name: Check required jobs\n";
    let lock_if = "          if [ \"${{ needs.lockfile.result }}\" != \"success\" ]; then";
    let ex_ok =
        "          ok \"examples-validate\" \"${{ needs.examples-validate.result }}\" || exit 1";
    let classify = "          if [ \"${{ needs.classify.result }}\" != \"success\" ]; then";
    let m = |n, f, o: &str, w: String, all, v| -> Mutant { (n, f, o.to_string(), w, all, v) };
    let (ci, li) = ("ci", "lint");
    let (g, s) = ("guard run differs", "gate: run differs");
    vec![
        m("lockfile if: false (bool)", ci, lock_name, format!("{lock_name}    if: false\n"), false, "lockfile: if"),
        m("file-health if: 0 (number)", li, "  file-health:\n    timeout-minutes: 15\n", "  file-health:\n    timeout-minutes: 15\n    if: 0\n".into(), false, "file-health: if"),
        m("examples-validate if loosened to bool", ci, "    if: needs.classify.outputs.code == 'true'\n    needs: classify\n    name: examples-validate", "    if: true\n    needs: classify\n    name: examples-validate".into(), false, "examples-validate: if"),
        m("mixed-case second checkout", ci, verify, format!("{ex}{verify}"), false, "checkout count 2"),
        m("file-health persist-credentials as string", li, "persist-credentials: false", "persist-credentials: \"false\"".into(), false, "file-health: persist-credentials"),
        m("lockfile persist-credentials true", ci, "        with:\n          persist-credentials: false\n      - name: Verify Cargo.lock", "        with:\n          persist-credentials: true\n      - name: Verify Cargo.lock".into(), false, "lockfile: persist-credentials"),
        m("file-health permissions widened", li, "      contents: read\n", "      contents: write\n".into(), false, "file-health: permissions"),
        m("workflow-level write-all", ci, "\nconcurrency:\n", "\npermissions: write-all\n\nconcurrency:\n".into(), false, "ci.yml: workflow permissions"),
        m("continue-on-error on a job", ci, lock_name, format!("{lock_name}    continue-on-error: true\n"), false, "lockfile: continue-on-error (job)"),
        m("continue-on-error on a step", ci, verify, format!("{verify}        continue-on-error: true\n"), false, "lockfile: continue-on-error (step)"),
        m("alias-map-guard job continue-on-error", ci, "  alias-map-guard:\n    timeout-minutes: 30\n", "  alias-map-guard:\n    timeout-minutes: 30\n    continue-on-error: true\n".into(), false, "alias-map-guard: continue-on-error (job)"),
        m("alias-map-guard step continue-on-error", ci, "      - name: Alias map guard\n", "      - name: Alias map guard\n        continue-on-error: true\n".into(), false, "alias-map-guard: continue-on-error (step)"),
        m("alias-map-guard step if: false", ci, "      - name: Alias map guard\n", "      - name: Alias map guard\n        if: false\n".into(), false, "alias-map-guard: step shape"),
        m("alias-map-guard if loosened", ci, "    if: needs.classify.outputs.roadmaps == 'true'\n", "    if: true\n".into(), false, "alias-map-guard: if"),
        m("gate drops lockfile from needs", ci, "needs: [classify, ci, lockfile, ", "needs: [classify, ci, ".into(), false, "gate: does not need lockfile"),
        m("gate drops alias-map-guard from needs", ci, ", alias-map-guard,", ",".into(), false, "gate: does not need alias-map-guard"),
        m("gate if: false", ci, "    if: always()\n", "    if: false\n".into(), false, "gate: if"),
        m("gate job continue-on-error", ci, "    if: always()\n", "    if: always()\n    continue-on-error: true\n".into(), false, "gate: continue-on-error"),
        m("gate step continue-on-error", ci, gate_step, format!("{gate_step}        continue-on-error: true\n"), false, "gate: step keys"),
        m("gate step if: false", ci, gate_step, format!("{gate_step}        if: false\n"), false, "gate: step keys"),
        m("gate step env changed", ci, "CODE: ${{ needs.classify.outputs.code }}", "CODE: 'true'".into(), false, "gate: step env"),
        m("guard step if: false", ci, &guards, format!("{guards}        if: false\n"), false, "guard step keys"),
        m("guard step continue-on-error", ci, &guards, format!("{guards}        continue-on-error: true\n"), false, "examples-validate: continue-on-error (step)"),
        m("target removed from the list", ci, &last, host.into(), false, g),
        m("target in an inline # comment", ci, &last, format!("{host} # --test {TARGET}"), false, g),
        m("target in a full-line comment", ci, &last, format!("{host}\n          # --test {TARGET}"), false, g),
        m("target renamed (prefix match)", ci, &format!("--test {TARGET}"), format!("--test {TARGET}_noop"), false, g),
        m("target passed to echo, not cargo", ci, &last, format!("{host}\n          echo --test {TARGET}"), false, g),
        m("cargo test --no-run", ci, first, "          cargo test --locked --no-run \\\n".into(), false, g),
        m("target masked by || true", ci, &last, format!("{host} \\\n          --test {TARGET} || true"), false, g),
        m("newline after ||", ci, &last, format!("{host} \\\n          --test {TARGET} ||\n          true"), false, g),
        m("backslash-quote trick", ci, first, "          cargo test --locked \\\n          --test \"x\\\" \\\n".into(), false, g),
        m("true || prefix", ci, first, "          true || cargo test --locked \\\n".into(), false, g),
        m("if false wrap", ci, first, "          if false; then cargo test --locked \\\n".into(), false, g),
        m("set +e in the guard step", ci, first, format!("          set +e\n{first}"), false, g),
        m("lockfile result only in a comment", ci, lock_if, "          if true; then # [ \"${{ needs.lockfile.result }}\" != \"success\" ]".into(), false, s),
        m("examples-validate result only in a comment", ci, ex_ok, "          true # ok examples-validate".into(), false, s),
        m("examples-validate exit 1 deleted", ci, ex_ok, "          ok \"examples-validate\" \"${{ needs.examples-validate.result }}\"".into(), false, s),
        m("alias-map-guard result only in a comment", ci, "          ok_roadmaps \"alias-map-guard\" \"${{ needs.alias-map-guard.result }}\" || exit 1", "          true # ok_roadmaps alias-map-guard".into(), false, s),
        m("set +e in gate", ci, classify, format!("          set +e\n{classify}"), false, s),
        m("gate echo All required jobs passed moved before checks", ci, classify, format!("          echo All required jobs passed\n{classify}"), false, s),
    ]
}

#[test]
fn every_mutant_is_refused() {
    let (ci, lint) = (
        read(".github/workflows/ci.yml"),
        read(".github/workflows/lint.yml"),
    );
    let table = mutants();
    assert!(table.len() >= 30, "{} mutants", table.len());
    for (name, file, old, new, all, want) in table {
        let src = if file == "ci" { &ci } else { &lint };
        let n = src.matches(old.as_str()).count();
        assert!(
            n >= 1 && (all || n == 1),
            "{name}: old text occurs {n} times"
        );
        let mutated = src.replace(old.as_str(), &new);
        let (c, l) = if file == "ci" {
            (mutated.as_str(), lint.as_str())
        } else {
            (ci.as_str(), mutated.as_str())
        };
        let got = violations(c, l);
        assert!(
            got.iter().any(|g| g.contains(want)),
            "{name}: wanted `{want}`, got {got:?}"
        );
    }
}

/// With exact equality any change to a pinned script is red, so the only
/// controls are YAML-level: a YAML comment, and the words continue-on-error in
/// another step's `name:`.
#[test]
fn yaml_level_edits_next_to_real_checks_are_accepted() {
    let (ci, lint) = (
        read(".github/workflows/ci.yml"),
        read(".github/workflows/lint.yml"),
    );
    let edits: [(&str, String); 2] = [
        (
            "      - name: Integration guards (examples,",
            "      # a YAML comment: continue-on-error is not set here\n      - name: Integration guards (examples,".into(),
        ),
        (
            "      - name: Verify Cargo.lock is committed and current\n",
            "      - name: Verify Cargo.lock is committed and current (no continue-on-error)\n".into(),
        ),
    ];
    for (old, new) in edits {
        assert_eq!(ci.matches(old).count(), 1, "{old}");
        assert!(
            violations(&ci.replace(old, &new), &lint).is_empty(),
            "{old}"
        );
    }
}
