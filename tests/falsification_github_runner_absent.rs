//! forjar#1000 (phase 3) falsifiers F7 and F8 for `contracts/github-runner-v1.yaml`
//! C7 (`absent_deregisters`) and C8 (`repo_scope_end_to_end`). Real `forjar`
//! binary, local transport, a local fake of the GitHub API standing in for
//! `gh`/curl. No network, no real App, no real key, no real token.

mod github_runner_fake;

use github_runner_fake::{ListedRunner, Rig, INSTALL_TOKEN, REG_TOKEN, REMOVE_TOKEN};

const LABELS: &str = "[self-hosted, linux, x64, gate, fast]";
const SEAT: &str = "/actions/runners/99";

fn state(rig: &Rig, name: &str) -> String {
    rig.state_dir(name).display().to_string()
}

fn registered(rig: &Rig, st: &str) {
    rig.fake
        .appear_on_token(ListedRunner::new(&["fast", "gate"], "Default"));
    let first = rig.apply(st);
    assert!(first.ok, "setup registration must succeed: {}", first.text);
    assert!(
        rig.runner_dir().join("runner/config.sh").exists(),
        "setup must leave an install to remove"
    );
}

// F7 (C7): a listed seat is deregistered with one remove-token request, the
// token travelling only on stdin; check follows.
#[test]
fn f7_absent_deregisters_a_listed_seat_with_one_remove_token() {
    let rig = Rig::new(LABELS, true);
    registered(&rig, &state(&rig, "st7a"));
    rig.set_absent();
    let before = rig.check(&state(&rig, "st7a-check"));
    assert!(
        !before.ok && before.text.contains("still registered"),
        "check must say the seat is still registered: {}",
        before.text
    );
    rig.fake.vanish_on_remove_token();
    let scripts_before = rig.seat_commands();
    let out = rig.apply(&state(&rig, "st7b"));
    assert!(out.ok, "absent apply must succeed: {}", out.text);
    assert_eq!(
        rig.fake.remove_token_requests(),
        1,
        "one remove-token request"
    );
    assert_eq!(rig.fake.token_requests(), 1, "no new registration token");
    assert_eq!(rig.seat_commands(), scripts_before + 1, "one seat script");
    let calls = rig.config_calls();
    assert!(
        calls.last().is_some_and(|c| c.starts_with("remove")),
        "config.sh remove ran last: {calls:?}"
    );
    assert!(
        calls.iter().all(|c| !c.contains(REMOVE_TOKEN)),
        "token never in config.sh argv: {calls:?}"
    );
    assert_eq!(
        rig.token_seen_by_config(),
        REMOVE_TOKEN,
        "token reached stdin"
    );
    let scripts = rig.seat_scripts();
    let carrying: Vec<&String> = scripts
        .iter()
        .filter(|s| s.contains(REMOVE_TOKEN))
        .collect();
    assert_eq!(carrying.len(), 1, "remove token in exactly one script");
    assert_eq!(carrying[0].matches(REMOVE_TOKEN).count(), 1);
    let svc = std::fs::read_to_string(rig.dir.path().join("seatlog/svc-calls")).unwrap();
    assert!(
        svc.contains("stop") && svc.contains("uninstall"),
        "service stopped and uninstalled: {svc}"
    );
    let st_text = github_runner_fake::tree_text(&rig.state_dir("st7b"));
    for secret in [REMOVE_TOKEN, REG_TOKEN, INSTALL_TOKEN] {
        assert!(!out.text.contains(secret), "output leaks a token");
        assert!(!st_text.contains(secret), "state dir leaks a token");
    }
    assert_eq!(
        rig.fake.count("DELETE", SEAT),
        0,
        "listing went with the seat"
    );
    let after = rig.check(&state(&rig, "st7b"));
    assert!(
        after.ok,
        "check passes once the seat is gone: {}",
        after.text
    );
}

