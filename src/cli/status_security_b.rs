#[allow(unused_imports)]
use super::status_security::*;
#[allow(unused_imports)]
use crate::core::types;
#[allow(unused_imports)]
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn mk(machine: &str, res: Vec<(&str, types::ResourceType)>) -> types::StateLock {
        let mut m = indexmap::IndexMap::new();
        for (id, rt) in res {
            m.insert(
                id.to_string(),
                types::ResourceLock {
                    resource_type: rt,
                    status: types::ResourceStatus::Converged,
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

    fn mk_secrets(machine: &str, n: u64) -> types::StateLock {
        let mut det = HashMap::new();
        det.insert("secret_refs".into(), serde_yaml_ng::Value::Number(n.into()));
        let mut m = indexmap::IndexMap::new();
        m.insert(
            "f".into(),
            types::ResourceLock {
                resource_type: types::ResourceType::File,
                status: types::ResourceStatus::Converged,
                applied_at: Some("2026-01-15T10:00:00Z".into()),
                duration_seconds: Some(0.5),
                hash: "d".into(),
                details: det,
            },
        );
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

    fn wr(dir: &Path, lock: &types::StateLock) {
        let d = dir.join(&lock.machine);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("state.lock.yaml"),
            serde_yaml_ng::to_string(lock).unwrap(),
        )
        .unwrap();
    }

    // ── FJ-1053 ────────────────────────────────────────────────────────────

    #[test]
    fn test_security_posture_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_security_posture_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_security_posture_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "w1",
                vec![
                    ("svc", types::ResourceType::Service),
                    ("tls-c", types::ResourceType::File),
                ],
            ),
        );
        assert!(cmd_status_fleet_security_posture_summary(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_security_posture_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk_secrets("db1", 3));
        assert!(cmd_status_fleet_security_posture_summary(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_security_counts_empty_and_mixed() {
        let empty = mk("e", vec![]);
        assert_eq!(security_counts(&empty), (0, 0, 0));
        let mixed = mk(
            "m",
            vec![
                ("nginx", types::ResourceType::Service),
                ("ssl-c", types::ResourceType::File),
            ],
        );
        let (sr, p, t) = security_counts(&mixed);
        assert_eq!((sr, p, t), (0, 1, 1));
    }

    #[test]
    fn test_classify_posture_variants() {
        assert_eq!(classify_posture(0, 0), "good");
        assert_eq!(classify_posture(2, 1), "moderate");
        assert_eq!(classify_posture(6, 0), "needs-attention");
        assert_eq!(classify_posture(0, 4), "needs-attention");
    }

    // ── FJ-1056 ────────────────────────────────────────────────────────────

    #[test]
    fn test_freshness_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_freshness_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_freshness_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk("s1", vec![("p", types::ResourceType::Package)]),
        );
        assert!(cmd_status_machine_resource_freshness_index(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_freshness_json() {
        let d = tempfile::tempdir().unwrap();
        wr(d.path(), &mk("s2", vec![("f", types::ResourceType::File)]));
        assert!(cmd_status_machine_resource_freshness_index(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_freshness_score_values() {
        let now = 1_700_000_000u64;
        assert!(freshness_score("2023-11-14T22:03:20Z", now) >= 80);
        assert_eq!(freshness_score("2020-01-01T00:00:00Z", 1_900_000_000), 0);
        assert_eq!(freshness_score("", now), 0);
        assert_eq!(freshness_score("garbage", now), 0);
    }

    #[test]
    fn test_freshness_filter() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk("a", vec![("p", types::ResourceType::Package)]),
        );
        wr(
            d.path(),
            &mk("b", vec![("s", types::ResourceType::Service)]),
        );
        assert!(cmd_status_machine_resource_freshness_index(d.path(), Some("a"), false).is_ok());
    }

    // ── FJ-1059 ────────────────────────────────────────────────────────────

    #[test]
    fn test_coverage_empty_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_coverage(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_coverage_with_data() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk(
                "n1",
                vec![
                    ("p", types::ResourceType::Package),
                    ("s", types::ResourceType::Service),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_type_coverage(d.path(), None, false).is_ok());
    }

    #[test]
    fn test_coverage_json() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk("w1", vec![("p", types::ResourceType::Package)]),
        );
        wr(
            d.path(),
            &mk(
                "w2",
                vec![
                    ("p2", types::ResourceType::Package),
                    ("f", types::ResourceType::File),
                ],
            ),
        );
        assert!(cmd_status_fleet_resource_type_coverage(d.path(), None, true).is_ok());
    }

    #[test]
    fn test_collect_coverage_multi() {
        let d = tempfile::tempdir().unwrap();
        wr(
            d.path(),
            &mk("m1", vec![("p", types::ResourceType::Package)]),
        );
        wr(
            d.path(),
            &mk(
                "m2",
                vec![
                    ("p2", types::ResourceType::Package),
                    ("s", types::ResourceType::Service),
                ],
            ),
        );
        let c = collect_type_coverage(d.path(), &["m1".into(), "m2".into()]);
        assert_eq!(c["package"].len(), 2);
        assert_eq!(c["service"].len(), 1);
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_rfc3339() {
        let e = parse_rfc3339_to_epoch("2024-01-01T00:00:00Z");
        assert!(e.is_some());
        assert!(e.unwrap() > 1_700_000_000 && e.unwrap() < 1_800_000_000);
        assert!(parse_rfc3339_to_epoch("").is_none());
        assert!(parse_rfc3339_to_epoch("short").is_none());
    }
}
