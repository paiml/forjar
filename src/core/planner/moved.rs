//! FJ-1210 / FJ-154 (#23): Apply and validate `moved` blocks.
//!
//! `moved` entries declaratively rename resource keys in lock state so a
//! rename does not show up as destroy+create in the plan. They must be applied
//! as an atomic permutation: chained (`a→b`, `b→c`) or colliding (`x→z`,
//! `y→z`) moves must never silently overwrite existing lock state.
//!
//! What is decidable WHERE matters here, and getting it wrong made the feature
//! unusable. From config alone you can decide: `from == to`, duplicate `from`,
//! duplicate `to`, and chains. You CANNOT decide whether a rename would clobber
//! converged state — that is a fact about the lock. A `to` that names a declared
//! resource is the REQUIRED shape of a rename, not a collision.

use crate::core::parser::ValidationError;
use crate::core::types::*;

/// FJ-1210: Apply moved blocks to rename resource keys in lock state.
///
/// Returns a new lock map with resource keys renamed according to moved
/// entries. Resolution is done in a single pass into a fresh map so that
/// chained or colliding entries cannot clobber state: each surviving key is
/// taken from the ORIGINAL lock exactly once. Validation
/// (`validate_moved_blocks`) rejects collisions/chains before we get here, so
/// in practice the only entries reaching this function form a clean rename
/// set; we still defend against clobbering if a caller skipped validation.
pub(super) fn apply_moved_blocks(
    moved: &[MovedEntry],
    locks: &std::collections::HashMap<String, StateLock>,
) -> std::collections::HashMap<String, StateLock> {
    if moved.is_empty() {
        return locks.clone();
    }

    let mut result = std::collections::HashMap::new();
    for (machine, lock) in locks {
        result.insert(machine.clone(), rename_lock(moved, lock, machine));
    }
    result
}

/// Rename one machine's lock by applying all `from → to` moves in a single
/// pass into a fresh `IndexMap`.
fn rename_lock(moved: &[MovedEntry], lock: &StateLock, machine: &str) -> StateLock {
    let mut new_lock = lock.clone();
    new_lock.resources = indexmap::IndexMap::with_capacity(lock.resources.len());

    // Index `from → to` for O(1) lookup; first mapping wins (validation
    // forbids duplicate `from`, so there is at most one in valid configs).
    let mut rename: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
    for entry in moved {
        rename.entry(entry.from.as_str()).or_insert(&entry.to);
    }

    // Single pass over the ORIGINAL resources: emit each under its renamed key
    // (or its own key if not moved). insert() never overwrites a key that has
    // not been emitted yet because validation guarantees `to` keys are unique
    // and disjoint from surviving (non-`from`) keys.
    for (id, rl) in &lock.resources {
        let new_key = match rename.get(id.as_str()) {
            Some(to) => {
                eprintln!("info: moved {id} → {to} in state for {machine}");
                (*to).to_string()
            }
            None => id.clone(),
        };
        // THE REAL COLLISION, and the only place it is decidable.
        //
        // Validation cannot see this: `moved: a → b` with `b` also declared in
        // config is the CANONICAL rename and perfectly safe when the lock holds
        // no `b`. It is destructive only when the lock holds BOTH — which is
        // state, not config. The old config-time check could not tell those
        // apart and so rejected every correct use of the feature.
        //
        // Losing an entry here would silently discard a resource's converged
        // state, so it is loud and it keeps the MOVED entry (the declared
        // intent) rather than whichever happened to be iterated last.
        if let Some(existing) = new_lock.resources.get(&new_key) {
            let moved_in = rename.contains_key(id.as_str());
            eprintln!(
                "warning: moved target '{new_key}' on {machine} already holds state \
                 (hash {}); {} entry kept. Remove the stale resource or pick a \
                 different target — two resources cannot share one state key.",
                &existing.hash.chars().take(12).collect::<String>(),
                if moved_in {
                    "the moved"
                } else {
                    "the existing"
                }
            );
            if !moved_in {
                continue;
            }
        }
        new_lock.resources.insert(new_key, rl.clone());
    }
    new_lock
}

