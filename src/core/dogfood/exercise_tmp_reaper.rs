//! `tmp_reaper` dogfood exercise (forjar#847): the generated shell run against
//! a real tree.

use super::Outcome;
use crate::core::types::{Resource, ResourceType};
use std::fs;
use std::process::Command;

fn outcome(t: &ResourceType, passed: bool, detail: &str) -> Outcome {
    Outcome {
        resource_type: t.to_string(),
        passed,
        detail: detail.to_string(),
    }
}

/// Exercise the real `apply_script`.
pub(super) fn run(t: &ResourceType) -> Outcome {
    run_with(t, &crate::resources::tmp_reaper::apply_script)
}

/// Exercise `script_for` (injectable so a test can feed it a broken reaper).
pub(super) fn run_with(t: &ResourceType, script_for: &dyn Fn(&Resource) -> String) -> Outcome {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "forjar-dogfood-tmp_reaper-{}-{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&dir);
    if fs::create_dir_all(&dir).is_err() {
        return outcome(t, false, "could not create scratch dir");
    }
    let (old, new) = (dir.join("old"), dir.join("new"));
    if fs::write(&old, "x").is_err() || fs::write(&new, "x").is_err() {
        return outcome(t, false, "could not seed files");
    }
    let aged = Command::new("touch")
        .args(["-d", "10 days ago"])
        .arg(&old)
        .status();
    if !aged.map(|s| s.success()).unwrap_or(false) {
        return outcome(t, false, "could not age the fixture");
    }
    let r = Resource {
        resource_type: ResourceType::TmpReaper,
        path: Some(dir.to_string_lossy().into_owned()),
        max_age: Some("7d".into()),
        ..Default::default()
    };
    let script = script_for(&r);
    let run = |s: &str| Command::new("bash").arg("-c").arg(s).output();
    let first = run(&script);
    let second = run(&script);
    let root_refused = crate::resources::tmp_reaper::spec_of(&Resource {
        path: Some("/".into()),
        ..r.clone()
    })
    .is_err();
    // Observe the tree BEFORE it is removed.
    let (old_gone, new_kept) = (!old.exists(), new.exists());
    let _ = fs::remove_dir_all(&dir);
    let said = |o: &std::io::Result<std::process::Output>, want: &str| {
        o.as_ref()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(want))
    };
    let good = said(&first, "removed 1 file(s), 1 inode(s)")
        && said(&second, "removed 0 file(s), 0 inode(s)")
        && old_gone
        && new_kept
        && root_refused;
    if good {
        outcome(
            t,
            true,
            "old removed, new kept, second pass removed 0, / refused",
        )
    } else {
        outcome(
            t,
            false,
            "reaper misbehaved: old kept, new removed, wrong counts, or / accepted",
        )
    }
}
