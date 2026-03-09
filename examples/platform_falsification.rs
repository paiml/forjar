//! Platform-wide falsification: verifies key claims from forjar-platform-spec.md.
//!
//! Tests SQLite query, generation model, config validation, task gates, and scoring
//! in a single runnable example. Every assertion maps to a spec rejection criterion.
//!
//! Usage: cargo run --example platform_falsification

use forjar::core::scoring::{compute, RuntimeData, ScoringInput, SCORE_VERSION};
use forjar::core::store::db::{fts5_search, open_state_db, SCHEMA_VERSION};
use forjar::core::types::{
    diff_resource_sets, DiffAction, ForjarConfig, GenerationDiff, GenerationMeta, MachineDelta,
    ResourceDiff,
};
use tempfile::TempDir;

fn main() {
    println!("Forjar Platform Falsification — forjar-platform-spec.md");
    println!("{}", "=".repeat(58));

    // ── FJ-2001: SQLite Query Engine ──
    println!("\n[FJ-2001] SQLite Query Engine:");

    let dir = TempDir::new().unwrap();
    let conn = open_state_db(&dir.path().join("state.db")).unwrap();
    println!("  Schema version: {SCHEMA_VERSION}");
    assert!(SCHEMA_VERSION > 0);

    // FTS5 works on fresh DB
    let results = fts5_search(&conn, "nginx", 10).unwrap();
    assert!(results.is_empty());
    println!("  FTS5 search on empty DB: ✓ (returns empty, not error)");

    // Insert and query
    conn.execute(
        "INSERT INTO machines (id, name, hostname, transport, first_seen, last_seen) \
         VALUES (1, 'web', 'web-01', 'ssh', '2026-03-09', '2026-03-09')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO generations (id, generation_num, run_id, config_hash, created_at) \
         VALUES (1, 1, 'run-1', 'hash0', '2026-03-09')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO resources (machine_id, resource_id, generation_id, resource_type, status, state_hash, applied_at) \
         VALUES (1, 'nginx-config', 1, 'file', 'converged', 'h1', '2026-03-09')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO resources_fts (resource_id, resource_type, path) \
         VALUES ('nginx-config', 'file', '/etc/nginx/nginx.conf')",
        [],
    )
    .unwrap();
    let results = fts5_search(&conn, "nginx", 10).unwrap();
    assert_eq!(results.len(), 1);
    println!("  FTS5 search after insert: ✓ (found 'nginx-config')");

    // ── FJ-2002/2003: Generation Model ──
    println!("\n[FJ-2002] Generation Model:");

    let mut meta = GenerationMeta::new(5, "2026-03-09T12:00:00Z".into());
    meta.config_hash = Some("blake3:abc123".into());
    meta.record_machine(
        "web",
        MachineDelta {
            created: vec!["nginx".into()],
            updated: vec!["config".into()],
            destroyed: vec![],
            unchanged: 3,
        },
    );
    assert_eq!(meta.total_changes(), 2);
    println!(
        "  GenerationMeta with deltas: ✓ (total_changes={})",
        meta.total_changes()
    );

    let yaml = serde_yaml_ng::to_string(&meta).unwrap();
    let roundtrip: GenerationMeta = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(roundtrip.generation, 5);
    assert_eq!(roundtrip.config_hash.as_deref(), Some("blake3:abc123"));
    println!(
        "  YAML roundtrip: ✓ (gen={}, hash preserved)",
        roundtrip.generation
    );

    // ── FJ-2003: Generation Diff ──
    println!("\n[FJ-2003] Generation Diff:");

    let old = vec![("a", "file", "h1"), ("b", "pkg", "h2"), ("c", "svc", "h3")];
    let new = vec![("a", "file", "h1"), ("b", "pkg", "hX"), ("d", "svc", "h4")];
    let diffs = diff_resource_sets(&old, &new);
    let added = diffs
        .iter()
        .filter(|d| d.action == DiffAction::Added)
        .count();
    let modified = diffs
        .iter()
        .filter(|d| d.action == DiffAction::Modified)
        .count();
    let removed = diffs
        .iter()
        .filter(|d| d.action == DiffAction::Removed)
        .count();
    println!("  Diff: +{added} ~{modified} -{removed}  ✓");
    assert_eq!(added, 1);
    assert_eq!(modified, 1);
    assert_eq!(removed, 1);

    let gen_diff = GenerationDiff {
        gen_from: 3,
        gen_to: 7,
        machine: "web".into(),
        resources: vec![
            ResourceDiff::added("new", "file"),
            ResourceDiff::removed("old", "pkg"),
            ResourceDiff::modified("cfg", "file"),
            ResourceDiff::unchanged("stable", "svc"),
        ],
    };
    assert_eq!(gen_diff.change_count(), 3);
    assert!(gen_diff.has_changes());
    println!(
        "  GenerationDiff(3→7): ✓ (changes={})",
        gen_diff.change_count()
    );

    // ── FJ-2500: Config Validation ──
    println!("\n[FJ-2500] Config Validation:");

    use forjar::core::parser::check_unknown_fields;
    let issues = check_unknown_fields("machnes:\n  web: {}\nresources: {}\n");
    assert!(!issues.is_empty());
    println!("  Unknown field 'machnes' detected: ✓");

    let issues =
        check_unknown_fields("machines:\n  web:\n    hostname: h\n    addr: a\nresources: {}\n");
    assert!(issues.is_empty());
    println!("  Valid fields pass cleanly: ✓");

    // ── FJ-2700: Quality Gates ──
    println!("\n[FJ-2700] Quality Gates:");

    use forjar::core::task::{evaluate_gate, GateResult};
    use forjar::core::types::QualityGate;

    assert_eq!(
        evaluate_gate(&QualityGate::default(), 0, ""),
        GateResult::Pass
    );
    assert!(matches!(
        evaluate_gate(&QualityGate::default(), 1, ""),
        GateResult::Fail(_, _)
    ));
    println!("  Exit code gate: ✓ (0=pass, 1=fail)");

    let json_gate = QualityGate {
        parse: Some("json".into()),
        field: Some("coverage".into()),
        min: Some(80.0),
        ..Default::default()
    };
    assert_eq!(
        evaluate_gate(&json_gate, 0, r#"{"coverage":95}"#),
        GateResult::Pass
    );
    println!("  JSON threshold gate: ✓ (95 >= 80)");

    // ── FJ-2803: ForjarScore v2 ──
    println!("\n[FJ-2803] ForjarScore v{SCORE_VERSION}:");

    let config = ForjarConfig::default();
    let input = ScoringInput {
        status: "qualified".into(),
        idempotency: "strong".into(),
        budget_ms: 60_000,
        runtime: Some(RuntimeData {
            validate_pass: true,
            plan_pass: true,
            first_apply_pass: true,
            second_apply_pass: true,
            zero_changes_on_reapply: true,
            hash_stable: true,
            all_resources_converged: true,
            state_lock_written: true,
            warning_count: 0,
            changed_on_reapply: 0,
            first_apply_ms: 20_000,
            second_apply_ms: 500,
        }),
        raw_yaml: None,
    };
    let result = compute(&config, &input);
    println!(
        "  Grade: {} (static={}, runtime={})",
        result.grade,
        result.static_composite,
        result.runtime_composite.unwrap_or(0),
    );
    assert!(result.grade.contains('/'));
    assert_eq!(SCORE_VERSION, "2.0");
    println!("  Two-tier format: ✓");

    println!("\n{}", "=".repeat(58));
    println!("All platform falsification criteria survived.");
}
