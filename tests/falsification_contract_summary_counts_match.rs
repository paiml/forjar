//! `pv validate` rule VS-COUNT-001: a contract's `verification_summary.total_obligations`
//! must equal the length of its `proof_obligations` list.
//!
//! `blake3-state-v1` said 4 against 3 entries (the four `FALSIFY-B3-*` ids are
//! falsification tests, not obligations). The proofs lane then failed under
//! `bash -e` without printing why. Read from the YAML directly, so the check
//! runs without `pv` installed (see falsification_contracts_parse.rs).
use std::path::PathBuf;

fn contracts() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("contracts");
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("contracts/ is readable")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    out.sort();
    out
}

#[test]
fn every_summary_total_equals_the_obligation_list() {
    let mut wrong = Vec::new();
    let mut checked = 0;
    for path in contracts() {
        let text = std::fs::read_to_string(&path).expect("contract is readable");
        let Ok(doc) = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&text) else {
            continue;
        };
        let total = doc["verification_summary"]["total_obligations"].as_u64();
        let listed = doc["proof_obligations"]
            .as_sequence()
            .map(|s| s.len() as u64);
        if let (Some(total), Some(listed)) = (total, listed) {
            checked += 1;
            if total != listed {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                wrong.push(format!(
                    "{name}: total_obligations {total}, proof_obligations {listed}"
                ));
            }
        }
    }
    assert!(
        checked > 0,
        "no contract carries both a summary and an obligation list"
    );
    assert!(
        wrong.is_empty(),
        "VS-COUNT-001 would fail:\n{}",
        wrong.join("\n")
    );
}
