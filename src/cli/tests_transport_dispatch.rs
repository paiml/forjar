//! GH-267: the transport verbs reach their servers through `dispatch`.
//!
//! These are the wiring assertions that can be made in-process. They are not
//! the reachability proof — that lives in `tests/uvs_e2e.rs`, which spawns the
//! shipped binary, because a module test cannot tell whether `main` calls it.

use super::commands::*;
use super::dispatch_misc::dispatch_misc_cmd;

/// The derived catalogue is what `forjar mcp --schema` prints.
#[test]
fn mcp_schema_routes_to_the_derived_catalogue() {
    let args = McpArgs {
        schema: true,
        legacy: false,
    };
    assert!(dispatch_misc_cmd(Commands::Mcp(args), false).is_ok());
}

/// `--schema --legacy` still reaches the pre-2.0 pforge catalogue. A
/// deprecated flag that silently does nothing is worse than a removed one.
#[test]
fn mcp_schema_legacy_routes_to_the_pforge_catalogue() {
    let args = McpArgs {
        schema: true,
        legacy: true,
    };
    assert!(dispatch_misc_cmd(Commands::Mcp(args), false).is_ok());
}

/// The two catalogues are different documents, so `--legacy` is observably a
/// different code path and not an ignored flag.
#[test]
fn the_derived_and_legacy_catalogues_differ() {
    let derived = crate::verb::catalogue();
    let legacy = crate::mcp::export_schema();
    assert_eq!(legacy["tool_count"], 9, "the 1.x server had nine tools");
    assert!(
        derived["verb_count"].as_u64().unwrap() > 150,
        "the derived catalogue carries the whole surface"
    );
    assert_ne!(derived, legacy);
}

/// `serve` binds a socket, so dispatch is exercised only for its failure path;
/// the success path is covered end-to-end in `tests/uvs_e2e.rs`.
#[test]
fn serve_reports_a_bind_failure_rather_than_succeeding_silently() {
    let args = ServeArgs {
        port: 1, // privileged
        host: "127.0.0.1".to_string(),
        read_only: false,
    };
    let err = dispatch_misc_cmd(Commands::Serve(args), false).unwrap_err();
    assert!(err.contains("127.0.0.1:1"), "{err}");
}
