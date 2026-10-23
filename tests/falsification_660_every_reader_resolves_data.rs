//! forjar#660: every subcommand that reads a manifest resolves `data:` sources.
//!
//! `{{data.x}}` resolves from the config's resolved data values, which only
//! `resolve_data_sources` writes. `plan`, `apply` and `lock` called it; `show`,
//! `explain`, `check`, `test`, `codegen`, `plan-compact` and `prove` did not,
//! so a manifest that `plan`s cleanly failed `show` with
//! `unknown data source: greeting` (or, where the error was swallowed, was
//! reported on with the reference left unresolved).
//!
//! The defect was per-command wiring, so the test is per-command too, and it
//! does not trust a hand list of readers: it asks the BUILT BINARY which
//! subcommands take `--file` and runs every one of them on a fixture with a
//! `{{data.x}}` reference. A new subcommand that loads the file without
//! resolving data goes red here without anyone remembering to add it.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// Subcommands that change a machine, a state directory or a lock, or block
/// on a server. They are not run against the fixture. Every entry must still
/// be a real subcommand — a stale line fails `skip_list_names_real_commands`.
const NOT_RUN: &[&str] = &[
    "apply",
    "destroy",
    "undo",
    "undo-destroy",
    "rollback",
    "exec",
    "run",
    "make",
    "retry-failed",
    "rolling",
    "canary",
    "multi-apply",
    "parallel-apply",
    "saga",
    "watch",
    "mcp",
    "lsp",
    "bootstrap",
    "trigger",
    "agent",
    "fault-inject",
    "promote",
    "state-mv",
    "state-rm",
    "state-encrypt",
    "state-decrypt",
    "state-rekey",
    "state-reconstruct",
    "import",
    "import-brownfield",
    "import-makefile",
    "init",
    "fmt",
    "migrate",
    "reseal",
    // `rules` files are rulebooks (`rulebooks:` key), not manifests.
    "rules",
    // Re-encrypt / rewrite the manifest's secrets in place.
    "secrets rekey",
    "secrets rotate",
    // Needs the `encryption` cargo feature, which the test binary lacks.
    "secrets view",
];

/// Subcommands that need more than `-f` to reach the loader.
fn extra_args(cmd: &str) -> &'static [&'static str] {
    match cmd {
        "explain" => &["cfg"],
        "codegen" | "build" | "impact" => &["--resource", "cfg"],
        "lock-rebase" => &["state-from"],
        "extract" => &["--glob", "*"],
        "iso-export" => &["-o", "iso-out"],
        "convert" => &["--reproducible"],
        "facts" => &["m1"],
        "environments diff" => &["a", "b"],
        _ => &[],
    }
}

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

fn run(dir: &Path, args: &[&str]) -> Output {
    forjar()
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("spawn forjar")
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// A manifest whose only template is a `data:` command source.
fn fixture() -> tempfile::TempDir {
    fixture_with("echo hello-from-data")
}

/// The same manifest with a `data:` command of the caller's choosing.
fn fixture_with(data_command: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tmpdir");
    let target = dir.path().join("out.txt");
    let yaml = format!(
        r#"version: "1.0"
name: data-660
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m1
    path: {}
    content: "greeting={{{{data.greeting}}}}"
outputs:
  g:
    value: "g: {{{{data.greeting}}}}"
data:
  greeting:
    type: command
    value: "{}"
"#,
        target.display(),
        data_command
    );
    std::fs::write(dir.path().join("forjar.yaml"), yaml).expect("write fixture");
    dir
}

/// The names under a `Commands:` heading of `args --help`.
fn children(dir: &Path, args: &[&str]) -> Vec<String> {
    let mut argv: Vec<&str> = args.to_vec();
    argv.push("--help");
    let help = text(&run(dir, &argv));
    let mut in_list = false;
    let mut cmds = Vec::new();
    for line in help.lines() {
        if line.starts_with("Commands:") {
            in_list = true;
            continue;
        }
        if in_list && !line.starts_with(' ') && !line.trim().is_empty() {
            break;
        }
        if in_list {
            if let Some(name) = line.split_whitespace().next() {
                if name != "help" {
                    cmds.push(name.to_string());
                }
            }
        }
    }
    cmds
}

/// Every subcommand path, nested ones included, as a space-joined label
/// (`environments list`). A group that only has children is listed through its
/// children; the group itself is listed too so a skip row can name it.
fn subcommands(dir: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &[String], out: &mut Vec<String>) {
        let args: Vec<&str> = prefix.iter().map(String::as_str).collect();
        for name in children(dir, &args) {
            let mut path = prefix.to_vec();
            path.push(name);
            out.push(path.join(" "));
            walk(dir, &path, out);
        }
    }
    let mut out = Vec::new();
    walk(dir, &[], &mut out);
    out
}

