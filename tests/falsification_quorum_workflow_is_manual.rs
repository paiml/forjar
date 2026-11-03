//! C58-3 (Noah ruling C58 item 3): the `Quorum` workflow is NOT a required check and was red on
//! every PR (it wants a committed `.quorum/<branch>.json` the fleet flow never writes; the fleet's
//! own head-bound receipt is unreadable from the ARM64 clean-room seats). It is therefore neutral:
//! manual `workflow_dispatch` only, no `pull_request` trigger. forjar#862.

use std::path::Path;

fn triggers() -> String {
    let w = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/quorum.yml"),
    )
    .expect("quorum.yml");
    // the `on:` block: from the line `on:` up to the next top-level key
    let mut out = String::new();
    let mut inside = false;
    for l in w.lines() {
        if l.starts_with("on:") {
            inside = true;
            continue;
        }
        if inside && !l.is_empty() && !l.starts_with(' ') && !l.starts_with('#') {
            break;
        }
        if inside && !l.trim_start().starts_with('#') {
            out.push_str(l);
            out.push('\n');
        }
    }
    out
}

#[test]
fn quorum_has_no_pull_request_trigger() {
    let t = triggers();
    for bad in [
        "pull_request",
        "pull_request_target",
        "push:",
        "merge_group",
    ] {
        assert!(
            !t.contains(bad),
            "quorum.yml must not trigger on {bad}; got:\n{t}"
        );
    }
}

#[test]
fn quorum_stays_runnable_by_hand() {
    assert!(
        triggers().contains("workflow_dispatch"),
        "workflow_dispatch must remain"
    );
}
