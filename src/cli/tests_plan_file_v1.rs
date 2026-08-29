//! Refs #358 round four — what a `forjar-plan-v1` document may not claim, and
//! what a plan body must round-trip.
//!
//! Split out of `tests_plan_file.rs`, which was at 511 lines against a 500-line
//! gate.

use super::plan_file::*;
use crate::core::plan_selectors::PlanSelectors;
use crate::core::types::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_config() -> ForjarConfig {
        let config_yaml = r#"
version: "1.0"
name: test-plan-file
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  web-pkg:
    type: package
    machine: m1
    packages: [nginx]
  web-config:
    type: file
    machine: m1
    path: /etc/nginx/nginx.conf
    content: "server {}"
"#;
        crate::core::parser::parse_config(config_yaml).unwrap()
    }

    fn compute_config_hash(config: &ForjarConfig) -> String {
        crate::core::config_hash::config_hash(config).expect("hashable")
    }

    fn write_doc(path: &Path, doc: &serde_json::Value) {
        std::fs::write(path, serde_json::to_string_pretty(doc).unwrap()).unwrap();
    }

    /// EVERY `ResourceType`, not a hand-picked ten.
    ///
    /// Refs #358: the reader's `match` knew 12 of the 21 variants and silently
    /// mapped the rest to `File`, and the previous version of this test listed
    /// exactly ten of the twelve it knew — so the suite agreed with the reader
    /// about a schema neither of them had. The list is now
    /// [`ResourceType::ALL`], which `core::types::resource_type_all` checks
    /// against the enum's own derive, so adding a variant without teaching the
    /// reader fails here rather than shipping a plan file that cannot be
    /// applied.
    #[test]
    fn every_resource_type_round_trips_through_a_plan_file() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();

        for expected in ResourceType::ALL {
            let plan_json = serde_json::json!({
                "format": FORMAT_V1,
                "config_hash": compute_config_hash(&config),
                "name": "test",
                "to_create": 1, "to_update": 0, "to_destroy": 0, "unchanged": 0,
                "execution_order": ["r"],
                "changes": [
                    {"resource_id": "r", "machine": "m1", "resource_type": expected, "action": "create", "description": "r: create"},
                ],
            });
            write_doc(&plan_path, &plan_json);
            let loaded = load_plan_file(&plan_path, &config, dir.path()).unwrap();
            assert_eq!(
                loaded.plan.changes[0].resource_type, expected,
                "type did not round-trip: {expected}"
            );
        }
    }

    /// THE ROUND-FOUR DEFECT, at the seam it broke: a plan of a config holding
    /// a resource type the reader did not know reconstructed as `File`, and the
    /// diff leg is computed over the RECONSTRUCTION, so an honest untouched
    /// plan failed its own seal. Measured on the branch binary before the fix,
    /// with a `task` resource and nobody having edited anything:
    ///
    /// ```text
    ///   error: PLAN_HASH_MISMATCH: the plan body was modified after it was sealed
    /// ```
    #[test]
    fn a_sealed_plan_of_every_resource_type_verifies_against_its_own_seal() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_test_config();
        for kind in ResourceType::ALL {
            let plan = ExecutionPlan {
                name: "test-plan-file".to_string(),
                changes: vec![PlannedChange {
                    resource_id: "web-pkg".to_string(),
                    machine: "m1".to_string(),
                    resource_type: kind.clone(),
                    action: PlanAction::Create,
                    description: "web-pkg: install nginx".to_string(),
                }],
                execution_order: vec!["web-pkg".to_string()],
                to_create: 1,
                to_update: 0,
                to_destroy: 0,
                unchanged: 0,
            };
            let plan_path = dir.path().join("plan.json");
            save_plan_file(
                &plan,
                &PlanSelectors::default(),
                &config,
                Path::new("forjar.yaml"),
                dir.path(),
                &plan_path,
            )
            .unwrap();
            load_plan_file(&plan_path, &config, dir.path())
                .unwrap_or_else(|e| panic!("a freshly sealed '{kind}' plan must load: {e}"));
        }
    }

    /// An action or a type this build cannot read is refused, not defaulted.
    #[test]
    fn an_unreadable_change_field_is_refused_rather_than_guessed_at() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();
        for (field, value) in [("action", "converge"), ("resource_type", "quantum")] {
            let mut change = serde_json::json!({
                "resource_id": "r", "machine": "m1",
                "resource_type": "file", "action": "create", "description": "r: create",
            });
            change[field] = serde_json::json!(value);
            let plan_json = serde_json::json!({
                "format": FORMAT_V1,
                "config_hash": compute_config_hash(&config),
                "name": "test",
                "to_create": 1, "to_update": 0, "to_destroy": 0, "unchanged": 0,
                "execution_order": ["r"],
                "changes": [change],
            });
            write_doc(&plan_path, &plan_json);
            let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
            assert!(err.starts_with("PLAN_MALFORMED:"), "{field}: {err}");
            assert!(err.contains(field), "{field}: {err}");
        }
    }

    /// A v1 body with `to_create: 1` and one create is honest; keep that green
    /// so the refusals below are known to be about the v2 keys and nothing else.
    fn v1_doc(config: &ForjarConfig) -> serde_json::Value {
        serde_json::json!({
            "format": FORMAT_V1,
            "config_hash": compute_config_hash(config),
            "name": "test",
            "to_create": 1, "to_update": 0, "to_destroy": 0, "unchanged": 0,
            "execution_order": ["r"],
            "changes": [
                {"resource_id": "r", "machine": "m1", "resource_type": "file", "action": "create", "description": "r: create"},
            ],
        })
    }

    #[test]
    fn a_v1_document_carrying_no_v2_key_still_loads_unfiltered() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();
        write_doc(&plan_path, &v1_doc(&config));
        let loaded = load_plan_file(&plan_path, &config, dir.path()).unwrap();
        assert!(!loaded.sealed);
        assert!(
            loaded.selectors.is_unfiltered(),
            "a v1 plan is checked against the whole config"
        );
    }

    /// THE BLOCKER (Refs #358, round four). `load_plan_file` read `selectors`
    /// unconditionally, BEFORE the `sealed` branch, so an unsealed document
    /// chose the filters `check_plan_still_holds` re-planned under and the
    /// re-plan then agreed with it. Measured on the branch binary: a v1
    /// document naming only `alpha`, plus
    /// `"selectors": {"machine":null,"resource":"alpha","tag":null,"group":null}`,
    /// printed `Plan applied: 1 converged, 1 unchanged, 0 failed` and exited 0
    /// with `bravo`'s create still pending; the same document with those four
    /// lines deleted was refused `PLAN_STALE`.
    #[test]
    fn a_v1_document_claiming_selectors_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();
        let mut doc = v1_doc(&config);
        doc["selectors"] =
            serde_json::json!({"machine": null, "resource": "r", "tag": null, "group": null});
        write_doc(&plan_path, &doc);
        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(err.starts_with("PLAN_MALFORMED:"), "{err}");
        assert!(err.contains("selectors"), "{err}");
    }

    /// The same rule for the other key v1 predates: a `seal` on a v1 document
    /// is never verified, so carrying one is a claim to an authentication the
    /// document does not get.
    #[test]
    fn a_v1_document_carrying_a_seal_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();
        let mut doc = v1_doc(&config);
        doc["seal"] = serde_json::json!({"version": "forjar-plan-seal-v1"});
        write_doc(&plan_path, &doc);
        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(err.starts_with("PLAN_MALFORMED:"), "{err}");
        assert!(err.contains("seal"), "{err}");
    }

}
