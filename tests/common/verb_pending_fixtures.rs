//! Fixtures for the verbs discharged from `Bucket::Pending` (paiml/forjar#356).
//!
//! Shared by `falsification_verb_pending_discharge.rs`. Extracted for the
//! 500-line file cap, not because there are two subjects.

use forjar::verb::find;

/// Invoke a verb through the unified surface — the same entry point every
/// transport derives from, so a test that passes here is a test of what MCP and
/// HTTP dispatch, not of a handler reached by a private path.
pub fn call(verb: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    let v = find(verb).unwrap_or_else(|| {
        panic!(
            "verb `{verb}` is not on the unified surface — every transport \
             derives from this one table, so a missing row means the capability \
             is reachable from the CLI and nowhere else"
        )
    });
    (v.invoke)(params)
}

/// Two provenance events on one machine, with distinct timestamps so ordering
/// is a property of the data rather than of `read_dir`.
pub fn audited_project(dir: &std::path::Path) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        "version: \"1.0\"\nname: audited\nmachines:\n  local:\n    hostname: localhost\n    \
         addr: localhost\nresources: {}\n",
    )
    .unwrap();
    let md = dir.join("state").join("local");
    std::fs::create_dir_all(&md).unwrap();
    std::fs::write(
        md.join("events.jsonl"),
        "{\"ts\":\"2026-08-01T10:00:00Z\",\"event\":\"apply_started\",\"machine\":\"local\",\
         \"run_id\":\"r-000000000001\",\"forjar_version\":\"1.20.1\",\"operator\":\"ng@box\"}\n\
         {\"ts\":\"2026-08-01T10:00:05Z\",\"event\":\"resource_converged\",\"machine\":\"local\",\
         \"resource\":\"conf\",\"duration_seconds\":0.5,\"hash\":\"abc123\"}\n",
    )
    .unwrap();
    cfg
}
