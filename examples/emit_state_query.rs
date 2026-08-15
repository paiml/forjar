//! Emit the state-query script forjar runs for a resource, resolved and ready
//! to execute. Answers "why is this resource drifting?" in one command:
//! anything in the output that changes between two runs is volatile and must
//! move to stderr, or every machine reports drifted on every check.
//!
//! Usage: cargo run --example emit_state_query -- <forjar.yaml> <resource-id>

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: <forjar.yaml> <resource-id>");
    let id = std::env::args()
        .nth(2)
        .expect("usage: <forjar.yaml> <resource-id>");
    let cfg = forjar::core::parser::parse_and_validate(std::path::Path::new(&path)).expect("parse");
    let resolved = forjar::core::resolver::resolve_all(
        &cfg.resources,
        &cfg.params,
        &cfg.machines,
        &cfg.secrets,
    );
    let r = resolved.get(&id).expect("resource not found");
    print!(
        "{}",
        forjar::core::codegen::state_query_script(r).expect("codegen")
    );
}