// F7: a seat that GitHub does not list costs nothing: idempotent, no commands.
#[test]
fn f7_absent_when_not_listed_is_a_noop() {
    let rig = Rig::new(LABELS, true);
    rig.set_absent();
    let out = rig.apply(&state(&rig, "st7c"));
    assert!(out.ok, "absent on an unlisted seat succeeds: {}", out.text);
    assert_eq!(rig.seat_commands(), 0, "0 commands on the seat");
    assert_eq!(rig.fake.remove_token_requests(), 0, "no remove token asked");
    assert_eq!(rig.fake.token_requests(), 0);
}

// F7: the seat was wiped by hand (no install) but GitHub still lists it: the
// listing is deleted by id, and config.sh never runs.
#[test]
fn f7_absent_deletes_a_listing_that_outlives_the_seat() {
    let rig = Rig::new(LABELS, true);
    rig.fake
        .list(ListedRunner::new(&["fast", "gate"], "Default"));
    rig.set_absent();
    let out = rig.apply(&state(&rig, "st7d"));
    assert!(out.ok, "stale listing is removed: {}", out.text);
    assert_eq!(rig.fake.count("DELETE", SEAT), 1, "listing deleted by id");
    assert!(rig.config_calls().is_empty(), "no install, so no config.sh");
    assert!(rig.check(&state(&rig, "st7d2")).ok);
}

// F7: a failing remove on the seat is a failure and leaves the listing alone.
#[test]
fn f7_absent_reports_a_failed_remove_and_keeps_the_listing() {
    let rig = Rig::new(LABELS, true);
    registered(&rig, &state(&rig, "st7e"));
    rig.set_absent();
    let cfg = rig.runner_dir().join("runner/config.sh");
    std::fs::write(&cfg, "#!/bin/sh\nexit 3\n").unwrap();
    let out = rig.apply(&state(&rig, "st7e2"));
    assert!(
        !out.ok,
        "a failed config.sh remove must fail apply: {}",
        out.text
    );
    assert_eq!(rig.fake.count("DELETE", SEAT), 0, "listing untouched");
}

// F8 (C8): a repo-scoped seat registers, is adopted, refuses drift and
// deregisters through /repos/<owner>/<repo>, never /orgs.
#[test]
fn f8_repo_scope_registers_adopts_and_deregisters_end_to_end() {
    let rig = Rig::new_repo(LABELS, "example-repo");
    registered(&rig, &state(&rig, "st8a"));
    let calls = rig.config_calls();
    assert_eq!(calls.len(), 1, "config.sh once: {calls:?}");
    assert!(
        calls[0].contains("--url https://github.com/example-org/example-repo"),
        "repo registration url: {}",
        calls[0]
    );
    assert!(
        !calls[0].contains("--runnergroup"),
        "no group at repo scope"
    );
    let base = "/repos/example-org/example-repo";
    assert_eq!(
        rig.fake.count(
            "POST",
            &format!("{base}/actions/runners/registration-token")
        ),
        1
    );
    let again = rig.apply(&state(&rig, "st8a"));
    assert!(
        again.ok && again.text.contains("unchanged"),
        "{}",
        again.text
    );
    let adopted = rig.apply(&state(&rig, "st8b"));
    assert!(adopted.ok, "adopt on fresh state: {}", adopted.text);
    assert_eq!(rig.fake.token_requests(), 1, "adopt asks for no token");

    rig.set_absent();
    rig.fake.vanish_on_remove_token();
    let gone = rig.apply(&state(&rig, "st8c"));
    assert!(gone.ok, "repo-scope deregistration: {}", gone.text);
    assert_eq!(
        rig.fake
            .count("POST", &format!("{base}/actions/runners/remove-token")),
        1
    );
    let bad: Vec<_> = rig
        .fake
        .requests()
        .into_iter()
        .filter(|r| r.path.starts_with("/orgs/"))
        .collect();
    assert!(bad.is_empty(), "no request may use the org scope: {bad:?}");
}

#[test]
fn f8_repo_scope_label_drift_is_refused() {
    let rig = Rig::new_repo(LABELS, "example-repo");
    rig.fake.list(ListedRunner::new(&["fast"], "Default"));
    let out = rig.apply(&state(&rig, "st8d"));
    assert!(!out.ok && out.text.contains("gate"), "{}", out.text);
    assert_eq!(rig.fake.token_requests(), 0);
    assert_eq!(rig.seat_commands(), 0);
}
