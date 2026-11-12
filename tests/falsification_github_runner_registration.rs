//! forjar#1000 (phase 2) falsifiers F1-F5 for `contracts/github-runner-v1.yaml`
//! C1-C5. Real `forjar` binary, local transport, a tempdir, a local fake of the
//! GitHub API and a fake runner tarball. No network, no real App, no real key.

mod github_runner_fake;

use github_runner_fake::{ListedRunner, Rig, INSTALL_TOKEN, REG_TOKEN};

const LABELS: &str = "[self-hosted, linux, x64, gate, fast]";

fn state(rig: &Rig, name: &str) -> String {
    rig.state_dir(name).display().to_string()
}

/// Every non-empty PEM body line plus the whole PEM: the key as it could leak.
fn key_fragments(rig: &Rig) -> Vec<String> {
    let mut v: Vec<String> = rig
        .key_pem
        .lines()
        .filter(|l| !l.starts_with("-----") && l.len() > 20)
        .map(str::to_string)
        .collect();
    v.push(rig.key_pem.trim().to_string());
    v
}

fn assert_absent(what: &str, haystack: &str, needles: &[String]) {
    for n in needles {
        assert!(
            !haystack.contains(n.as_str()),
            "{what} contains a secret: {}...",
            &n[..8.min(n.len())]
        );
    }
}

// F1 (C1): the key, the JWT and the tokens never leave the control host.
#[test]
fn f1_key_jwt_and_tokens_stay_off_every_surface() {
    let rig = Rig::new(LABELS, true);
    rig.fake
        .appear_on_token(ListedRunner::new(&["fast", "gate"], "Default"));
    let st = state(&rig, "st1");
    let applied = rig.apply(&st);
    assert!(applied.ok, "apply must succeed: {}", applied.text);
    let checked = rig.check(&st);
    let jwt = rig.fake.jwt_seen();
    assert_eq!(jwt.matches('.').count(), 2, "the fake must have seen a JWT");
    // Positive controls: the registration token really reached config.sh, and
    // the state dir really has content, so the absences below mean something.
    assert_eq!(
        rig.token_seen_by_config(),
        REG_TOKEN,
        "config.sh must receive the token"
    );
    let state_text = github_runner_fake::tree_text(&rig.state_dir("st1"));
    assert!(!state_text.is_empty(), "state dir must not be empty");

    let mut secrets = key_fragments(&rig);
    secrets.extend([jwt, INSTALL_TOKEN.to_string()]);
    let mut with_reg = secrets.clone();
    with_reg.push(REG_TOKEN.to_string());
    for (what, text) in [
        ("apply output", &applied.text),
        ("check output", &checked.text),
        ("state dir", &state_text),
    ] {
        assert_absent(what, text, &with_reg);
    }
    let scripts = rig.seat_scripts();
    assert!(!scripts.is_empty(), "the seat must have received a script");
    for s in &scripts {
        assert_absent("seat script", s, &secrets);
    }
    let carrying: Vec<&String> = scripts.iter().filter(|s| s.contains(REG_TOKEN)).collect();
    assert_eq!(
        carrying.len(),
        1,
        "the registration token goes in exactly one seat script"
    );
    assert_eq!(
        carrying[0].matches(REG_TOKEN).count(),
        1,
        "and exactly once in it"
    );
    assert!(
        carrying[0].lines().any(|l| l.trim() == REG_TOKEN),
        "the token travels on its own stdin line, never in a command line"
    );
    for call in rig.config_calls() {
        assert!(
            !call.contains(REG_TOKEN),
            "token must not be in config.sh argv: {call}"
        );
    }
}

// F2 (C2): same labels in another order, same group: nothing runs on the seat.
#[test]
fn f2_equal_labels_in_another_order_adopt_with_zero_seat_commands() {
    let rig = Rig::new(LABELS, true);
    rig.fake
        .list(ListedRunner::new(&["fast", "gate"], "Default"));
    let st = state(&rig, "st2");
    let applied = rig.apply(&st);
    assert!(applied.ok, "adopt must succeed: {}", applied.text);
    assert_eq!(
        rig.seat_commands(),
        0,
        "adopt runs 0 commands on the seat; saw {:?}",
        rig.seat_scripts()
    );
    assert_eq!(
        rig.fake.token_requests(),
        0,
        "adopt asks for no registration token"
    );
    let checked = rig.check(&st);
    assert!(
        checked.ok,
        "check passes for an adopted seat: {}",
        checked.text
    );
}

// F3 (C3): one label fewer is refused, both sets named, nothing asked or run.
#[test]
fn f3_label_drift_is_refused_naming_the_difference() {
    let rig = Rig::new(LABELS, true);
    rig.fake.list(ListedRunner::new(&["fast"], "Default"));
    let st = state(&rig, "st3");
    let applied = rig.apply(&st);
    assert!(!applied.ok, "drift must fail apply: {}", applied.text);
    assert!(
        applied.text.contains("gate"),
        "must name the missing label: {}",
        applied.text
    );
    assert!(
        applied.text.contains("fast"),
        "must name the listed set: {}",
        applied.text
    );
    assert_eq!(
        rig.fake.token_requests(),
        0,
        "refusal asks for no registration token"
    );
    assert_eq!(
        rig.seat_commands(),
        0,
        "refusal runs 0 commands on the seat"
    );
    assert!(rig.config_calls().is_empty(), "config.sh never runs");

    let checked = rig.check(&st);
    assert!(!checked.ok, "check must report the drift: {}", checked.text);
    assert!(
        checked.text.contains("gate"),
        "check names the label: {}",
        checked.text
    );
}

