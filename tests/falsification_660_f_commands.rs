//! forjar#660: EVERY command that takes `-f/--file` fails on a bad source.
//!
//! The command list is DERIVED FROM THE CLAP DEFINITION (`forjar::cli::Commands`
//! walked with `CommandFactory`), not from `--help` text and not from a floor.
//! Each command, and each of its boolean flags and each value of a magic value
//! flag (`test -g behavior`), is run against two manifests:
//!   * `bad.yaml`: a `data:` source whose command exits 3;
//!   * `missing.yaml`: a resource that reads `{{data.missing}}`, declared nowhere.
//!
//! Both must exit non-zero and say which source is wrong. The number of probes
//! run must equal the number derived, or the test is RED.

mod common_660;

use common_660::{file_commands, FileCmd};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, Stdio};

/// `-f` here names an ARCHIVE, not a manifest. Each must still be a derived
/// command (a stale row is RED).
const NOT_A_MANIFEST: &[(&str, &str)] = &[
    (
        "reseal",
        "FILE is a lock or state document whose BLAKE3 sidecar is rewritten; any YAML",
    ),
    ("digest", "FILE is a recipe, not a manifest"),
    ("rules validate", "FILE is a rulebook"),
    ("rules coverage", "FILE is a rulebook"),
    ("rules serve", "FILE is a rulebook"),
    ("trigger", "FILE is a rulebook"),
    ("archive unpack", "FILE is a .far archive"),
    ("archive inspect", "FILE is a .far archive"),
    ("archive verify", "FILE is a .far archive"),
];

/// Value flags whose VALUE selects a different reader (`test -g behavior`
/// never loaded `-f` at all). `(command, flag, values)`; checked against clap.
const VALUE_SETS: &[(&str, &str, &[&str])] = &[(
    "test",
    "--group",
    &["mutation", "convergence", "coverage", "behavior"],
)];

/// Commands that cannot reach the loader with placeholder arguments: they
/// reject the placeholder first (a recipient key, a machine). Each is probed,
/// must still exit non-zero, and is exempt only from naming the data source.
/// Arguments a command needs before it reaches the loader (a confirmation flag,
/// a required filter). Without them it refuses first and proves nothing.
const UNBLOCK: &[(&str, &[&str])] = &[
    ("destroy", &["--yes"]),
    ("convert", &["--reproducible"]),
    ("extract", &["--tags", "x"]),
    ("exec", &["--", "true"]),
];

/// Probes that never read the manifest and exit 0 by design.
const NEVER_LOADS: &[(&str, &str)] = &[
    (
        "lint --bashrs-version",
        "prints the bashrs version and exits",
    ),
    (
        "lock --restamp",
        "renames existing lock files to the running forjar version; reads no manifest",
    ),
];

/// Probes that must exit non-zero but are refused before the loader for a
/// reason of their own.
const REFUSES_FIRST: &[(&str, &str)] = &[
    (
        "rollback",
        "reads the PREVIOUS git revision of the file; refuses outside a repo",
    ),
    (
        "rollback --dry-run",
        "reads the previous git revision of the file",
    ),
    (
        "rollback --yes",
        "reads the previous git revision of the file",
    ),
    ("secrets view", "this build has no encryption support"),
    (
        "watch --apply",
        "refuses --apply without --yes; --yes would start a blocking watch",
    ),
    (
        "pin --check",
        "needs an existing inputs lock before it reads the manifest",
    ),
];

const REJECTS_PLACEHOLDER_FIRST: &[(&str, &str)] = &[
    (
        "secrets rekey",
        "validates --recipient as an age key before reading the manifest",
    ),
    (
        "secrets rotate",
        "validates --recipient as an age key before reading the manifest",
    ),
];

