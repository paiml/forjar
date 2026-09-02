//! FJ-409 (E06): Store is not scored
//!
//! WHAT WAS OBSERVABLY WRONG:
//! The `store: true` flag was falsely increasing the reproducibility score and purity level
//! (upgrading it to Pure) even though no actual store behavior was enforced for `apt` packages.
//! Two configs that yielded byte-identical apply scripts were scoring differently (68 vs 38).
//!
//! WHY THESE ASSERTIONS:
//! We assert that parsing a config with `store: true` and one without it results in the
//! exact same reproducibility score, verifying that declared-but-not-enforced flags do not
//! skew the score.

use forjar::core::store::purity::{classify, PurityLevel, PuritySignals};
use forjar::core::store::repro_score::{compute_score, ReproInput};

#[test]
fn test_e06_store_flag_does_not_affect_score() {
    let signals_plain = PuritySignals {
        has_version: true,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    };

    let signals_with_store = PuritySignals {
        has_version: true,
        has_store: true, // This is the only difference
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    };

    let result_plain = classify("ripgrep", &signals_plain);
    let result_store = classify("ripgrep", &signals_with_store);

    // They should have the same purity level
    assert_eq!(
        result_plain.level, result_store.level,
        "Purity level should not change just because store: true is declared"
    );
    assert_eq!(result_store.level, PurityLevel::Pinned);

    let score_plain = compute_score(&[ReproInput {
        name: "ripgrep".to_string(),
        purity: result_plain.level,
        has_store: false,
        has_lock_pin: true,
    }]);

    let score_store = compute_score(&[ReproInput {
        name: "ripgrep".to_string(),
        purity: result_store.level,
        has_store: true,
        has_lock_pin: true,
    }]);

    // The composite score must be completely equal
    assert!(
        (score_plain.composite - score_store.composite).abs() < 0.001,
        "Scores must be equal for configs differing only by store: true"
    );
}

// ── The ticket's own success criterion, end to end ─────────────────────────
//
// Two configs identical but for `store: true` on one resource: the apply
// scripts codegen emits must be byte-identical (nothing on the apply path
// honours the flag), and the shipped `validate --check-reproducibility-score`
// must give them the SAME composite. On 1.24.0 the second half was 68 vs 38.

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

fn config_yaml(store: bool) -> String {
    let store_line = if store { "    store: true\n" } else { "" };
    format!(
        "version: \"1.0\"\nname: e06\nmachines:\n  local:\n    hostname: local\n    addr: 127.0.0.1\n\
resources:\n  ripgrep:\n    type: package\n    provider: apt\n    packages: [ripgrep]\n    version: \"14.1.0\"\n{store_line}"
    )
}

fn composite_for(dir: &std::path::Path, name: &str, store: bool) -> f64 {
    let cfg = dir.join(name);
    std::fs::write(&cfg, config_yaml(store)).expect("write config");
    let out = std::process::Command::new(FORJAR)
        .args(["validate", "-f"])
        .arg(&cfg)
        .args(["--check-reproducibility-score", "--json"])
        .output()
        .expect("run forjar");
    let text = String::from_utf8_lossy(&out.stdout);
    let start = text
        .find('{')
        .unwrap_or_else(|| panic!("no JSON in: {text}"));
    let v: serde_json::Value = serde_json::from_str(&text[start..]).expect("score json");
    v["composite"].as_f64().expect("composite")
}

#[test]
fn two_byte_identical_apply_scripts_score_equal() {
    use forjar::core::codegen::apply_script;
    use forjar::core::parser::parse_config;

    let plain = parse_config(&config_yaml(false)).expect("parse plain");
    let stored = parse_config(&config_yaml(true)).expect("parse stored");
    let script_plain = apply_script(&plain.resources["ripgrep"]).expect("codegen plain");
    let script_stored = apply_script(&stored.resources["ripgrep"]).expect("codegen stored");
    assert_eq!(
        script_plain, script_stored,
        "`store: true` changed the apply script — then it IS on the apply path and must be scored"
    );

    let dir = std::env::temp_dir().join(format!("forjar-e06-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tmp");
    let a = composite_for(&dir, "plain.yaml", false);
    let b = composite_for(&dir, "stored.yaml", true);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(
        (a - b).abs() < 0.001,
        "byte-identical apply scripts scored {a} vs {b}: the score credits a flag nothing enforces"
    );
}
