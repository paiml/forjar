//! forjar#847: a `tmp_reaper` resource through the real `apply`, twice.

use super::apply::cmd_apply;
use std::path::Path;
use std::process::Command;

fn apply(file: &Path, state: &Path) -> Result<(), String> {
    cmd_apply(
        file,
        state,
        None,
        None,
        None,
        None,
        false,
        false,
        false,
        &[],
        false,
        None,
        false,
        false,
        None,
        None,
        false,
        false,
        None,
        false,
        false,
        0,
        true,
        false,
        None,
        false,
        None,
        None,
        None,
        false,
        None,
        false,
        None,
        false,
        None,
        &[],
    )
}

#[test]
fn the_second_apply_of_a_tmp_reaper_resource_changes_nothing() {
    let d = tempfile::tempdir().unwrap();
    let reap = d.path().join("reap");
    std::fs::create_dir(&reap).unwrap();
    let (old, new) = (reap.join("old"), reap.join("new"));
    std::fs::write(&old, "x").unwrap();
    std::fs::write(&new, "x").unwrap();
    assert!(Command::new("touch")
        .args(["-d", "10 days ago"])
        .arg(&old)
        .status()
        .unwrap()
        .success());
    let file = d.path().join("forjar.yaml");
    std::fs::write(
        &file,
        format!(
            "version: \"1.0\"\nname: t\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n\
             resources:\n  reap:\n    type: tmp_reaper\n    machine: local\n    path: {}\n    max_age: 7d\n",
            reap.display()
        ),
    )
    .unwrap();
    let state = d.path().join("state");

    apply(&file, &state).unwrap();
    assert!(
        !old.exists() && new.exists(),
        "first apply must reap old and keep new"
    );

    // Re-plan against the lock the first apply wrote: nothing to do.
    let config = super::helpers::parse_and_validate(&file).unwrap();
    let order = crate::core::resolver::build_execution_order(&config).unwrap();
    let locks = super::helpers_state::load_machine_locks(&config, &state, None).unwrap();
    let plan = crate::core::planner::plan(&config, &order, &locks, None);
    assert_eq!(
        (
            plan.to_create,
            plan.to_update,
            plan.to_destroy,
            plan.unchanged
        ),
        (0, 0, 0, 1),
        "{:?}",
        plan.changes
    );

    apply(&file, &state).unwrap();
    assert!(new.exists());
}
