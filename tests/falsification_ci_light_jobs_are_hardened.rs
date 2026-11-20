//! The three light PR jobs (lockfile, examples-validate, file-health) read
//! nothing but the repository, so each carries `permissions: contents: read`
//! and checks out with `persist-credentials: false`. `violations` judges the
//! parsed YAML, and shell text only through `commands` (never a comment, never
//! a re-serialised string):
//! - job `permissions` are exactly `contents: read`; a workflow-level
//!   `permissions` is absent or no wider than read;
//! - no `continue-on-error` key on a light job, on any of its steps, on `gate`
//!   or on any `gate` step;
//! - a light job's `if` is the YAML value it already had (absent stays absent;
//!   a bool or number is a failure); `gate`'s `if` is `always()` and no `gate`
//!   step has an `if`;
//! - every checkout step (`uses` matched case-insensitively) sets
//!   `persist-credentials` to the YAML bool false, and there is exactly one;
//! - `gate` needs both ci.yml jobs and reads both results as the workflow
//!   reads them today, on command text after comment stripping;
//! - examples-validate runs this target as a `--test <target>` argument of a
//!   `cargo test` command (no `--no-run`, no `--list`, not masked by `|| true`),
//!   in a step with no `if` and no `continue-on-error`.
//!
//! mutation: each row of `mutants()` edits the workflow text in memory and the
//! checker must name the violation; the unmutated trees must be clean.

use serde_yaml_ng::Value;

const TARGET: &str = "falsification_ci_light_jobs_are_hardened";
const CODE_IF: &str = "needs.classify.outputs.code == 'true'";
const LOCK: &str = "${{ needs.lockfile.result }}";
const EXAMPLES: &str = "${{ needs.examples-validate.result }}";

fn read(rel: &str) -> String {
    let p = format!("{}/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p}: {e}"))
}

fn parse(text: &str) -> Value {
    serde_yaml_ng::from_str(text).expect("workflow parses")
}

/// One shell command: its words (quotes removed, comments gone) and the
/// separator that preceded it (`""` newline/start, `;`, `&&`, `||`).
struct Cmd {
    sep: &'static str,
    words: Vec<String>,
}

/// Incremental reader state for `commands`.
#[derive(Default)]
struct Lexer {
    out: Vec<Cmd>,
    words: Vec<String>,
    cur: String,
    in_word: bool,
    sep: &'static str,
}

impl Lexer {
    fn end_word(&mut self) {
        if self.in_word {
            self.words.push(std::mem::take(&mut self.cur));
            self.in_word = false;
        }
    }

    fn end_cmd(&mut self, next_sep: &'static str) {
        self.end_word();
        if !self.words.is_empty() {
            self.out.push(Cmd {
                sep: self.sep,
                words: std::mem::take(&mut self.words),
            });
        }
        self.sep = next_sep;
    }

    fn push(&mut self, c: char) {
        self.cur.push(c);
        self.in_word = true;
    }
}

/// The separator a character starts outside quotes, if any, and its width.
fn separator(c: char, next: Option<char>) -> Option<(&'static str, usize)> {
    match (c, next) {
        ('\n', _) => Some(("", 1)),
        (';', _) => Some((";", 1)),
        ('&', Some('&')) => Some(("&&", 2)),
        ('|', Some('|')) => Some(("||", 2)),
        _ => None,
    }
}

/// Index of the last character of a comment that starts at `i`.
fn comment_end(chars: &[char], mut i: usize) -> usize {
    while i + 1 < chars.len() && chars[i + 1] != '\n' {
        i += 1;
    }
    i
}

/// Turn a `run:` block into commands: `\`-newline continuations joined
/// (including a final line ending in `\`), unquoted `#` comments stripped at
/// the start of a word (inline or full line), quotes honoured, split on
/// newline, `;`, `&&` and `||`.
fn commands(run: &str) -> Vec<Cmd> {
    let text = run.replace("\\\n", "");
    let chars: Vec<char> = text.trim_end().trim_end_matches('\\').chars().collect();
    let (mut lx, mut quote, mut i) = (Lexer::default(), None::<char>, 0);
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                lx.cur.push(c);
            }
        } else if c == '\'' || c == '"' {
            quote = Some(c);
            lx.in_word = true;
        } else if c == '#' && !lx.in_word {
            i = comment_end(&chars, i);
        } else if c == ' ' || c == '\t' {
            lx.end_word();
        } else if let Some((sep, width)) = separator(c, chars.get(i + 1).copied()) {
            lx.end_cmd(sep);
            i += width - 1;
        } else {
            lx.push(c);
        }
        i += 1;
    }
    lx.end_cmd("");
    lx.out
}

fn steps(job: &Value) -> Vec<&Value> {
    job["steps"]
        .as_sequence()
        .map_or(vec![], |s| s.iter().collect())
}

fn run_commands(job: &Value) -> Vec<Cmd> {
    steps(job)
        .iter()
        .flat_map(|s| commands(s["run"].as_str().unwrap_or("")))
        .collect()
}

