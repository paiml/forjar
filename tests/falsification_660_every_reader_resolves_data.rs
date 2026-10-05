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
data:
  greeting:
    type: command
    value: "echo hello-from-data"
"#,
        target.display()
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
    let cmds = subcommands(dir.path());
    assert!(
        cmds.len() > 100,
        "read only {} subcommands from --help; the parser is measuring nothing",
        cmds.len()
    );

    let mut probed = Vec::new();
    let mut failed = Vec::new();
    for cmd in &cmds {
        if NOT_RUN.contains(&cmd.as_str()) {
            continue;
        }
        if !text(&run(dir.path(), &[cmd, "--help"])).contains("--file") {
            continue;
        }
        let mut args = vec![cmd.as_str(), "-f", "forjar.yaml"];
        args.extend_from_slice(extra_args(cmd));
        let out = text(&run(dir.path(), &args));
        if out.contains("unknown data source") {
            failed.push(format!("forjar {}: {}", args.join(" "), out.trim()));
        }
        probed.push(cmd.clone());
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
