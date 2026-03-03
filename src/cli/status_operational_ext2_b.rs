#[allow(unused_imports)]
use super::status_operational_ext2::*;
#[allow(unused_imports)]
use crate::core::types;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mk(
        machine: &str,
        res: Vec<(&str, types::ResourceType, types::ResourceStatus)>,
    ) -> types::StateLock {
        let mut m = indexmap::IndexMap::new();
        for (id, rt, st) in res {
            m.insert(
                id.to_string(),
                types::ResourceLock {
                    resource_type: rt,
                    status: st,
                    applied_at: Some("2026-01-15T10:00:00Z".into()),
                    duration_seconds: Some(1.0),
                    hash: "abc".into(),
                    details: HashMap::new(),
                },
            );
        }
        types::StateLock {
            schema: "1".into(),
            machine: machine.into(),
            hostname: machine.into(),
            generated_at: "2026-01-15T10:00:00Z".into(),
            generator: "test".into(),
            blake3_version: "1.0".into(),
            resources: m,
        }
    }

    fn wr(dir: &std::path::Path, lock: &types::StateLock) {
        let d = dir.join(&lock.machine);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("state.lock.yaml"),
            serde_yaml_ng::to_string(lock).unwrap(),
        )
        .unwrap();
    }

    // ── FJ-1061: Fleet Apply Cadence ────────────────────────────────────────

    #[test]
    fn test_apply_cadence_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_apply_cadence(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_cadence_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_apply_cadence(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_apply_cadence_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_apply_cadence(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_apply_cadence_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_apply_cadence(d.path(), Some("web"), false).is_ok());
    }

    // ── FJ-1064: Error Classification ───────────────────────────────────────

    #[test]
    fn test_error_classification_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_classification(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_error_classification_mixed_status() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Failed,
                    ),
                ],
            ),
        );
        assert!(cmd_status_machine_resource_error_classification(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_error_classification_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "db",
                vec![
                    (
                        "f1",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "f2",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_machine_resource_error_classification(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_error_classification_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Drifted,
                )],
            ),
        );
        assert!(
            cmd_status_machine_resource_error_classification(d.path(), Some("db"), false).is_ok()
        );
    }

    // ── FJ-1067: Convergence Summary ────────────────────────────────────────

    #[test]
    fn test_convergence_summary_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_summary_all_converged() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_summary_partial() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Failed,
                    ),
                ],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                vec![(
                    "f1",
                    types::ResourceType::File,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_convergence_summary_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "n1",
                vec![
                    (
                        "p",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "s",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, true).is_ok());
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    #[test]
    fn test_classify_resources_all_statuses() {
        let lock = mk(
            "m",
            vec![
                (
                    "a",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                ),
                (
                    "b",
                    types::ResourceType::Service,
                    types::ResourceStatus::Drifted,
                ),
                (
                    "c",
                    types::ResourceType::File,
                    types::ResourceStatus::Failed,
                ),
                (
                    "d",
                    types::ResourceType::File,
                    types::ResourceStatus::Unknown,
                ),
            ],
        );
        assert_eq!(classify_resources(&lock), (1, 1, 1, 1));
    }

    #[test]
    fn test_classify_resources_empty() {
        let lock = mk("m", vec![]);
        assert_eq!(classify_resources(&lock), (0, 0, 0, 0));
    }

    #[test]
    fn test_parse_rfc3339_valid() {
        let e = parse_rfc3339_to_epoch("2024-01-01T00:00:00Z");
        assert!(e.is_some());
        assert!(e.unwrap() > 1_700_000_000 && e.unwrap() < 1_800_000_000);
    }

    #[test]
    fn test_parse_rfc3339_invalid() {
        assert!(parse_rfc3339_to_epoch("").is_none());
        assert!(parse_rfc3339_to_epoch("short").is_none());
        assert!(parse_rfc3339_to_epoch("not-a-timestamp!!").is_none());
    }
}