/// A `cargo test` command (no `--no-run`, no `--list`) that passes
/// `--test <TARGET>`.
fn is_target_cmd(c: &Cmd) -> bool {
    let has = |s: &str| c.words.iter().any(|x| x == s);
    c.words.starts_with(&["cargo".into(), "test".into()])
        && !has("--no-run")
        && !has("--list")
        && c.words
            .windows(2)
            .any(|p| p[0] == "--test" && p[1] == TARGET)
}

/// `|| true`, `|| :` or `|| exit 0`: the command's failure is swallowed.
fn swallows_failure(c: &Cmd) -> bool {
    c.sep == "||" && (matches!(c.words[0].as_str(), "true" | ":") || c.words == ["exit", "0"])
}

/// Some(masked) when the step runs the target; `masked` is true when a
/// failure of that command is swallowed.
fn runs_target(step: &Value) -> Option<bool> {
    let cmds = commands(step["run"].as_str().unwrap_or(""));
    let i = cmds.iter().position(is_target_cmd)?;
    Some(cmds.get(i + 1).is_some_and(swallows_failure))
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

fn check_gate(gate: &Value, out: &mut Vec<String>) {
    let needs = gate["needs"].as_sequence().cloned().unwrap_or_default();
    for j in ["lockfile", "examples-validate"] {
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
    if steps(gate)
        .iter()
        .any(|s| s.get("if").is_some() || s.get("continue-on-error").is_some())
    {
        out.push("gate: step if or continue-on-error".into());
    }
    let cmds = run_commands(gate);
    // `if [ "${{ needs.lockfile.result }}" != "success" ]; then ... exit 1`
    let lock = cmds.iter().any(|c| {
        let w: Vec<&str> = c
            .words
            .iter()
            .map(String::as_str)
            .skip_while(|x| *x == "if")
            .collect();
        w == ["[", LOCK, "!=", "success", "]"]
    });
    if !lock {
        out.push("gate: does not read lockfile result".into());
    }
    // `ok "examples-validate" "${{ needs.examples-validate.result }}" || exit 1`
    let ex = cmds.windows(2).any(|p| {
        p[0].words == ["ok", "examples-validate", EXAMPLES]
            && p[1].sep == "||"
            && p[1].words == ["exit", "1"]
    });
    if !ex {
        out.push("gate: does not read examples-validate result".into());
    }
}

fn violations(ci: &str, lint: &str) -> Vec<String> {
    let (ci, lint) = (parse(ci), parse(lint));
    let mut out = Vec::new();
    check_job(&ci, "lockfile", None, &mut out);
    check_job(&ci, "examples-validate", Some(CODE_IF), &mut out);
    check_job(&lint, "file-health", None, &mut out);
    for (file, doc) in [("ci.yml", &ci), ("lint.yml", &lint)] {
        if wide_permissions(doc) {
            out.push(format!("{file}: workflow permissions"));
        }
    }
    check_gate(&ci["jobs"]["gate"], &mut out);
    let ev = &ci["jobs"]["examples-validate"];
    let runs: Vec<Option<bool>> = steps(ev)
        .into_iter()
        .filter(|s| s.get("if").is_none() && s.get("continue-on-error").is_none())
        .map(runs_target)
        .collect();
    match runs.iter().flatten().next() {
        None => out.push("examples-validate: does not run the target".into()),
        Some(true) => out.push("examples-validate: target masked by || true".into()),
        Some(false) => {}
    }
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

fn mutants() -> Vec<Mutant> {
    let lock_name = "    name: lockfile-preflight\n";
    let verify = "      - name: Verify Cargo.lock is committed and current\n";
    let ex = format!("      - uses: Actions/checkout@{SHA}\n");
    let guards = "      - name: Integration guards (examples, orphans, undo, quorum gates, nightly, version)\n";
    let host = "          --test falsification_pr_jobs_name_no_arm64_or_host";
    let last = format!("{host} \\\n          --test {TARGET}");
    let gate_step = "      - name: Check required jobs\n";
    let lock_if = "          if [ \"${{ needs.lockfile.result }}\" != \"success\" ]; then";
    let ex_ok =
        "          ok \"examples-validate\" \"${{ needs.examples-validate.result }}\" || exit 1";
    let m = |n, f, o: &str, w: String, all, v| -> Mutant { (n, f, o.to_string(), w, all, v) };
    let ci = "ci";
    let li = "lint";
    let run_t = "does not run the target";
    vec![
        m("B1 lockfile if: false (bool)", ci, lock_name, format!("{lock_name}    if: false\n"), false, "lockfile: if"),
        m("B1 file-health if: 0 (number)", li, "  file-health:\n    timeout-minutes: 15\n", "  file-health:\n    timeout-minutes: 15\n    if: 0\n".into(), false, "file-health: if"),
        m("B1 examples-validate if loosened to bool", ci, "    if: needs.classify.outputs.code == 'true'\n    needs: classify\n    name: examples-validate", "    if: true\n    needs: classify\n    name: examples-validate".into(), false, "examples-validate: if"),
        m("B2 target commented out (full line)", ci, &last, format!("{host}\n          # --test {TARGET}"), false, run_t),
        m("B2 target commented out (inline #)", ci, &last, format!("{host} # --test {TARGET}"), false, run_t),
        m("B2 target renamed (prefix match)", ci, &format!("--test {TARGET}"), format!("--test {TARGET}_noop"), false, run_t),
        m("B2 target step disabled by if: false", ci, guards, format!("{guards}        if: false\n"), false, run_t),
        m("B2 target removed from the list", ci, &last, host.into(), false, run_t),
        m("B2 target passed to echo, not cargo", ci, &last, format!("{host}\n          echo --test {TARGET}"), false, run_t),
        m("B2 cargo test --no-run", ci, "          cargo test --locked \\\n          --test examples_validate", "          cargo test --locked --no-run \\\n          --test examples_validate".into(), false, run_t),
        m("B2 target masked by || true", ci, &last, format!("{host} \\\n          --test {TARGET} || true"), false, "masked by || true"),
        m("B3 mixed-case second checkout", ci, verify, format!("{ex}{verify}"), false, "checkout count 2"),
        m("B3 file-health persist-credentials as string", li, "persist-credentials: false", "persist-credentials: \"false\"".into(), false, "file-health: persist-credentials"),
        m("B3 lockfile persist-credentials true", ci, "        with:\n          persist-credentials: false\n      - name: Verify Cargo.lock", "        with:\n          persist-credentials: true\n      - name: Verify Cargo.lock".into(), false, "lockfile: persist-credentials"),
        m("perm file-health widened", li, "      contents: read\n", "      contents: write\n".into(), false, "file-health: permissions"),
        m("perm workflow-level write-all", ci, "\nconcurrency:\n", "\npermissions: write-all\n\nconcurrency:\n".into(), false, "ci.yml: workflow permissions"),
        m("coe job-level", ci, lock_name, format!("{lock_name}    continue-on-error: true\n"), false, "lockfile: continue-on-error (job)"),
        m("coe step-level", ci, verify, format!("{verify}        continue-on-error: true\n"), false, "lockfile: continue-on-error (step)"),
        m("gate drops lockfile from needs", ci, "needs: [classify, ci, lockfile, ", "needs: [classify, ci, ".into(), false, "gate: does not need lockfile"),
        m("gate stops reading lockfile result", ci, "needs.lockfile.result", "needs.classify.result".into(), true, "gate: does not read lockfile result"),
        m("gate stops reading examples-validate result", ci, "needs.examples-validate.result", "needs.classify.result".into(), true, "gate: does not read examples-validate result"),
        m("gate lockfile result only in an inline comment", ci, lock_if, format!("          if true; then # [ \"{LOCK}\" != \"success\" ]"), false, "gate: does not read lockfile result"),
        m("gate examples-validate result only in an inline comment", ci, ex_ok, format!("          true # ok \"examples-validate\" \"{EXAMPLES}\" || exit 1"), false, "gate: does not read examples-validate result"),
        m("gate step continue-on-error", ci, gate_step, format!("{gate_step}        continue-on-error: true\n"), false, "gate: step if or continue-on-error"),
        m("gate step if: false", ci, gate_step, format!("{gate_step}        if: false\n"), false, "gate: step if or continue-on-error"),
        m("gate if: false", ci, "    if: always()\n", "    if: false\n".into(), false, "gate: if"),
    ]
}

#[test]
fn every_mutant_is_refused() {
    let (ci, lint) = (
        read(".github/workflows/ci.yml"),
        read(".github/workflows/lint.yml"),
    );
    let table = mutants();
    assert!(table.len() >= 10, "{} mutants", table.len());
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

/// Each control sits next to a real check: the thing it changes is present,
/// and the checker must still accept the tree.
#[test]
fn comments_and_names_next_to_real_checks_are_accepted() {
    let (ci, lint) = (
        read(".github/workflows/ci.yml"),
        read(".github/workflows/lint.yml"),
    );
    let last =
        format!("--test falsification_pr_jobs_name_no_arm64_or_host \\\n          --test {TARGET}");
    let edits: [(&str, String); 3] = [
        // the target is also named in an inline comment on the line that really runs it
        (&last, format!("{last} # --test {TARGET}")),
        // the words continue-on-error sit in a step name
        (
            "      - name: Integration guards (examples,",
            "      - name: Integration guards (continue-on-error is not set; examples,".into(),
        ),
        // a shell comment inside a run block says continue-on-error
        (
            "        run: cargo package --locked --no-verify --workspace",
            "        run: cargo package --locked --no-verify --workspace # no continue-on-error"
                .into(),
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

#[test]
fn the_script_reader_splits_what_the_shell_splits() {
    let words = |s: &str| {
        commands(s)
            .into_iter()
            .map(|c| (c.sep, c.words.join(" ")))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        words("a \\\n  b # c\n# d\ne;f && g || h"),
        [
            ("", "a b".into()),
            ("", "e".into()),
            (";", "f".into()),
            ("&&", "g".into()),
            ("||", "h".into())
        ]
    );
    assert_eq!(
        words("echo \"a # b\" 'c;d'"),
        [("", "echo a # b c;d".into())]
    );
    assert_eq!(words("cargo test \\"), [("", "cargo test".into())]);
}
