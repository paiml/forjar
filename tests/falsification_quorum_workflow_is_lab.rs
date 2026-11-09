//! forjar#862 / #876: the `quorum receipt` check is LAB (GATE/LAB rule C324).
//!
//! It is not required on main, so it cannot block a merge; running it on every
//! pull_request only paints PRs red. It runs nightly and by hand instead.

fn workflow() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.github/workflows/quorum.yml"
    ))
    .expect("read quorum.yml")
}

/// The `on:` block, comments stripped.
fn triggers() -> String {
    let w = workflow();
    let mut out = String::new();
    let mut inside = false;
    for l in w.lines() {
        let t = l.trim_end();
        if t.starts_with("on:") {
            inside = true;
            continue;
        }
        if inside {
            if !t.is_empty() && !t.starts_with(' ') && !t.starts_with('#') {
                break;
            }
            if !t.trim_start().starts_with('#') {
                out.push_str(t);
                out.push('\n');
            }
        }
    }
    out
}

#[test]
fn quorum_does_not_run_on_pull_request() {
    assert!(
        !triggers().contains("pull_request"),
        "quorum.yml still triggers on pull_request:\n{}",
        triggers()
    );
}

#[test]
fn quorum_runs_nightly() {
    assert!(
        triggers().contains("schedule:") && triggers().contains("cron: \"17 3 * * *\""),
        "{}",
        triggers()
    );
}

#[test]
fn quorum_can_be_dispatched_by_hand_with_a_pr_number() {
    let t = triggers();
    assert!(
        t.contains("workflow_dispatch:") && t.contains("pr_number:"),
        "{t}"
    );
}

#[test]
fn quorum_job_runs_only_where_the_receipt_store_is_mounted() {
    let w = workflow();
    let line = w
        .lines()
        .find(|l| l.trim_start().starts_with("runs-on:"))
        .expect("runs-on line");
    assert!(
        line.contains("lambda-night"),
        "job can land on a store-less runner: {line}"
    );
    assert!(
        !line.contains("ARM64"),
        "ARM64 gx10 seats have no receipt store: {line}"
    );
}
