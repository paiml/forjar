//! Tests for the bake register tags and old-AMI cleanup policy (#1310, slice 3).
//! Pure functions only: no AWS call is made here.

use super::*;

fn digest() -> String {
    format!("sha256:{}", "b".repeat(64))
}

fn bake(id: &str, created: u64) -> BakedImage {
    BakedImage {
        ami_id: id.into(),
        created,
        kind: Some(KIND_VALUE.into()),
        fleet_digest: Some(digest()),
    }
}

#[test]
fn register_tags_carry_digest_and_kind() {
    let tags = register_tags(&digest()).unwrap();
    assert_eq!(
        tags,
        vec![(DIGEST_TAG, digest()), (KIND_TAG, KIND_VALUE.to_string())]
    );
}

#[test]
fn register_refuses_a_digest_that_is_not_sha256_hex() {
    // Guard: an image is never registered without a recognisable fleet digest.
    assert!(matches!(
        register_tags("sha256:short"),
        Err(BakePlanError::BadDigest(_))
    ));
    assert!(matches!(
        register_tags(&format!("sha256:{}", "B".repeat(64))),
        Err(BakePlanError::BadDigest(_))
    ));
    assert!(matches!(
        register_tags(&"b".repeat(64)),
        Err(BakePlanError::BadDigest(_))
    ));
}

#[test]
fn cleanup_keeps_the_newest_keep_last_and_deregisters_the_rest() {
    let imgs = vec![
        bake("ami-old", 100),
        bake("ami-new", 300),
        bake("ami-mid", 200),
    ];
    // keep 2 newest (ami-new, ami-mid); victim is the oldest only.
    assert_eq!(
        cleanup_victims(&imgs, 2).unwrap(),
        vec!["ami-old".to_string()]
    );
}

#[test]
fn cleanup_orders_by_created_not_by_id() {
    // Guard: the sort is by creation time. An id-ordered cleanup would keep "ami-a"
    // (older) and deregister the newest image.
    let imgs = vec![bake("ami-a", 100), bake("ami-z", 900)];
    assert_eq!(
        cleanup_victims(&imgs, 1).unwrap(),
        vec!["ami-a".to_string()]
    );
}

#[test]
fn cleanup_never_removes_the_newest_bake() {
    let imgs = vec![bake("ami-1", 10), bake("ami-2", 20), bake("ami-3", 30)];
    for keep in 1..=3u32 {
        let victims = cleanup_victims(&imgs, keep).unwrap();
        assert!(
            !victims.contains(&"ami-3".to_string()),
            "keep={keep} removed newest"
        );
    }
}

#[test]
fn cleanup_ignores_untagged_and_non_bake_images() {
    // Guard: a hand-made AMI, or one with no kind tag, is never a candidate even when old.
    let mut hand = bake("ami-hand", 1);
    hand.kind = None;
    hand.fleet_digest = None;
    let mut other = bake("ami-other", 2);
    other.kind = Some("something_else".into());
    let imgs = vec![hand, other, bake("ami-new", 300)];
    assert!(cleanup_victims(&imgs, 1).unwrap().is_empty());
}

#[test]
fn cleanup_ignores_a_bake_without_a_valid_fleet_digest() {
    // Guard: a bake kind tag with no valid digest is not a fleet bake and is kept.
    let mut bad = bake("ami-bad", 1);
    bad.fleet_digest = Some("not-a-digest".into());
    let imgs = vec![bad, bake("ami-new", 300)];
    assert!(cleanup_victims(&imgs, 1).unwrap().is_empty());
}

#[test]
fn cleanup_refuses_keep_last_zero() {
    // Guard: keep_last 0 would remove the only image.
    assert_eq!(
        cleanup_victims(&[bake("ami-1", 1)], 0),
        Err(BakePlanError::KeepLastZero)
    );
}

#[test]
fn cleanup_with_fewer_bakes_than_keep_last_removes_nothing() {
    let imgs = vec![bake("ami-1", 1)];
    assert!(cleanup_victims(&imgs, 5).unwrap().is_empty());
}

#[test]
fn cleanup_ties_break_by_ami_id_so_the_result_is_stable() {
    let imgs = vec![bake("ami-b", 50), bake("ami-a", 50), bake("ami-c", 50)];
    // Same created time: order is ami-a, ami-b, ami-c. Keep 1 (ami-a), remove the rest.
    assert_eq!(
        cleanup_victims(&imgs, 1).unwrap(),
        vec!["ami-b".to_string(), "ami-c".to_string()]
    );
}