fn bad_manifest(dir: &Path) {
    let yaml = |data: &str, content: &str| {
        format!(
            "version: \"1.0\"\nname: d660\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m1\n    path: {}/out.txt\n    content: \"{content}\"\n{data}",
            dir.display()
        )
    };
    std::fs::write(
        dir.join("bad.yaml"),
        yaml(
            "data:\n  greeting:\n    type: command\n    value: \"exit 3\"\n",
            "greeting={{data.greeting}}",
        ),
    )
    .unwrap();
    std::fs::write(
        dir.join("missing.yaml"),
        yaml("", "greeting={{data.missing}}"),
    )
    .unwrap();
}

/// One argv per probe: `(label, argv)`.
fn plan(manifest: &str) -> Vec<(String, Vec<String>)> {
    let skip: BTreeSet<&str> = NOT_A_MANIFEST.iter().map(|(c, _)| *c).collect();
    let mut out = Vec::new();
    for c in file_commands() {
        if skip.contains(c.label.as_str()) {
            continue;
        }
        let base = base_argv(&c, manifest);
        out.push((c.label.clone(), with_extras(&c.label, base.clone(), &[])));
        for flag in &c.bool_flags {
            out.push((
                format!("{} {flag}", c.label),
                with_extras(&c.label, base.clone(), std::slice::from_ref(flag)),
            ));
        }
        for (cmd, flag, values) in VALUE_SETS {
            if *cmd == c.label {
                for v in *values {
                    out.push((
                        format!("{} {flag} {v}", c.label),
                        with_extras(&c.label, base.clone(), &[flag.to_string(), v.to_string()]),
                    ));
                }
            }
        }
    }
    out
}

fn with_extras(label: &str, mut a: Vec<String>, flags: &[String]) -> Vec<String> {
    a.extend(flags.iter().cloned());
    for (cmd, extra) in UNBLOCK {
        if *cmd == label {
            for e in extra.iter().filter(|e| !flags.iter().any(|f| f == *e)) {
                a.push(e.to_string());
            }
        }
    }
    a
}

fn base_argv(c: &FileCmd, manifest: &str) -> Vec<String> {
    let mut a = c.path.clone();
    for (_, long, takes) in &c.required {
        match long {
            Some(l) => {
                a.push(l.clone());
                if *takes {
                    a.push("cfg".into());
                }
            }
            None => a.push("cfg".into()),
        }
    }
    match &c.flag {
        Some(f) => a.extend([f.clone(), manifest.to_string()]),
        None => a.push(manifest.to_string()),
    }
    a
}

/// The probe count, from the clap tree and the two tables, counted without
/// going through `plan`.
fn derived_count() -> usize {
    let skip: BTreeSet<&str> = NOT_A_MANIFEST.iter().map(|(c, _)| *c).collect();
    file_commands()
        .iter()
        .filter(|c| !skip.contains(c.label.as_str()))
        .map(|c| {
            1 + c.bool_flags.len()
                + VALUE_SETS
                    .iter()
                    .filter(|(cmd, _, _)| *cmd == c.label)
                    .map(|(_, _, v)| v.len())
                    .sum::<usize>()
        })
        .sum()
}

struct Verdict {
    label: String,
    probe: String,
    code: Option<i32>,
    body: String,
}

fn run_all(dir: &Path, probes: Vec<(String, Vec<String>)>) -> Vec<Verdict> {
    let probes = std::sync::Arc::new(probes);
    let next = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let results = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut handles = Vec::new();
    for _ in 0..8 {
        let (probes, next, results) = (probes.clone(), next.clone(), results.clone());
        let dir = dir.to_path_buf();
        handles.push(std::thread::spawn(move || loop {
            let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let Some((label, argv)) = probes.get(i) else {
                break;
            };
            let out = Command::new("timeout")
                .arg("30")
                .arg(env!("CARGO_BIN_EXE_forjar"))
                .args(argv)
                .current_dir(&dir)
                .env("HOME", &dir)
                .stdin(Stdio::null())
                .output()
                .expect("spawn timeout forjar");
            results.lock().unwrap().push((
                i,
                Verdict {
                    label: format!("forjar {}", argv.join(" ")),
                    probe: label.clone(),
                    code: out.status.code(),
                    body: format!(
                        "{}{}",
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr)
                    ),
                },
            ));
        }));
    }
    for h in handles {
        h.join().expect("probe thread");
    }
    let mut v = std::mem::take(&mut *results.lock().unwrap());
    v.sort_by_key(|(i, _)| *i);
    v.into_iter().map(|(_, v)| v).collect()
}

