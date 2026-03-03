#[allow(unused_imports)]
use super::status_quality::*;
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

    // -- FJ-1117: Fleet Resource Quality Score -----------------------------------

    #[test]
    fn test_quality_score_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_quality_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_quality_score_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg1",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "pkg2",
                        types::ResourceType::Package,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "svc1",
                        types::ResourceType::Service,
                        types::ResourceStatus::Failed,
                    ),
                    (
                        "cfg1",
                        types::ResourceType::File,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg1",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc1",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_quality_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_quality_score_json() {
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
        assert!(cmd_status_fleet_resource_quality_score(d.path(), None, true).is_ok());
    }
    #[test]
    fn test_quality_score_filtered() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![(
                    "p",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![(
                    "p",
                    types::ResourceType::Package,
                    types::ResourceStatus::Failed,
                )],
            ),
        );
        assert!(cmd_status_fleet_resource_quality_score(d.path(), Some("web"), false).is_ok());
    }

    // -- FJ-1120: Machine Resource Drift Pattern Classification ------------------

    #[test]
    fn test_drift_pattern_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(
            cmd_status_machine_resource_drift_pattern_classification(d.path(), None, false).is_ok()
        );
    }
    #[test]
    fn test_drift_pattern_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg1",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "pkg2",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc1",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg1",
                        types::ResourceType::Package,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "svc1",
                        types::ResourceType::Service,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "cfg1",
                        types::ResourceType::File,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        assert!(
            cmd_status_machine_resource_drift_pattern_classification(d.path(), None, false).is_ok()
        );
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
                ],
            ),
        );
        assert!(
            cmd_status_machine_resource_drift_pattern_classification(d.path(), None, true).is_ok()
        );
    }
    #[test]
    fn test_drift_pattern_chronic() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "sick",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "p1",
                        types::ResourceType::Package,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "p2",
                        types::ResourceType::Package,
                        types::ResourceStatus::Drifted,
                    ),
                    (
                        "p3",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                ],
            ),
        );
        assert!(
            cmd_status_machine_resource_drift_pattern_classification(d.path(), None, false).is_ok()
        );
    }

    // -- FJ-1123: Fleet Resource Convergence Window Analysis ---------------------

    #[test]
    fn test_convergence_window_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(
            cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, false).is_ok()
        );
    }
    #[test]
    fn test_convergence_window_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "web",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg1",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc1",
                        types::ResourceType::Service,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "cfg1",
                        types::ResourceType::File,
                        types::ResourceStatus::Drifted,
                    ),
                ],
            ),
        );
        wr(
            d.path(),
            &mk(
                "db",
                "2026-01-15T10:00:00Z",
                vec![
                    (
                        "pkg1",
                        types::ResourceType::Package,
                        types::ResourceStatus::Converged,
                    ),
                    (
                        "svc1",
                        types::ResourceType::Service,
                        types::ResourceStatus::Failed,
                    ),
                ],
            ),
        );
        assert!(
            cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, false).is_ok()
        );
    }
    #[test]
    fn test_convergence_window_json() {
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
        assert!(
            cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, true).is_ok()
        );
    }
    #[test]
    fn test_convergence_window_fleet_average() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "a",
                "2026-01-15T10:00:00Z",
                vec![(
                    "p",
                    types::ResourceType::Package,
                    types::ResourceStatus::Converged,
                )],
            ),
        );
        wr(
            d.path(),
            &mk(
                "b",
                "2026-01-15T10:00:00Z",
                vec![(
                    "p",
                    types::ResourceType::Package,
                    types::ResourceStatus::Drifted,
                )],
            ),
        );
        assert!(
            cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, false).is_ok()
        );
    }
}