#[test]
fn f3_group_drift_is_refused_naming_both_groups() {
    let rig = Rig::new(LABELS, true);
    rig.fake.list(ListedRunner::new(&["fast", "gate"], "Other"));
    let applied = rig.apply(&state(&rig, "st3g"));
    assert!(!applied.ok, "group drift must fail apply: {}", applied.text);
    assert!(
        applied.text.contains("Other"),
        "names the listed group: {}",
        applied.text
    );
    assert!(
        applied.text.to_lowercase().contains("default"),
        "names the declared group: {}",
        applied.text
    );
    assert_eq!(rig.fake.token_requests(), 0);
    assert_eq!(rig.seat_commands(), 0, "seat saw: {:?}", rig.seat_scripts());
}

// F4 (C4): an absent seat is registered with one token request; then it is adopted.
#[test]
fn f4_absent_registers_once_then_stays_unchanged() {
    let rig = Rig::new(LABELS, true);
    rig.fake
        .appear_on_token(ListedRunner::new(&["fast", "gate"], "Default"));
    let st = state(&rig, "st4");
    let first = rig.apply(&st);
    assert!(first.ok, "first apply must succeed: {}", first.text);
    assert_eq!(
        rig.fake.token_requests(),
        1,
        "exactly one registration-token request"
    );
    let calls = rig.config_calls();
    assert_eq!(calls.len(), 1, "config.sh runs once: {calls:?}");
    assert!(
        calls[0].contains("--unattended"),
        "unattended: {}",
        calls[0]
    );
    assert!(
        !calls[0].contains("--replace"),
        "never --replace: {}",
        calls[0]
    );
    assert!(calls[0].contains("gate"), "labels passed: {}", calls[0]);
    assert!(
        std::fs::read_to_string(rig.dir.path().join("seatlog/svc-calls"))
            .unwrap_or_default()
            .contains("start"),
        "the service is started"
    );

    let second = rig.apply(&st);
    assert!(second.ok, "second apply must succeed: {}", second.text);
    assert!(
        second.text.contains("unchanged"),
        "second apply is unchanged: {}",
        second.text
    );
    assert_eq!(rig.fake.token_requests(), 1, "still one token request");

    // A fresh state dir sees the seat listed: adopt, no new token, no new script.
    let third = rig.apply(&state(&rig, "st4b"));
    assert!(
        third.ok,
        "adopt on a fresh state must succeed: {}",
        third.text
    );
    assert_eq!(rig.fake.token_requests(), 1, "adopt asks for no token");
    assert_eq!(rig.seat_commands(), 1, "adopt adds no seat command");
}

// F5 (C5): a tarball with one byte changed is refused before config.sh.
#[test]
fn f5_tampered_tarball_is_refused_before_config_sh_runs() {
    let rig = Rig::new(LABELS, true);
    rig.fake
        .appear_on_token(ListedRunner::new(&["fast", "gate"], "Default"));
    rig.corrupt_tarball();
    let applied = rig.apply(&state(&rig, "st5"));
    assert!(
        !applied.ok,
        "tampered tarball must fail apply: {}",
        applied.text
    );
    assert!(
        applied.text.to_lowercase().contains("sha256"),
        "names the digest check: {}",
        applied.text
    );
    assert!(rig.config_calls().is_empty(), "config.sh must never run");
    assert!(
        !rig.runner_dir().join("runner/config.sh").exists(),
        "nothing was extracted"
    );
}

// Extra: the installation is looked up when the manifest omits installation_id,
// and the JWT is a valid RS256 signature over header.payload.
#[test]
fn installation_is_looked_up_and_jwt_verifies() {
    let rig = Rig::new(LABELS, false);
    rig.fake
        .list(ListedRunner::new(&["fast", "gate"], "Default"));
    let applied = rig.apply(&state(&rig, "st6"));
    assert!(applied.ok, "apply must succeed: {}", applied.text);
    assert_eq!(rig.fake.count("GET", "/orgs/example-org/installation"), 1);

    let jwt = rig.fake.jwt_seen();
    let parts: Vec<&str> = jwt.split('.').collect();
    assert_eq!(parts.len(), 3);
    let root = rig.dir.path();
    let pubkey = std::process::Command::new("openssl")
        .args(["rsa", "-pubout", "-in"])
        .arg(root.join("app-key.pem"))
        .output()
        .unwrap();
    assert!(pubkey.status.success());
    std::fs::write(root.join("pub.pem"), pubkey.stdout).unwrap();
    std::fs::write(
        root.join("signed.txt"),
        format!("{}.{}", parts[0], parts[1]),
    )
    .unwrap();
    let sig = base64_url_decode(parts[2]);
    std::fs::write(root.join("sig.bin"), sig).unwrap();
    let verify = std::process::Command::new("openssl")
        .args(["dgst", "-sha256", "-verify"])
        .arg(root.join("pub.pem"))
        .arg("-signature")
        .arg(root.join("sig.bin"))
        .arg(root.join("signed.txt"))
        .output()
        .unwrap();
    assert!(
        verify.status.success(),
        "JWT signature must verify as RS256"
    );
    let header = String::from_utf8(base64_url_decode(parts[0])).unwrap();
    assert!(header.contains("RS256"), "header: {header}");
    let payload = String::from_utf8(base64_url_decode(parts[1])).unwrap();
    assert!(payload.contains("123456"), "iss is the app id: {payload}");
}

fn base64_url_decode(s: &str) -> Vec<u8> {
    let table = |c: u8| -> u32 {
        match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'-' => 62,
            _ => 63,
        }
    };
    let (mut out, mut acc, mut bits) = (Vec::new(), 0u32, 0u32);
    for c in s.bytes() {
        acc = (acc << 6) | table(c);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xff) as u8);
        }
    }
    out
}