/// FJ-154 (#23): Reject moved blocks that would silently corrupt lock state.
///
/// Errors are pushed for: duplicate `from`, duplicate `to`, a `to` that
/// collides with an existing (managed) resource id, and chains where a `to`
/// is also used as a `from` (transitive moves). `from == to` no-ops are
/// rejected as redundant.
///
/// Note (#165): the managed-resource collision check here runs against the
/// *pre-expansion* `config.resources`, so it catches collisions with literal
/// resources only. Collisions with recipe-expanded keys (e.g. `to:
/// myrecipe/foo`) are caught by [`validate_moved_targets`], which the parser
/// runs *after* `expand_recipes` / `expand_resources`.
pub(crate) fn validate_moved_blocks(config: &ForjarConfig, errors: &mut Vec<ValidationError>) {
    let moved = &config.moved;
    if moved.is_empty() {
        return;
    }

    let froms: std::collections::HashSet<&str> = moved.iter().map(|m| m.from.as_str()).collect();
    let mut seen_from: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut seen_to: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for entry in moved {
        let (from, to) = (entry.from.as_str(), entry.to.as_str());

        if from == to {
            errors.push(err(format!(
                "moved entry '{from}' → '{to}' is a no-op (from == to) — remove it"
            )));
            continue;
        }
        if !seen_from.insert(from) {
            errors.push(err(format!(
                "moved block has duplicate 'from: {from}' — each resource may be moved at most once"
            )));
        }
        if !seen_to.insert(to) {
            errors.push(err(format!(
                "moved block has colliding 'to: {to}' — two moves target the same resource id"
            )));
        }
        // NO config-time "collides with an existing resource" check here.
        //
        // It used to reject `to` whenever the config declared a resource by
        // that name — which is the CANONICAL usage and the only correct one:
        // after a rename the resource IS declared under its new name, and a
        // `to` that is not declared would point the rename at nothing.
        //
        // The stated concern ("would overwrite its converged state") is a claim
        // about the LOCK, and config-contains-`to` cannot decide it:
        //
        //   canonical rename   config has `to`: YES   lock has `to`: no   safe
        //   genuine clobber    config has `to`: YES   lock has `to`: YES  destructive
        //
        // True in both, so it discriminated nothing and forbade the feature's
        // only working shape — paiml/forjar-cookbook recipes/34-moved-blocks.yaml
        // (the documented example) failed validation. The real collision is
        // detected against the lock in `rename_lock`, which has it.
        // Chained move: this entry's `to` is some other entry's `from`.
        if to != from && froms.contains(to) {
            errors.push(err(format!(
                "moved block is chained: 'to: {to}' is also a 'from' — \
                 chained renames (a→b, b→c) are order-dependent and not allowed; \
                 declare the final rename directly (a→c)"
            )));
        }
    }
}

// #165's `validate_moved_targets` is removed: it re-ran the config-time
// collision check against the POST-expansion resource set, so a `moved.to`
// landing on a recipe-expanded key (`recipe_id/foo`) was rejected too.
//
// It went with the check it duplicated. The premise — that `to` naming a
// declared resource is destructive — is false; that is the required shape of a
// rename, expanded keys included. Such a `to` is destructive only when the LOCK
// already holds the key, which `rename_lock` now detects with the state in hand.

