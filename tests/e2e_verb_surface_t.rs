//! End-to-end proof that the unified verb surface is REACHABLE.
//!
//! Every assertion here spawns `CARGO_BIN_EXE_forjar`. That is the entire
//! point of this file, and it is not a stylistic preference.
//!
//! rmedia — the sibling that wrote the UVS spec first — registered 34 verbs and
//! proved four-way parity across CLI, MCP, HTTP and a generated manifest. Its
//! suite was green for its whole life while `main.rs` routed only two of those
//! surfaces: the derived CLI tree had no caller at all. A parity test compares
//! transports to each other, so it is structurally incapable of noticing that
//! none of them is reachable. Agreement is not reachability.
//!
//! So: no in-process calls into `forjar::verb::*` in this file. If the surface
//! stops being wired into `main.rs`, these tests must fail.

use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

fn stdout_of(args: &[&str]) -> String {
    let out = forjar()
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

/// The verb surface must be reachable from the shipped binary at all.
#[test]
fn verb_is_a_real_subcommand() {
    let help = stdout_of(&["--help"]);
    assert!(
        help.contains("verb"),
        "`verb` is absent from `forjar --help` — the derived surface is not \
         wired into main.rs, which is exactly the failure a parity test cannot see"
    );
}

/// The shipped binary lists a non-empty surface. An empty list would make every
/// parity assertion downstream vacuously true.
#[test]
fn verb_list_is_not_empty() {
    let out = stdout_of(&["verb", "list"]);
    let names: Vec<&str> = out.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        names.len() >= 9,
        "the shipped binary lists {} verbs; expected the full unified surface",
        names.len()
    );
}

/// FVS-1, the CLI leg: names as a CLIENT sees them, read out of the process.
#[test]
fn verb_list_matches_the_json_rendering() {
    let plain: Vec<String> = stdout_of(&["verb", "list"])
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect();

    let json = stdout_of(&["verb", "list", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).expect("verb list --json is not JSON");
    let from_json: Vec<String> = v["verbs"]
        .as_array()
        .expect("verbs array")
        .iter()
        .map(|r| r["name"].as_str().unwrap_or_default().to_string())
        .collect();

    assert_eq!(
        plain, from_json,
        "the two renderings of the same surface disagree"
    );
}

/// Every verb publishes both schemas, from the binary. A Null schema would let
/// FVS-2 pass while validating nothing.
#[test]
fn every_verb_publishes_both_schemas() {
    for name in stdout_of(&["verb", "list"])
        .lines()
        .filter(|l| !l.is_empty())
    {
        let out = stdout_of(&["verb", "schema", name]);
        let v: serde_json::Value = serde_json::from_str(&out).expect("schema is not JSON");
        for key in ["input_schema", "output_schema"] {
            assert!(
                v[key].is_object() && !v[key].as_object().unwrap().is_empty(),
                "{name}: {key} is empty"
            );
        }
    }
}

/// FVS-1 proper: the CLI surface and the MCP surface, each read out of the
/// SPAWNED binary, must name the same verbs — and `readOnlyHint` must be the
/// same value on both, because both derive it from one `Effects` field.
///
/// This is the assertion that would have caught the four-way duplication:
/// `export_schema`, `build_registry`, `build_forge_config` and `serve` each
/// carried their own list, and only `serve` shipped.
#[test]
fn cli_and_mcp_surfaces_agree() {
    let cli: serde_json::Value =
        serde_json::from_str(&stdout_of(&["verb", "list", "--json"])).expect("cli json");
    let mcp: serde_json::Value =
        serde_json::from_str(&stdout_of(&["mcp", "--schema"])).expect("mcp schema json");

    let cli_names: Vec<String> = cli["verbs"]
        .as_array()
        .expect("verbs")
        .iter()
        .map(|r| r["mcp_name"].as_str().unwrap_or_default().to_string())
        .collect();
    let mcp_names: Vec<String> = mcp["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|t| t["name"].as_str().unwrap_or_default().to_string())
        .collect();

    assert_eq!(
        cli_names, mcp_names,
        "the CLI and MCP surfaces name different verbs"
    );
    assert!(
        !cli_names.is_empty(),
        "both surfaces are empty — parity is vacuous"
    );

    for r in cli["verbs"].as_array().expect("verbs") {
        let want = r["mcp_name"].as_str().unwrap_or_default();
        let tool = mcp["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .find(|t| t["name"] == want)
            .unwrap_or_else(|| panic!("{want} is on the CLI surface but not in MCP"));
        assert_eq!(
            tool["annotations"]["readOnlyHint"], r["read_only"],
            "{want}: readOnlyHint disagrees between MCP and CLI"
        );
        assert_eq!(
            tool["description"], r["description"],
            "{want}: description disagrees between MCP and CLI"
        );
    }
}

/// FVS-2 from outside the process: params are rejected before the handler runs.
#[test]
fn bad_params_are_rejected() {
    let out = forjar()
        .args(["verb", "call", "validate", "--json", "{\"wrong\":1}"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "invalid params should not exit 0");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("invalid params"),
        "expected a params rejection, got: {err}"
    );
}

/// An unknown verb names the alternatives rather than just refusing.
#[test]
fn unknown_verb_names_the_surface() {
    let out = forjar()
        .args(["verb", "schema", "definitely-not-a-verb"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("validate") && err.contains("unknown verb"),
        "the error should teach the surface, got: {err}"
    );
}
