//! FALSIFY-FVS-005: the binary advertises no transport it has not declared.
//!
//! `contracts/verb-surface-v1.yaml` has cited this file since #288. It did not
//! exist. Its equation —
//!
//!     advertised(binary) ∩ {mcp, serve, http} ⊆ keys([package.metadata.transports])
//!
//! read as an enforced property for months while nothing computed either side,
//! which is GH-298: a guard reporting what it did not measure.
//!
//! The honest options when a citation dangles are to write the missing test or
//! to say in the contract that the obligation is discharged somewhere else.
//! Pointing it at the nearest green test would re-commit the original defect at
//! a smaller scale, so this is the first option: the property, computed.
//!
//! Everything here spawns `CARGO_BIN_EXE_forjar`, for the reason
//! `e2e_verb_surface_t.rs` gives at length — a transport that exists in the
//! source and is routed to by nothing is exactly the failure this contract was
//! written after. Reading `src/` would prove the wrong thing.
//!
//! WHY `rules serve` IS NOT A COUNTEREXAMPLE. `forjar rules serve` opens a
//! socket and is not declared as a transport. That is deliberate and is
//! recorded in the contract: a webhook receiver ACCEPTS events, it does not
//! expose forjar's capability set. Declaring it one would assert parity between
//! `forjar plan` and an HMAC event endpoint, which is not a meaningful
//! equality. The membership test below is over capability surfaces, so it asks
//! about `verb serve`, not about every subcommand that happens to listen.

use std::collections::BTreeSet;
use std::process::Command;

fn stdout_of(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawning forjar {args:?} failed: {e}"));
    assert!(
        out.status.success(),
        "forjar {args:?} exited {:?}\nstderr: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// Subcommand names from a clap help page's `Commands:` block.
fn subcommands(help: &str) -> BTreeSet<String> {
    help.lines()
        .skip_while(|l| l.trim() != "Commands:")
        .skip(1)
        .take_while(|l| !l.trim().is_empty())
        .filter_map(|l| l.strip_prefix("  "))
        .filter(|l| !l.starts_with(' '))
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// `[package.metadata.transports]` keys, read from Cargo.toml as shipped.
///
/// Parsed by hand rather than with a TOML crate so the declaration this test
/// reads is the same text a human reads. The block is a flat run of
/// `key = { ... }` lines.
fn declared_transports() -> BTreeSet<String> {
    let manifest = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("Cargo.toml must be readable");
    manifest
        .lines()
        .skip_while(|l| l.trim() != "[package.metadata.transports]")
        .skip(1)
        .take_while(|l| !l.trim_start().starts_with('['))
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .map(|(k, _)| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect()
}

/// The e2e each transport declaration names as its reachability proof.
fn declared_e2es() -> Vec<(String, String)> {
    let manifest = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("Cargo.toml must be readable");
    let mut out = Vec::new();
    for line in manifest
        .lines()
        .skip_while(|l| l.trim() != "[package.metadata.transports]")
        .skip(1)
        .take_while(|l| !l.trim_start().starts_with('['))
    {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Some(rest) = value.split_once("e2e") else {
            continue;
        };
        let Some(name) = rest.1.split('"').nth(1) else {
            continue;
        };
        out.push((key.trim().to_string(), name.to_string()));
    }
    out
}

#[test]
fn the_probe_can_see_the_surface() {
    // Guards the guard. If `--help` stopped listing subcommands — a clap
    // upgrade, a spawn that silently produced nothing — the membership test
    // below would pass over an empty set, which is the vacuous green this
    // contract exists to make impossible.
    let top = subcommands(&stdout_of(&["--help"]));
    assert!(
        top.len() > 20,
        "forjar --help advertised only {} subcommand(s); the probe is broken, \
         not the binary: {top:?}",
        top.len()
    );
    assert!(
        !declared_transports().is_empty(),
        "[package.metadata.transports] parsed as empty — the declaration side \
         of the containment is unreadable, so the test proves nothing"
    );
}

#[test]
fn the_binary_advertises_no_undeclared_transport() {
    let declared = declared_transports();
    let top = subcommands(&stdout_of(&["--help"]));
    let verb = subcommands(&stdout_of(&["verb", "--help"]));

    let mut undeclared = Vec::new();
    // The CLI is itself a transport, and it is always advertised.
    if !declared.contains("cli") {
        undeclared.push("the CLI is shipped but `cli` is not declared".to_string());
    }
    if top.contains("mcp") && !declared.contains("mcp") {
        undeclared.push("`forjar mcp` is advertised but `mcp` is not declared".to_string());
    }
    if verb.contains("serve") && !declared.contains("http") {
        undeclared.push("`forjar verb serve` is advertised but `http` is not declared".to_string());
    }

    assert!(
        undeclared.is_empty(),
        "an undeclared transport is an unverified one — it is exempt from the \
         parity gate by being invisible to it:\n  {}",
        undeclared.join("\n  ")
    );
}

#[test]
fn every_declared_transport_names_a_reachability_proof_that_exists() {
    // The reverse containment. A declaration whose `e2e` names nothing is the
    // same phantom citation in a different file.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let declared = declared_e2es();
    assert!(
        !declared.is_empty(),
        "no transport declares an e2e — every key must name the test that \
         proves it reachable from the shipped binary"
    );
    let mut dangling = Vec::new();
    for (key, e2e) in declared {
        if !root.join(format!("{e2e}.rs")).is_file() {
            dangling.push(format!(
                "`{key}` names e2e `{e2e}` — tests/{e2e}.rs does not exist"
            ));
        }
    }
    assert!(
        dangling.is_empty(),
        "declared transports cite reachability proofs that do not exist:\n  {}",
        dangling.join("\n  ")
    );
}
