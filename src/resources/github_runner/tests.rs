use super::*;
use crate::core::types::{ResourceType, RunnerApp};

fn resource(scope: &str, repo: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::GithubRunner,
        name: Some("host-gate-1".into()),
        owner: Some("example-org".into()),
        repo: repo.map(str::to_string),
        version: Some("2.330.0".into()),
        runner: crate::core::types::RunnerSpec {
            scope: Some(scope.into()),
            labels: vec!["gate".into()],
            sha256: Some("ab".repeat(32)),
            user: Some("runner".into()),
            app: Some(RunnerApp {
                app_id: Some(1),
                installation_id: None,
                private_key: Some("PEMBODY".into()),
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn org_and_repo_scope_use_their_own_api_paths() {
    let org = Seat::from_resource(&resource("org", None)).unwrap();
    assert_eq!(org.base().unwrap(), "/orgs/example-org");
    assert_eq!(org.registration_url(), "https://github.com/example-org");
    let repo = Seat::from_resource(&resource("repo", Some("example-repo"))).unwrap();
    assert_eq!(repo.base().unwrap(), "/repos/example-org/example-repo");
    assert_eq!(
        repo.registration_url(),
        "https://github.com/example-org/example-repo"
    );
}

#[test]
fn an_unresolved_key_is_refused_without_printing_it() {
    let mut r = resource("org", None);
    r.runner.app.as_mut().unwrap().private_key = Some("{{secrets.runner-app-pem}}".into());
    let err = Seat::from_resource(&r).err().unwrap();
    assert!(err.contains("secrets provider"), "{err}");
}

#[test]
fn a_missing_user_is_named() {
    let mut r = resource("org", None);
    r.runner.user = None;
    assert!(Seat::from_resource(&r).err().unwrap().contains("`user`"));
}

#[test]
fn the_plan_passes_only_custom_labels_and_defaults_the_work_dir() {
    let mut r = resource("org", None);
    r.runner.labels = vec!["self-hosted".into(), "gate".into()];
    let plan = Seat::from_resource(&r).unwrap().plan();
    assert_eq!(plan.labels, "gate");
    assert_eq!(plan.install_dir, "/opt/github-runner/host-gate-1/runner");
}

#[test]
fn redaction_removes_every_occurrence() {
    assert_eq!(redact("a TOK b TOK", "TOK"), "a <redacted> b <redacted>");
}

// The post-resolution pass refuses what validate refuses: an org seat with a
// `repo` is never quietly widened to the whole org.
#[test]
fn an_org_seat_carrying_a_repo_is_refused_not_dropped() {
    let err = Seat::from_resource(&resource("org", Some("forjar")))
        .err()
        .expect("an org seat with a repo must be refused");
    assert!(err.contains("`repo` must not be set"), "{err}");
    // Control: an org seat with a blank repo is still an org seat.
    assert!(Seat::from_resource(&resource("org", Some("  "))).is_ok());
}

#[test]
fn a_repo_seat_with_a_blank_repo_is_refused() {
    for repo in [None, Some(""), Some("   ")] {
        let err = Seat::from_resource(&resource("repo", repo)).err().unwrap();
        assert!(err.contains("`repo` is required"), "{repo:?}: {err}");
    }
}

#[test]
fn an_unknown_state_is_refused_and_known_ones_pass() {
    for bad in ["removed", "stopped", "Absent"] {
        let mut r = resource("org", None);
        r.state = Some(bad.into());
        let err = Seat::from_resource(&r).err().expect(bad);
        assert!(err.contains("`state`") && err.contains(bad), "{err}");
    }
    for good in [None, Some("present"), Some("absent")] {
        let mut r = resource("org", None);
        r.state = good.map(str::to_string);
        assert!(Seat::from_resource(&r).is_ok(), "{good:?}");
    }
}
