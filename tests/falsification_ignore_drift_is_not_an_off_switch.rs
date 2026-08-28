//! `lifecycle.ignore_drift` is a field list, not an off switch.
//!
//! forjar#335. The schema says `ignore_drift` names FIELDS whose drift is
//! suppressed. The engine read it as `!lifecycle.ignore_drift.is_empty()` —
//! any entry at all meant "stop looking at this resource entirely".
//!
//! So `ignore_drift: ["mode"]` — written to tolerate a mode change while
//! still catching content tampering — silently disabled content, owner,
//! group, existence and image drift as well. Narrowing the written exemption
//! widened the real one, and the narrowest thing an operator could write (one
//! misspelled field name) was the broadest exemption forjar could express.
//! Nothing rejected it either: `known_fields.rs` knew the KEY and no
//! validator looked at the values, so `forjar validate` printed a clean
//! verdict over a declaration that meant the opposite of what it said.
//!
//! Per-field suppression is genuinely not implementable yet — the lock stores
//! a DIGEST of the state query, not the per-field observation — so the fix is
//! to REFUSE the narrowed form rather than silently widen it. `["*"]` stays
//! the one honoured value.
//!
//! DRIVEN THROUGH THE REAL BINARY. The defect is that every user-visible
//! surface accepted the declaration and then did something else, so what the
//! user sees is the thing under test.

use std::fs;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sandbox {
    dir: std::path::PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-335-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("sandbox");
        Self { dir }
    }

    fn managed(&self) -> std::path::PathBuf {
        self.dir.join("external.conf")
    }

    /// One file resource carrying the given `ignore_drift` entries.
    fn write_config(&self, ignore_drift: &[&str]) {
        let entries: String = ignore_drift
            .iter()
            .map(|f| format!("      - \"{f}\"\n"))
            .collect();
        let cfg = format!(
            "version: \"1.0\"\nname: ignore-drift-repro\nmachines:\n  sandbox:\n\
             \x20   hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  external-config:\n\
             \x20   type: file\n    machine: sandbox\n    path: {}\n    content: |\n\
             \x20     replica_count=3\n    lifecycle:\n      ignore_drift:\n{}",
            self.managed().display(),
            entries
        );
        fs::write(self.dir.join("forjar.yaml"), cfg).expect("config");
    }

    /// Returns (combined output, exit success). The status is half the
    /// verdict: `validate` printing a warning and exiting 0 is exactly the
    /// silent acceptance this file exists to forbid.
    fn run(&self, args: &[&str]) -> (String, bool) {
        let out = Command::new(FORJAR)
            .args(args)
            .arg("-f")
            .arg(self.dir.join("forjar.yaml"))
            .current_dir(&self.dir)
            .output()
            .expect("run forjar");
        (
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            out.status.success(),
        )
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// THE FALSIFICATION. A narrowed `ignore_drift` must be refused, not accepted
/// and then read as skip-everything. Before the fix `validate` exited 0 with a
/// clean verdict over a declaration the engine would honour inside out.
#[test]
fn a_narrowed_ignore_drift_is_refused_not_silently_widened() {
    let sb = Sandbox::new("narrowed");
    sb.write_config(&["mode"]);

    let (out, ok) = sb.run(&["validate"]);

    assert!(
        !ok,
        "validate accepted ignore_drift: [mode], which the engine reads as \
         suppress-everything. A silently broader exemption is the defect.\n{out}"
    );
    assert!(
        out.contains("ignore_drift"),
        "the refusal must name the key it is refusing:\n{out}"
    );
    assert!(
        out.contains("335"),
        "the refusal must say per-field suppression is unimplemented, not \
         merely illegal — the operator needs to know which one:\n{out}"
    );
}

/// GUARDS OVER-REJECTION. `["*"]` is the one form forjar actually implements
/// and it must keep validating, or the fix has outlawed the escape hatch it
/// tells people to use.
#[test]
fn wildcard_ignore_drift_still_validates() {
    let sb = Sandbox::new("wildcard");
    sb.write_config(&["*"]);

    let (out, ok) = sb.run(&["validate"]);

    assert!(ok, "ignore_drift: [\"*\"] must stay legal:\n{out}");
    assert!(
        !out.contains("335"),
        "the wildcard is implemented; it must not be flagged:\n{out}"
    );
}

/// ISSUE ITEM (3). A typo was not a no-op and not an error — it was the
/// broadest possible exemption, because "modes" is a non-empty list like any
/// other. The refusal must echo the offending token back.
#[test]
fn a_typo_is_refused_rather_than_becoming_a_skip_all() {
    let sb = Sandbox::new("typo");
    sb.write_config(&["modes"]);

    let (out, ok) = sb.run(&["validate"]);

    assert!(!ok, "a misspelled field name was a skip-all:\n{out}");
    assert!(
        out.contains("modes"),
        "the message must name the token so the typo is findable:\n{out}"
    );
}

/// END TO END, through apply and the tripwire. The shape here is the one
/// shipped in examples/cookbook/33-lifecycle.yaml: `ignore_drift: [content]`
/// under a comment promising it only ignores content.
///
/// Before the fix, apply converged the file, the bytes were then changed on
/// disk, and `forjar drift` printed "No drift detected." over the tampered
/// file — the tripwire had been switched off by a declaration that asked for
/// one dimension. After the fix apply refuses at validation, so no path
/// through forjar ever calls the tampered file clean.
#[test]
fn a_narrowed_ignore_drift_never_reaches_the_tripwire() {
    let sb = Sandbox::new("tripwire");
    sb.write_config(&["content"]);

    let (apply_out, applied) = sb.run(&["apply", "--yes"]);

    if !applied {
        assert!(
            apply_out.contains("ignore_drift") && apply_out.contains("335"),
            "apply failed for some reason OTHER than the #335 refusal, so this \
             test proves nothing:\n{apply_out}"
        );
        return;
    }

    // Apply accepted the declaration. Then the tripwire owes us an answer
    // about the bytes it was told to keep watching.
    assert!(
        sb.managed().exists(),
        "precondition: apply must have written the file:\n{apply_out}"
    );
    fs::write(sb.managed(), "replica_count=9999\n").expect("tamper");

    let (drift_out, _) = sb.run(&["drift"]);

    assert!(
        !drift_out.contains("No drift detected."),
        "forjar reported a clean verdict over a file whose bytes were changed \
         under it. ignore_drift: [content] turned the whole tripwire off.\n\
         apply:\n{apply_out}\ndrift:\n{drift_out}"
    );
}