#[test]
fn every_file_reading_subcommand_resolves_data_sources() {
    let dir = fixture();
    assert!(
        subcommands(dir.path()).len() > 100,
        "read too few subcommands from --help; the parser is measuring nothing"
    );

    let mut probed = Vec::new();
    let mut failed = Vec::new();
    for (label, args) in probes(dir.path()) {
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = text(&run(dir.path(), &argv));
        if out.contains("unknown data source") {
            failed.push(format!("forjar {}: {}", argv.join(" "), out.trim()));
        }
        probed.push(label);
    }

    // Count first: "0 failures" over 0 commands is not a pass.
    assert!(
        probed.len() >= 40,
        "only {} --file subcommands were probed: {probed:?}",
        probed.len()
    );
    for must in [
        "show",
        "explain",
        "check",
        "test",
        "codegen",
        "plan",
        "plan-compact",
        "prove",
        "remediate",
    ] {
        assert!(
            probed.iter().any(|c| c == must),
            "`{must}` was never probed"
        );
    }
    assert!(
        failed.is_empty(),
        "{} of {} --file subcommands did not resolve `data:` sources:\n{}",
        failed.len(),
        probed.len(),
        failed.join("\n")
    );
}

#[test]
fn show_explain_and_codegen_print_the_resolved_value() {
    // Not erroring is not enough: the value must be the one the command produced.
    let dir = fixture();
    // `codegen` ships file content base64-encoded; this is
    // base64("greeting=hello-from-data").
    const ENCODED: &str = "Z3JlZXRpbmc9aGVsbG8tZnJvbS1kYXRh";
    for (args, want) in [
        (&["show", "-f", "forjar.yaml"][..], "hello-from-data"),
        (
            &["explain", "-f", "forjar.yaml", "cfg"][..],
            "hello-from-data",
        ),
        (
            &["codegen", "-f", "forjar.yaml", "--resource", "cfg"][..],
            ENCODED,
        ),
    ] {
        let out = run(dir.path(), args);
        let body = text(&out);
        assert!(
            out.status.success(),
            "forjar {} failed: {body}",
            args.join(" ")
        );
        assert!(
            body.contains(want),
            "forjar {} did not print the data source's value: {body}",
            args.join(" ")
        );
    }
}

#[test]
fn skip_list_names_real_commands() {
    let dir = fixture();
    let cmds: BTreeSet<String> = subcommands(dir.path()).into_iter().collect();
    let stale: Vec<&&str> = NOT_RUN.iter().filter(|c| !cmds.contains(**c)).collect();
    assert!(
        stale.is_empty(),
        "NOT_RUN names commands the binary does not have: {stale:?}"
    );
}

/// Flag-gated readers: the same subcommand reads the manifest only under a flag,
/// so the plain `apply` entry in `NOT_RUN` hides them. Every row here reads and
/// reports without changing a machine, a state directory or a lock.
const FLAG_GATED: &[&[&str]] = &[
    &["apply", "--check"],
    &["apply", "--preview"],
    &["apply", "--dry-run"],
    &["apply", "--dry-run-cost"],
    &["validate", "--dry-expand"],
    &["validate", "--deep"],
];

