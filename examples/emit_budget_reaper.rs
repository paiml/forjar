//! FJ-036: emit the generated disk-budget reaper for a resource, so an
//! operator can dry-run a reclaim before authorising `forjar apply`.
//!
//! Usage: cargo run --example emit_budget_reaper -- <forjar.yaml> <resource-id>
//! Then:  sh reaper.sh   # previews; the reaper deletes only under
//!                       # FORJAR_BUDGET_EXECUTE=1 (forjar#334)
//!
//! `forjar codegen -r <id> --phase reaper` is the supported form of this.

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: <forjar.yaml> <resource-id>");
    let id = std::env::args()
        .nth(2)
        .expect("usage: <forjar.yaml> <resource-id>");
    let cfg = forjar::core::parser::parse_and_validate(std::path::Path::new(&path)).expect("parse");
    // Resolve templates first — an unexpanded `{{params.home}}` in a reclaim
    // root would silently match nothing and make the preview a lie.
    let resolved = forjar::core::resolver::resolve_all(
        &cfg.resources,
        &cfg.params,
        &cfg.machines,
        &cfg.secrets,
    );
    let r = resolved.get(&id).expect("resource not found");
    let apply = forjar::core::codegen::apply_script(r).expect("codegen");
    const OPEN: &str = "<<'FORJAR_REAPER_EOF'\n";
    const CLOSE: &str = "\nFORJAR_REAPER_EOF\n";
    let s = apply.find(OPEN).expect("not a disk_budget resource") + OPEN.len();
    let e = apply[s..].find(CLOSE).expect("heredoc end") + s;
    print!("{}", &apply[s..e]);
}