fn check(manifest: &str, key: &str) {
    let dir = tempfile::tempdir().unwrap();
    bad_manifest(dir.path());
    let probes = plan(manifest);
    let expected = derived_count();
    let verdicts = run_all(dir.path(), probes);
    // Count first: "0 failures" over 0 probes is not a pass.
    assert_eq!(
        verdicts.len(),
        expected,
        "ran {} probes but the clap definition derives {expected}",
        verdicts.len()
    );
    assert!(expected > 0);
    let never: BTreeSet<&str> = NEVER_LOADS.iter().map(|(c, _)| *c).collect();
    let first: BTreeSet<&str> = REFUSES_FIRST.iter().map(|(c, _)| *c).collect();
    let exempt: BTreeSet<&str> = REJECTS_PLACEHOLDER_FIRST.iter().map(|(c, _)| *c).collect();
    let mut bad = Vec::new();
    for v in &verdicts {
        if never.contains(v.probe.as_str()) {
            continue;
        }
        let refused_first = first.contains(v.probe.as_str());
        let only_exempt = exempt
            .iter()
            .any(|c| v.label.starts_with(&format!("forjar {c} ")));
        let failed = v.code.is_some_and(|c| c != 0);
        let named = v.body.contains("data source") && v.body.contains(key);
        // A flag the build rejects before it reads anything fails closed too.
        let rejected = v.body.contains("not implemented in this build");
        if !failed || (!named && !only_exempt && !rejected && !refused_first) {
            bad.push(format!(
                "{}: exit={:?} named={named}\n    {}",
                v.label,
                v.code,
                v.body.trim().lines().next().unwrap_or("")
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} `-f` probes on {manifest} did not fail naming data source `{key}`:\n{}",
        bad.len(),
        verdicts.len(),
        bad.join("\n")
    );
}

#[test]
fn every_f_command_fails_on_a_failing_data_source() {
    check("bad.yaml", "greeting");
}

#[test]
fn every_f_command_fails_on_an_undeclared_data_reference() {
    check("missing.yaml", "missing");
}

#[test]
fn the_tables_name_real_commands_and_flags() {
    let cmds = file_commands();
    let labels: BTreeSet<&str> = cmds.iter().map(|c| c.label.as_str()).collect();
    let probes: BTreeSet<String> = plan("x.yaml").into_iter().map(|(l, _)| l).collect();
    for (p, why) in NEVER_LOADS.iter().chain(REFUSES_FIRST) {
        assert!(probes.contains(*p), "stale row: no probe `{p}`");
        assert!(!why.is_empty());
    }
    for (c, why) in NOT_A_MANIFEST.iter().chain(REJECTS_PLACEHOLDER_FIRST) {
        assert!(labels.contains(c), "stale row: `{c}` is not a `-f` command");
        assert!(!why.is_empty());
    }
    for (c, flag, values) in VALUE_SETS {
        let cmd = cmds
            .iter()
            .find(|x| x.label == *c)
            .unwrap_or_else(|| panic!("stale VALUE_SETS row: `{c}`"));
        assert!(!values.is_empty());
        assert!(
            cmd.value_flags.iter().any(|f| f == flag),
            "`{c}` has no value flag {flag}: {:?}",
            cmd.value_flags
        );
    }
    // The derivation sees the commands the earlier help-text scrape was blind to.
    for must in [
        "test", "template", "output", "show", "validate", "status", "env", "fmt",
    ] {
        assert!(labels.contains(must), "`{must}` was not derived");
    }
}