fn err(message: String) -> ValidationError {
    ValidationError { message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parser::{parse_config, validate_config};

    fn test_lock(machine: &str) -> StateLock {
        StateLock {
            schema: "1".to_string(),
            machine: machine.to_string(),
            hostname: machine.to_string(),
            generated_at: String::new(),
            generator: "test".to_string(),
            blake3_version: "1".to_string(),
            resources: indexmap::IndexMap::new(),
        }
    }

    fn rl(hash: &str) -> ResourceLock {
        ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: hash.to_string(),
            observed: None,
            details: std::collections::HashMap::new(),
        }
    }

    fn locks_with(
        machine: &str,
        entries: &[(&str, &str)],
    ) -> std::collections::HashMap<String, StateLock> {
        let mut lock = test_lock(machine);
        for (id, hash) in entries {
            lock.resources.insert((*id).to_string(), rl(hash));
        }
        let mut locks = std::collections::HashMap::new();
        locks.insert(machine.to_string(), lock);
        locks
    }

    fn errors_for(yaml: &str) -> Vec<String> {
        let config = parse_config(yaml).expect("yaml parses");
        validate_config(&config)
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    // -- apply_moved_blocks correctness (#23) -------------------------------

    #[test]
    fn chained_moves_resolve_in_single_pass_without_clobber() {
        // a→b and b→c declared together. The OLD sequential insert() would
        // move a's lock to key b (clobbering original b), then move that to c
        // — landing a's history under c and losing b. The single-pass version
        // reads from the ORIGINAL lock: a→b, b→c, each from its true source.
        let locks = locks_with("m1", &[("a", "ha"), ("b", "hb")]);
        let moved = vec![
            MovedEntry {
                from: "a".into(),
                to: "b".into(),
            },
            MovedEntry {
                from: "b".into(),
                to: "c".into(),
            },
        ];

        let result = apply_moved_blocks(&moved, &locks);
        let m1 = &result["m1"].resources;

        assert_eq!(
            m1.get("b").map(|r| r.hash.as_str()),
            Some("ha"),
            "a's state lands at b"
        );
        assert_eq!(
            m1.get("c").map(|r| r.hash.as_str()),
            Some("hb"),
            "b's state lands at c (not lost)"
        );
        assert!(!m1.contains_key("a"), "source a removed");
        assert_eq!(m1.len(), 2, "no spurious or dropped keys");
    }

    #[test]
    fn colliding_targets_do_not_overwrite_each_others_source() {
        // x→z and y→z both target z. Single-pass emits the first surviving
        // mapping; the point is neither silently clobbers an UNRELATED key.
        let locks = locks_with("m1", &[("x", "hx"), ("y", "hy"), ("keep", "hk")]);
        let moved = vec![
            MovedEntry {
                from: "x".into(),
                to: "z".into(),
            },
            MovedEntry {
                from: "y".into(),
                to: "z".into(),
            },
        ];
        let result = apply_moved_blocks(&moved, &locks);
        let m1 = &result["m1"].resources;
        // Unrelated managed resource is untouched.
        assert_eq!(m1.get("keep").map(|r| r.hash.as_str()), Some("hk"));
    }

    #[test]
    fn simple_rename_preserves_hash_and_status() {
        let locks = locks_with("m1", &[("old", "h1")]);
        let moved = vec![MovedEntry {
            from: "old".into(),
            to: "new".into(),
        }];
        let result = apply_moved_blocks(&moved, &locks);
        let m1 = &result["m1"].resources;
        assert!(!m1.contains_key("old"));
        let new = m1.get("new").expect("renamed key exists");
        assert_eq!(new.hash, "h1");
        assert_eq!(new.status, ResourceStatus::Converged);
    }

    #[test]
    fn empty_moved_is_clone() {
        let locks = locks_with("m1", &[("a", "h")]);
        let result = apply_moved_blocks(&[], &locks);
        assert_eq!(result["m1"].resources.len(), 1);
    }

    // -- validate_moved_blocks rejection (#23) ------------------------------

    const HEADER: &str = "version: \"1.0\"\nname: t\n";

    #[test]
    fn validate_rejects_colliding_to() {
        let yaml = format!("{HEADER}moved:\n  - from: x\n    to: z\n  - from: y\n    to: z\n");
        let errs = errors_for(&yaml);
        assert!(
            errs.iter().any(|m| m.contains("colliding 'to: z'")),
            "expected colliding-to error, got {errs:?}"
        );
    }

    #[test]
    fn validate_rejects_chained_moves() {
        let yaml = format!("{HEADER}moved:\n  - from: a\n    to: b\n  - from: b\n    to: c\n");
        let errs = errors_for(&yaml);
        assert!(
            errs.iter().any(|m| m.contains("chained")),
            "expected chained-move error, got {errs:?}"
        );
    }

    #[test]
    fn validate_accepts_to_naming_the_renamed_resource() {
        // INVERTED from what this test used to assert.
        //
        // It required an error when `to` named a declared resource — which is
        // the REQUIRED shape of a rename: after `old → existing`, the resource
        // IS declared as `existing`. A `to` that is not declared would point
        // the rename at nothing and the resource would be destroyed.
        //
        // The old error claimed the rename "would overwrite its converged
        // state", which is a fact about the LOCK. Config-contains-`to` is true
        // both when the lock holds `to` (destructive) and when it does not
        // (the normal rename), so it decided nothing and forbade the feature's
        // only working shape. forjar's own cookbook example
        // (recipes/34-moved-blocks.yaml) failed validation because of it.
        let yaml = format!(
            "{HEADER}resources:\n  existing:\n    type: file\n    path: /tmp/e\n    content: x\n\
             moved:\n  - from: old\n    to: existing\n"
        );
        let errs = errors_for(&yaml);
        assert!(
            errs.is_empty(),
            "the canonical rename must validate, got {errs:?}"
        );
    }

    #[test]
    fn validate_rejects_duplicate_from() {
        let yaml = format!("{HEADER}moved:\n  - from: a\n    to: b\n  - from: a\n    to: c\n");
        let errs = errors_for(&yaml);
        assert!(
            errs.iter().any(|m| m.contains("duplicate 'from: a'")),
            "expected duplicate-from error, got {errs:?}"
        );
    }

    #[test]
    fn validate_rejects_noop_move() {
        let yaml = format!("{HEADER}moved:\n  - from: a\n    to: a\n");
        let errs = errors_for(&yaml);
        assert!(
            errs.iter().any(|m| m.contains("no-op")),
            "expected no-op error, got {errs:?}"
        );
    }

    #[test]
    fn validate_accepts_clean_renames() {
        // a→a2 and b→b2: distinct froms, distinct tos, no chains/collisions.
        let yaml = format!("{HEADER}moved:\n  - from: a\n    to: a2\n  - from: b\n    to: b2\n");
        let errs = errors_for(&yaml);
        assert!(
            !errs.iter().any(|m| m.contains("moved")),
            "clean renames must not error, got {errs:?}"
        );
    }

    // -- #165: moved.to vs POST-expansion (recipe) resource keys ------------

    /// Write a `setup` recipe (expanding to `setup/config-file`) plus the
    /// given `moved:` block to a config in a tempdir, then run the full
    /// parse+validate+expand pipeline (`parse_and_validate`). Returns the
    /// pipeline result so tests can assert acceptance or the collision error.
    fn parse_and_validate_with_recipe(moved_block: &str) -> Result<ForjarConfig, String> {
        use crate::core::parser::parse_and_validate;
        let dir = tempfile::tempdir().unwrap();
        let recipes_dir = dir.path().join("recipes");
        std::fs::create_dir_all(&recipes_dir).unwrap();
        std::fs::write(
            recipes_dir.join("test-recipe.yaml"),
            "recipe:\n  name: test-recipe\nresources:\n  config-file:\n    \
             type: file\n    path: /etc/test.conf\n    content: hello\n",
        )
        .unwrap();

        let cfg = dir.path().join("forjar.yaml");
        std::fs::write(
            &cfg,
            format!(
                "version: \"1.0\"\nname: recipe-test\nmachines:\n  m1:\n    \
                 hostname: box\n    addr: 1.2.3.4\nresources:\n  setup:\n    \
                 type: recipe\n    machine: m1\n    recipe: test-recipe\n{moved_block}"
            ),
        )
        .unwrap();

        parse_and_validate(&cfg)
    }

    #[test]
    fn validate_accepts_to_naming_a_recipe_expanded_key() {
        // Same inversion, post-expansion (#165). `to: setup/config-file` names
        // a key that exists only after recipe expansion — which is exactly what
        // renaming a recipe-produced resource looks like, not a collision.
        let result =
            parse_and_validate_with_recipe("moved:\n  - from: old\n    to: setup/config-file\n");
        assert!(
            result.is_ok(),
            "a rename onto an expanded key must validate; the destructive case \
             (the LOCK already holding it) is caught in rename_lock: {:?}",
            result.err()
        );
    }

    #[test]
    fn validate_accepts_non_colliding_rename_alongside_recipe() {
        // A legitimate rename whose `to` does NOT collide with any expanded
        // key must still pass the full pipeline.
        let result = parse_and_validate_with_recipe("moved:\n  - from: old\n    to: brand-new\n");
        let config = result.expect("non-colliding rename must pass");
        assert!(config.resources.contains_key("setup/config-file"));
    }
}