/// Every probe: each `--file` subcommand not in `NOT_RUN`, then `FLAG_GATED`.
/// Returns `(label, argv)` pairs.
fn probes(dir: &Path) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for cmd in subcommands(dir) {
        let path: Vec<&str> = cmd.split(' ').collect();
        if NOT_RUN.contains(&cmd.as_str()) || NOT_RUN.contains(&path[0]) {
            continue;
        }
        let mut help_argv = path.clone();
        help_argv.push("--help");
        if !text(&run(dir, &help_argv)).contains("--file") {
            continue;
        }
        let mut args: Vec<String> = path.iter().map(|s| s.to_string()).collect();
        args.extend(["-f".into(), "forjar.yaml".into()]);
        args.extend(extra_args(&cmd).iter().map(|s| s.to_string()));
        out.push((cmd, args));
    }
    for row in FLAG_GATED {
        let mut args: Vec<String> = row.iter().map(|s| s.to_string()).collect();
        args.extend(["-f".into(), "forjar.yaml".into()]);
        out.push((row.join(" "), args));
    }
    out
}

/// Commands that provably never touch a template, so a failing `data:` source
/// does not stop them. DENY BY DEFAULT: everything else probed must exit
/// non-zero and name the failing source. A stale row (the command now fails, or
/// is gone) fails `never_touches_templates_is_not_stale`.
const NEVER_TOUCHES_TEMPLATES: &[(&str, &str)] = &[
    (
        "extract",
        "emits a sub-manifest: templates and data: sources stay in it for the next run; loaded unresolved",
    ),
    (
        "env",
        "reports the environment; the config is optional context, loaded unresolved",
    ),
    (
        "complexity",
        "counts structure (resources, includes, depth); renders nothing",
    ),
];

fn names_the_failing_source(out: &str) -> bool {
    out.contains("data source 'greeting'")
}

