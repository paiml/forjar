#[allow(unused_imports)]
use super::status_maturity::*;
#[allow(unused_imports)]
use crate::core::types;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mk(
        machine: &str,
        ts: &str,
        res: Vec<(&str, types::ResourceType, types::ResourceStatus)>,
    ) -> types::StateLock {
        let mut m = indexmap::IndexMap::new();
        for (id, rt, st) in res {
            m.insert(
                id.to_string(),
                types::ResourceLock {
                    resource_type: rt,
                    status: st,
                    applied_at: Some(ts.into()),
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
            generated_at: ts.into(),
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

    // -- FJ-1099: Drift Pattern Analysis ---------------------------------------

    #[test]
    fn test_drift_pattern_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_pattern_none() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
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
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_pattern_sporadic() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
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
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_pattern_chronic() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "cfg",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_pattern_cascading() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg",
                        types::ResourceType::Package,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "svc",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_drift_pattern_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "n1",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "p",
                        types::ResourceType::Package,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "s",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "f",
                        types::ResourceType::File,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_drift_pattern_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![(
                    "pkg",
                    types::ResourceType::Package,
                    types::ResourceStatus::Drifted,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![(
                    "svc",
                    types::ResourceType::Service,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        assert!(
            cmd_status_fleet_resource_drift_pattern_analysis(d.path(), Some("web"), false).is_ok()
        );
    }

    // -- Helper unit tests -----------------------------------------------------

    #[test]
    fn test_classify_resources_all_statuses() {
        let lock = mk(
            "m",
            "2026-01-15T10:00:00Z",
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
        let lock = mk("m", "2026-01-15T10:00:00Z", vec![]);
        assert_eq!(classify_resources(&lock), (0, 0, 0, 0));
    }

    #[test]
    fn test_distinct_resource_types() {
        let lock = mk(
            "m",
            "2026-01-15T10:00:00Z",
            vec![
                (
                    "a",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                ),
                (
                    "b",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                ),
                (
                    "c",
                    types::ResourceType::Service,
                    types::ResourceStatus::Converged,
                ),
            ],
        );
        assert_eq!(distinct_resource_types(&lock), 2);
    }

    #[test]
    fn test_distinct_resource_types_empty() {
        let lock = mk("m", "2026-01-15T10:00:00Z", vec![]);
        assert_eq!(distinct_resource_types(&lock), 0);
    }

    #[test]
    fn test_classify_drift_pattern_none() {
        assert_eq!(classify_drift_pattern(0, 5), "none");
    }

    #[test]
    fn test_classify_drift_pattern_sporadic() {
        assert_eq!(classify_drift_pattern(1, 5), "sporadic");
    }

    #[test]
    fn test_classify_drift_pattern_chronic() {
        assert_eq!(classify_drift_pattern(3, 5), "chronic");
    }

    #[test]
    fn test_classify_drift_pattern_cascading() {
        assert_eq!(classify_drift_pattern(5, 5), "cascading");
    }
}
