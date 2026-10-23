//! forjar#660: every subcommand that reads a manifest resolves `data:` sources.
//!
//! `{{data.x}}` resolves from `params.__data__x`, which only
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
    "remediate",
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

fn subcommands(dir: &Path) -> Vec<String> {
    let help = text(&run(dir, &["--help"]));
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
        if NOT_RUN.contains(&cmd.as_str()) || !text(&run(dir, &[&cmd, "--help"])).contains("--file")
        {
            continue;
        }
        let mut args = vec![cmd.clone(), "-f".into(), "forjar.yaml".into()];
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
        "env",
        "reports the environment; the config is optional context, loaded unresolved",
    ),
    (
        "complexity",
        "counts structure (resources, includes, depth); renders nothing",
    ),
    (
        "score",
        "static scoring over the raw YAML text (scoring::compute_from_file)",
    ),
    (
        "pin",
        "hashes each resource's raw provider/version pair from the YAML mapping",
    ),
    (
        "convert",
        "reads the raw YAML to propose a reproducible conversion; no template is rendered",
    ),
    (
        "dist",
        "reads the raw dist: section; the artifacts carry no resource templates",
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
        "src/cli/config_merge.rs",
        "WRITES the merged manifest: resolved data would bake __data__ params and command output into the file",
    ),
    (
        "src/cli/remediate.rs",
        "rewrites the manifest text; it checks policy on the raw config and must not run data: commands",
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