/// A `data:` source that FAILS must fail the command too.
///
/// DENY BY DEFAULT. The first version of this test kept a hand list of commands
/// that "render templates" and only those had to fail; `output` and the
/// flag-gated readers were never on it, so they exited 0 (or printed the raw
/// `{{data.greeting}}`) and the test stayed green. Now every probed reader must
/// exit non-zero AND say which source failed, unless it is in
/// `NEVER_TOUCHES_TEMPLATES` with a reason.
#[test]
fn a_failing_data_source_fails_every_reader() {
    let dir = fixture_with("exit 3");
    let all = probes(dir.path());
    assert!(
        all.len() >= 40,
        "only {} readers were probed; the enumeration is measuring nothing",
        all.len()
    );
    for must in [
        "show",
        "output",
        "explain",
        "check",
        "plan",
        "contracts",
        "apply --check",
        "apply --preview",
        "validate --dry-expand",
        "validate --deep",
    ] {
        assert!(all.iter().any(|(l, _)| l == must), "`{must}` never probed");
    }
    let mut bad = Vec::new();
    for (label, args) in &all {
        if NEVER_TOUCHES_TEMPLATES.iter().any(|(c, _)| c == label) {
            continue;
        }
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = run(dir.path(), &argv);
        let body = text(&out);
        if out.status.success() || !names_the_failing_source(&body) {
            bad.push(format!(
                "forjar {}: exit={:?}\n    {}",
                argv.join(" "),
                out.status.code(),
                body.trim().lines().next().unwrap_or("")
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} readers did not fail with the failing `data:` source named:\n{}",
        bad.len(),
        all.len(),
        bad.join("\n")
    );
}

#[test]
fn never_touches_templates_is_not_stale() {
    let dir = fixture_with("exit 3");
    let all = probes(dir.path());
    for (cmd, why) in NEVER_TOUCHES_TEMPLATES {
        assert!(!why.is_empty(), "`{cmd}` has no reason");
        let (_, args) = all
            .iter()
            .find(|(l, _)| l == cmd)
            .unwrap_or_else(|| panic!("`{cmd}` is not a probed reader"));
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = run(dir.path(), &argv);
        assert!(
            out.status.success() || !names_the_failing_source(&text(&out)),
            "`{cmd}` now fails on the failing source; delete it from NEVER_TOUCHES_TEMPLATES"
        );
    }
}

/// `output` must print the RESOLVED value, not the template.
#[test]
fn output_prints_the_resolved_value() {
    let dir = fixture();
    let out = run(dir.path(), &["output", "-f", "forjar.yaml"]);
    let body = text(&out);
    assert!(out.status.success(), "forjar output failed: {body}");
    assert!(
        body.contains("hello-from-data") && !body.contains("{{"),
        "output did not resolve the data reference: {body}"
    );
}

/// The unresolved loader is for the few commands that cannot take the resolving
/// one. Each row is a FILE that may call it, with the reason. A new caller
/// anywhere else fails here; a row whose file no longer calls it also fails.
const UNRESOLVED_LOADER_ALLOWLIST: &[(&str, &str)] = &[
    (
        "src/cli/helpers.rs",
        "defines parse_and_validate / parse_and_validate_unresolved",
    ),
    (
        "src/core/resolver/data.rs",
        "load_resolved_config = raw parse + resolve_data_sources, the one resolving loader",
    ),
    (
        "src/mcp/handlers.rs",
        "ValidateHandler parses unresolved FIRST only to tell an invalid manifest (a `valid: false` result) from a failing data: source (an error); it then calls the resolving loader",
    ),
    (
        "src/cli/apply.rs",
        "injects --env-file / --workspace params BEFORE resolving, so data: commands can template them",
    ),
    (
        "src/cli/apply_variants.rs",
        "same ordering: env-file and workspace params first, then data: sources",
    ),
    (
        "src/cli/apply_from_plan.rs",
        "same ordering: must rebuild exactly the config `plan` sealed",
    ),
    (
        "src/cli/plan.rs",
        "applies --what-if / env-file / workspace overrides before resolving data:",
    ),
    (
        "src/cli/lock_core.rs",
        "same ordering as apply: env-file and workspace params, then data:",
    ),
    (
        "src/cli/contracts.rs",
        "a parse failure is graceful (codebase contracts still print) while a data: failure is an error; the resolving loader merges both into one Err",
    ),
    (
        "src/cli/diff_cmd.rs",
        "`env` loads the config as optional context for an environment report; no template is rendered",
    ),
    (
        "src/cli/complexity_analysis.rs",
        "counts structure only; running data: commands would add a failure mode and no information",
    ),
    (
        "src/cli/extract.rs",
        "EMITS a sub-manifest: templates and `data:` sources must stay in it for the next run to resolve",
    ),
    (
        "src/cli/config_merge.rs",
        "WRITES the merged manifest: resolving would bake data: command output into the file",
    ),
    (
        "src/cli/infra_bench.rs",
        "times parse+validate; running data: commands would measure the commands",
    ),
];

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).expect("read_dir") {
        let p = e.expect("entry").path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Non-test source lines of `path` (everything before the first `#[cfg(test)]`;
/// files named `tests*.rs` are all test code).
fn production_lines(path: &Path) -> Vec<String> {
    let name = path.file_name().unwrap().to_string_lossy();
    if name.starts_with("tests") {
        return Vec::new();
    }
    let src = std::fs::read_to_string(path).expect("read source");
    src.lines()
        .take_while(|l| l.trim() != "#[cfg(test)]")
        .map(str::to_string)
        .collect()
}

fn calls_unresolved_loader(line: &str) -> bool {
    let code = line.split("//").next().unwrap_or("");
    code.contains("parse_and_validate_unresolved")
        || code.contains("parse_and_validate_opts")
        || (code.contains("parser") && code.contains("parse_and_validate"))
}

#[test]
fn unresolved_loader_callers_are_allowlisted() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    assert!(files.len() > 300, "only {} source files found", files.len());
    let mut users = BTreeSet::new();
    for f in &files {
        if production_lines(f)
            .iter()
            .any(|l| calls_unresolved_loader(l))
        {
            users.insert(
                f.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            );
        }
    }
    let allowed: BTreeSet<String> = UNRESOLVED_LOADER_ALLOWLIST
        .iter()
        .map(|(f, _)| f.to_string())
        .collect();
    let new: Vec<_> = users.difference(&allowed).collect();
    let stale: Vec<_> = allowed.difference(&users).collect();
    assert!(
        new.is_empty(),
        "these files load a manifest WITHOUT resolving `data:` sources and are not allowlisted \
         (use helpers::parse_and_validate, or add a row with a reason): {new:?}"
    );
    assert!(
        stale.is_empty(),
        "allowlist rows whose file no longer calls the unresolved loader: {stale:?}"
    );
    assert!(UNRESOLVED_LOADER_ALLOWLIST
        .iter()
        .all(|(_, why)| !why.is_empty()));
}

// ---------------------------------------------------------------------------
// Deny-by-default source scan: no raw manifest parse outside the loader.
// ---------------------------------------------------------------------------

/// Every production site that parses YAML straight into a `ForjarConfig` (or
/// into a generic `serde_yaml_ng::Value` document, the other way to read a
/// manifest by hand) outside `src/core/parser/` (the loader's own parser). Each
/// row is `(file, needle, reason)`: the needle must match exactly ONE site's
/// statement text in that file, every site must match exactly one row, and a row
/// that matches nothing is stale. A new raw parse therefore fails here until
/// someone writes down why it may not use the resolving loader.
const RAW_PARSE_ALLOWLIST: &[(&str, &str, &str)] = &[
    (
        "src/cli/template_cmd.rs",
        "::<serde_yaml_ng::Value>(content)",
        "classify() only asks which top-level keys the file has (recipe vs config) before the loader runs; no value is read",
    ),
    (
        "src/cli/template_cmd.rs",
        "::<serde_yaml_ng::Value>(raw)",
        "parses one `-V KEY=VALUE` scalar, not a manifest",
    ),
    (
        "src/cli/apply_helpers.rs",
        "IndexMap<String, serde_yaml_ng::Value>",
        "parses the flat --env-file params mapping, not a manifest",
    ),
    (
        "src/cli/state_identity.rs",
        "serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&s)",
        "reads only `name:` of a stored applied-config snapshot under the state dir",
    ),
    (
        "src/core/state/integrity.rs",
        "serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&content)",
        "checks that a LOCK file is well-formed YAML, not a manifest",
    ),
    (
        "src/core/store/ingest.rs",
        "Value = serde_yaml_ng::from_str(&yaml_str)$",
        "reads a state LOCK file into the store index",
    ),
    (
        "src/core/store/ingest.rs",
        "parse lock: {e}",
        "reads a state LOCK file into the store index",
    ),
    (
        "src/core/store/ingest.rs",
        "match serde_yaml_ng::from_str(&yaml_str)",
        "reads a stored generation snapshot, not the live manifest",
    ),
    (
        "src/core/store/secret_scan.rs",
        "match serde_yaml_ng::from_str(yaml)",
        "scans arbitrary YAML text for secrets; the text is the caller's, nothing resolves",
    ),
    (
        "src/core/store/convert_exec.rs",
        "parse config YAML",
        "WRITES the converted manifest in place after a backup; resolving would bake data: output into the file",
    ),
    (
        "src/cli/destroy.rs",
        "ForjarConfig",
        "parses a PREVIOUS git revision of the manifest to diff resource sets; data: sources must not run against history",
    ),
    (
        "src/cli/init.rs",
        "ForjarConfig",
        "init WRITES a new manifest and re-reads its own template to validate it; there is nothing to resolve",
    ),
    (
        "src/cli/reseal.rs",
        "serde_yaml_ng::Value",
        "reseal rewrites the manifest bytes in place; resolving would bake data: output into the file",
    ),
    (
        "src/cli/lint_fix.rs",
        "serde_yaml_ng::Value",
        "lint --fix rewrites the manifest text; it edits the raw document and must not run data: commands",
    ),
    (
        "src/cli/validate_policy.rs",
        "serde_yaml_ng::Value",
        "reads a separate POLICY file, not the manifest",
    ),
    (
        "src/cli/verify.rs",
        "serde_yaml_ng::Value",
        "reads a lock/attestation document under the state dir, not the manifest",
    ),
];

/// `(line number, statement text)` of every raw-parse site in one source file.
fn raw_parse_sites(rel: &str, src: &str) -> Vec<(usize, String)> {
    // The parser module IS the raw parse, and `resolver/data.rs` is the loader
    // that wraps it (it reads the `data:` section of the raw document).
    if rel.starts_with("src/core/parser/") || rel == "src/core/resolver/data.rs" {
        return Vec::new();
    }
    let lines: Vec<&str> = src
        .lines()
        .take_while(|l| l.trim() != "#[cfg(test)]")
        .collect();
    let code = |l: &str| -> String {
        if l.trim_start().starts_with("//") {
            String::new()
        } else {
            l.split(" // ").next().unwrap_or("").to_string()
        }
    };
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let c = code(l);
        let parses = ["from_str", "from_reader", "from_slice", "from_value"]
            .iter()
            .any(|f| c.contains(&format!("serde_yaml_ng::{f}")));
        if !parses {
            continue;
        }
        let mut start = i;
        while start > 0 && i - start < 6 {
            let t = lines[start].trim_start();
            if t.starts_with("let ") || t.starts_with("fn ") || t.starts_with("pub ") {
                break;
            }
            start -= 1;
        }
        let stmt = lines[start..=i]
            .iter()
            .map(|l| code(l))
            .collect::<Vec<_>>()
            .join(" ");
        let stmt = stmt.split_whitespace().collect::<Vec<_>>().join(" ");
        if stmt.contains("ForjarConfig") || stmt.contains("serde_yaml_ng::Value") {
            out.push((i + 1, stmt));
        }
    }
    out
}

fn all_raw_parse_sites() -> Vec<(String, usize, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    assert!(files.len() > 300, "only {} source files found", files.len());
    let mut sites = Vec::new();
    for f in &files {
        if f.file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("tests")
        {
            continue;
        }
        let rel = f
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let src = std::fs::read_to_string(f).expect("read source");
        for (line, stmt) in raw_parse_sites(&rel, &src) {
            sites.push((rel.clone(), line, stmt));
        }
    }
    sites
}

/// A needle matches a statement by substring; a trailing `$` anchors it to the
/// END of the statement.
fn needle_matches(stmt: &str, needle: &str) -> bool {
    match needle.strip_suffix('$') {
        Some(n) => stmt.ends_with(n),
        None => stmt.contains(needle),
    }
}

/// Which rows match which sites; returns the failures.
fn judge_raw_parses(sites: &[(String, usize, String)]) -> Vec<String> {
    let mut bad = Vec::new();
    for (file, line, stmt) in sites {
        let n = RAW_PARSE_ALLOWLIST
            .iter()
            .filter(|(f, needle, _)| f == file && needle_matches(stmt, needle))
            .count();
        if n != 1 {
            bad.push(format!(
                "{file}:{line}: {} allowlist row(s) match this raw manifest parse (need exactly 1): {stmt}",
                n
            ));
        }
    }
    for (f, needle, why) in RAW_PARSE_ALLOWLIST {
        assert!(!why.is_empty(), "{f} has no reason");
        let n = sites
            .iter()
            .filter(|(sf, _, stmt)| sf == f && needle_matches(stmt, needle))
            .count();
        if n != 1 {
            bad.push(format!(
                "stale or ambiguous allowlist row ({f}, {needle:?}): matches {n} site(s), need exactly 1"
            ));
        }
    }
    bad
}

#[test]
fn no_raw_manifest_parse_outside_the_loader() {
    let sites = all_raw_parse_sites();
    let bad = judge_raw_parses(&sites);
    assert!(
        bad.is_empty(),
        "raw manifest parses outside the loader (use helpers::parse_and_validate / \
         core::resolver::load_resolved_config, or allowlist with a reason):\n{}",
        bad.join("\n")
    );
}

/// The scanner must be able to FAIL: a planted raw parse is flagged, in the
/// three shapes the tree uses (typed let, turbofish, multi-line).
#[test]
fn the_raw_parse_scan_goes_red_on_a_planted_parse() {
    for (name, src) in [
        (
            "typed let",
            "fn f(s: &str) {\n    let c: ForjarConfig = serde_yaml_ng::from_str(s).unwrap();\n}\n",
        ),
        (
            "turbofish",
            "fn f(s: &str) {\n    let c = serde_yaml_ng::from_str::<types::ForjarConfig>(s).unwrap();\n}\n",
        ),
        (
            "multi-line",
            "fn f(s: &str) {\n    let c: types::ForjarConfig =\n        serde_yaml_ng::from_str(s)\n            .unwrap();\n}\n",
        ),
        (
            "generic value",
            "fn f(s: &str) {\n    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(s).unwrap();\n}\n",
        ),
    ] {
        let found = raw_parse_sites("src/cli/planted.rs", src);
        assert_eq!(found.len(), 1, "scan missed the planted `{name}` parse");
        let sites = vec![("src/cli/planted.rs".to_string(), found[0].0, found[0].1.clone())];
        assert!(
            !judge_raw_parses(&sites).is_empty(),
            "an un-allowlisted `{name}` parse was accepted"
        );
    }
    // And the scan does not flag the parser module, comments or non-manifest types.
    assert!(raw_parse_sites(
        "src/core/parser/x.rs",
        "let c: ForjarConfig = serde_yaml_ng::from_str(s).unwrap();"
    )
    .is_empty());
    assert!(raw_parse_sites(
        "src/cli/x.rs",
        "// let c: ForjarConfig = serde_yaml_ng::from_str(s).unwrap();"
    )
    .is_empty());
    assert!(raw_parse_sites(
        "src/cli/x.rs",
        "let l: StateLock = serde_yaml_ng::from_str(s).unwrap();"
    )
    .is_empty());
}

// ---------------------------------------------------------------------------
// Positive probes: a passing `data:` source reaches the output.
// ---------------------------------------------------------------------------

/// Readers that PRINT a rendered template, with a substring only the resolved
/// value produces. (`codegen` ships base64, see `show_explain_and_codegen_...`.)
const PRINTS_THE_VALUE: &[(&str, &str)] = &[
    ("show", "hello-from-data"),
    ("output", "hello-from-data"),
    ("explain", "hello-from-data"),
    ("template", "hello-from-data"),
    ("codegen", "Z3JlZXRpbmc9aGVsbG8tZnJvbS1kYXRh"),
];

/// Every probed reader on a manifest whose `data:` source SUCCEEDS: no leaked
/// `{{data.` text, no `__data__` key, and either the resolved value (where the
/// command prints templates) or exit 0 (where it prints none).
#[test]
fn every_reader_on_a_good_source_prints_the_value_or_exits_clean() {
    let dir = fixture();
    let all = probes(dir.path());
    assert!(all.len() >= 40, "only {} readers probed", all.len());
    let mut bad = Vec::new();
    let mut printed = 0;
    for (label, args) in &all {
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = run(dir.path(), &argv);
        let body = text(&out);
        let mut why = Vec::new();
        if body.contains("{{data.") && !RAW_BY_DESIGN.contains(&label.as_str()) {
            why.push("left `{{data.` in the output");
        }
        if body.contains("__data__") {
            why.push("printed a `__data__` key");
        }
        if let Some((_, want)) = PRINTS_THE_VALUE.iter().find(|(l, _)| l == label) {
            printed += 1;
            if !body.contains(want) {
                why.push("did not print the resolved value");
            }
        } else if !out.status.success()
            && !EXITS_NONZERO_ON_THIS_FIXTURE
                .iter()
                .any(|(l, _)| l == label)
        {
            why.push("exited non-zero");
        }
        if !why.is_empty() {
            bad.push(format!(
                "forjar {}: {} (exit {:?})\n    {}",
                argv.join(" "),
                why.join(", "),
                out.status.code(),
                body.trim().lines().next().unwrap_or("")
            ));
        }
    }
    assert!(printed >= 4, "only {printed} printing readers were probed");
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// Readers that exit non-zero on the fixture for a reason unrelated to data:
/// (the machine is not reachable, nothing is applied yet). Each is still held to
/// "no `{{data.` and no `__data__`" above.
const EXITS_NONZERO_ON_THIS_FIXTURE: &[(&str, &str)] = &[
    ("drift", "no lock yet: drift exits 2 on a fresh machine"),
    ("status", "no state directory yet"),
    (
        "check",
        "the target file does not exist yet, which is a failed check",
    ),
    (
        "test",
        "the target file does not exist yet, which is a failed test",
    ),
    (
        "apply --check",
        "the target file does not exist yet, which is a failed check",
    ),
    (
        "apply --preview",
        "stdin is closed, so the confirmation prompt aborts",
    ),
    (
        "score",
        "a fresh manifest with no runtime data grades F, below the threshold",
    ),
    ("build", "the fixture resource is not type: image"),
    ("environments diff", "the fixture has no environments"),
    ("dist", "the fixture has no dist: section"),
];

/// Readers whose output is the manifest (or the manifest text) by design, so it
/// keeps `{{...}}` templates: `explain` prints the raw definition beside the
/// resolved one, `remediate` copies the source bytes through, `extract` emits a
/// sub-manifest.
const RAW_BY_DESIGN: &[&str] = &["explain", "remediate", "extract"];

/// `show` without `--resource` prints the RESOLVED outputs, and no `__data__`.
#[test]
fn show_prints_resolved_outputs_and_no_data_key() {
    let dir = fixture();
    let out = run(dir.path(), &["show", "-f", "forjar.yaml"]);
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    assert!(
        body.contains("g: hello-from-data"),
        "outputs unresolved: {body}"
    );
    assert!(
        !body.contains("__data__"),
        "show leaked a __data__ key: {body}"
    );
    assert!(!body.contains("{{"), "show left a template: {body}");
}

/// `validate --strict|--exhaustive|--deep` must not call the data value an
/// unused or unknown param.
#[test]
fn validate_modes_do_not_see_a_data_param() {
    let dir = fixture();
    let yaml = std::fs::read_to_string(dir.path().join("forjar.yaml")).unwrap();
    std::fs::write(
        dir.path().join("forjar.yaml"),
        format!("description: data fixture\n{yaml}"),
    )
    .unwrap();
    for flag in ["--strict", "--exhaustive", "--deep"] {
        let out = run(dir.path(), &["validate", flag, "-f", "forjar.yaml"]);
        let body = text(&out);
        assert!(out.status.success(), "validate {flag} failed: {body}");
        assert!(
            !body.contains("__data__") && !body.to_lowercase().contains("unused param"),
            "validate {flag} reported the data value as a param: {body}"
        );
    }
}

/// An empty-`resources` manifest whose `data:` source fails still errors in
/// `contracts`: the source is resolved at load, not only when a resource needs it.
#[test]
fn contracts_with_no_resources_and_a_failing_source_errors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("forjar.yaml"),
        "version: \"1.0\"\nname: empty-660\nmachines: {}\nresources: {}\ndata:\n  greeting:\n    type: command\n    value: \"exit 3\"\n",
    )
    .unwrap();
    let out = run(dir.path(), &["contracts", "-f", "forjar.yaml"]);
    let body = text(&out);
    assert!(!out.status.success(), "contracts exited 0: {body}");
    assert!(names_the_failing_source(&body), "{body}");
}

/// `remediate` never writes, so it resolves like MCP `forjar_remediate`: a
/// failing source stops it, a good one lets it run.
#[test]
fn remediate_resolves_data_sources() {
    let bad = fixture_with("exit 3");
    let out = run(bad.path(), &["remediate", "-f", "forjar.yaml"]);
    assert!(!out.status.success(), "remediate exited 0: {}", text(&out));
    assert!(names_the_failing_source(&text(&out)), "{}", text(&out));
    let good = fixture();
    let out = run(good.path(), &["remediate", "-f", "forjar.yaml"]);
    assert!(out.status.success(), "{}", text(&out));
}

/// Nested subcommands are enumerated (not just the top level).
#[test]
fn subcommand_discovery_recurses() {
    let dir = fixture();
    let cmds = subcommands(dir.path());
    for want in ["environments list", "environments diff"] {
        assert!(cmds.iter().any(|c| c == want), "`{want}` not discovered");
    }
}
