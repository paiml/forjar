//! Generated-script tests: the box phases run for real under `sh` against a stateful
//! fake `docker`/`aws`/`zstd`; the controller phases run against a fake `aws`/`gh`.

use super::tests::{res, ID};
use super::*;
use std::os::unix::fs::PermissionsExt;

const DOCKER: &str = r#"#!/bin/sh
d="$FAKE_DIR"; echo "docker $*" >> "$d/log"
case "$1 $2" in
  "image inspect") [ -e "$d/loaded" ]; exit $?;;
  "image ls") echo "sha256:deadbeef"; exit 0;;
esac
case "$1" in
  load) cat >/dev/null; [ -e "$d/loadfail" ] && { echo "load boom" >&2; exit 1; }
        [ -e "$d/quirk" ] || touch "$d/loaded"; echo "Loaded image ID: x"; exit 0;;
  rm) rm -f "$d/run-$3"; exit 0;;
  logs) echo "runner log line"; exit 0;;
  inspect) n=""; for a in "$@"; do n="$a"; done
        [ -e "$d/run-$n" ] && [ ! -e "$d/never" ] && { echo true; exit 0; }; exit 1;;
  run) [ -e "$d/runfail" ] && { echo "docker: Error response from daemon" >&2; exit 125; }
       n=""; p=""; for a in "$@"; do [ "$p" = --name ] && n="$a"; p="$a"; done
       touch "$d/run-$n"; echo cid; exit 0;;
esac
exit 64
"#;

const AWS: &str = r#"#!/bin/sh
d="$FAKE_DIR"; echo "aws $*" >> "$d/log"
[ "$1 $2" = "s3 cp" ] && { [ -e "$d/fetchfail" ] && exit 1; printf 'tarcontent\n' > "$4"; exit 0; }
case "$1 $2" in
  "ec2 describe-instances") case "$*" in *client-token*) echo None;; *Price*) cat "$d/prices" 2>/dev/null;; *) cat "$d/live" 2>/dev/null;; esac; exit 0;;
  "ec2 run-instances") [ -e "$d/nocap" ] && { echo "An error occurred (InsufficientInstanceCapacity)" >&2; exit 254; }; echo i-launched; exit 0;;
esac
exit 0
"#;

pub(super) struct Box_ {
    dir: tempfile::TempDir,
}

fn exe(p: &std::path::Path, body: &str) {
    std::fs::write(p, body).unwrap();
    std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap();
}

impl Box_ {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        exe(&bin.join("docker"), DOCKER);
        exe(&bin.join("aws"), AWS);
        exe(&bin.join("zstd"), "#!/bin/sh\ncat \"$2\"\n");
        Self { dir }
    }
    fn p(&self, n: &str) -> std::path::PathBuf {
        self.dir.path().join(n)
    }
    fn touch(&self, n: &str) {
        std::fs::write(self.p(n), "").unwrap();
    }
    /// A resource whose pin file lives in the fake dir and matches the fake tar.
    fn res(&self, phase: &str) -> Resource {
        let real = String::from_utf8(
            std::process::Command::new("sh")
                .arg("-c")
                .arg("printf 'tarcontent\\n' | sha256sum | cut -d' ' -f1")
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        let pin = format!(
            "image\tid\t{ID}\nimage\ttar_sha256\t{}\nimage\ts3_key\timage/x.tar.zst\n",
            real.trim()
        );
        std::fs::write(self.p("pin.tsv"), pin).unwrap();
        let mut r = res();
        r.spot.phase = Some(phase.into());
        r.spot.pin_file = Some(self.p("pin.tsv").display().to_string());
        r.spot.jit_source = Some(format!("dir:{}", self.p("jit").display()));
        r
    }
    fn run(&self, script: &str) -> (i32, String) {
        let path = format!(
            "{}:{}",
            self.p("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let o = std::process::Command::new("sh")
            .arg("-c")
            .arg(script)
            .env("PATH", path)
            .env("FAKE_DIR", self.dir.path())
            .env("FORJAR_SPOT_LOG", self.p("spot.log"))
            .env("FORJAR_SPOT_WORK", self.p("work"))
            .env("FORJAR_SPOT_SEATLOGS", self.p("seatlogs"))
            .env("FORJAR_SPOT_WAIT", "2")
            .env("FORJAR_SPOT_POLL", "1")
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        (o.status.code().unwrap_or(-1), text)
    }
    fn log(&self) -> String {
        std::fs::read_to_string(self.p("log")).unwrap_or_default()
    }
    fn spot_log(&self) -> String {
        std::fs::read_to_string(self.p("spot.log")).unwrap_or_default()
    }
}

#[test]
fn image_load_loads_the_pinned_image_from_the_pin() {
    let b = Box_::new();
    let (rc, out) = b.run(&apply_script(&b.res("image-load")));
    assert_eq!(rc, 0, "{out}");
    assert!(b.spot_log().contains(&format!("image-loaded:{ID}")));
    assert!(b.log().contains("s3 cp s3://img-bucket/image/x.tar.zst"));
    // second apply: already present, no fetch
    let before = b.log().matches("s3 cp").count();
    let (rc, _) = b.run(&apply_script(&b.res("image-load")));
    assert_eq!(rc, 0);
    assert_eq!(b.log().matches("s3 cp").count(), before);
}

#[test]
fn image_missing_after_load_is_non_zero_with_a_log_line() {
    let b = Box_::new();
    b.touch("quirk");
    let (rc, out) = b.run(&apply_script(&b.res("image-load")));
    assert_ne!(rc, 0);
    assert!(out.contains("image-missing:"), "{out}");
    assert!(b.spot_log().contains("image-missing:"));
    assert!(b.log().contains("docker image ls -a"), "uses image ls -a");
}

#[test]
fn bad_sha_fetch_and_load_failures_are_non_zero() {
    let b = Box_::new();
    let mut r = b.res("image-load");
    r.spot.pin_file = Some(b.p("pin.tsv").display().to_string());
    // rewrite the sha to a wrong-but-valid value
    let bad = format!(
        "image\tid\t{ID}\nimage\ttar_sha256\t{}\nimage\ts3_key\timage/x.tar.zst\n",
        "a".repeat(64)
    );
    std::fs::write(b.p("pin.tsv"), bad).unwrap();
    let (rc, out) = b.run(&apply_script(&r));
    assert_ne!(rc, 0);
    assert!(out.contains("image-sha256-mismatch"), "{out}");

    let b = Box_::new();
    b.touch("fetchfail");
    let (rc, out) = b.run(&apply_script(&b.res("image-load")));
    assert_ne!(rc, 0);
    assert!(out.contains("image-fetch-failed"), "{out}");

    let b = Box_::new();
    b.touch("loadfail");
    let (rc, out) = b.run(&apply_script(&b.res("image-load")));
    assert_ne!(rc, 0);
    assert!(out.contains("image-load-failed"), "{out}");
}

#[test]
fn unreadable_pin_is_refused_non_zero() {
    let b = Box_::new();
    let mut r = b.res("image-load");
    r.spot.pin_file = Some("/nonexistent/pin.tsv".into());
    let (rc, out) = b.run(&apply_script(&r));
    assert_ne!(rc, 0);
    assert!(out.contains("REFUSED"), "{out}");
}

#[test]
fn no_script_spells_a_digest_but_the_pin_one() {
    let b = Box_::new();
    let s = apply_script(&b.res("image-load"));
    assert_eq!(s.matches("sha256:").count(), s.matches(ID).count());
    assert!(!apply_script(&b.res("launch")).contains("sha256:"));
}

fn jits(b: &Box_) {
    std::fs::create_dir_all(b.p("jit")).unwrap();
    for n in ["seat-forjar-spot-01", "seat-forjar-spot-02"] {
        std::fs::write(b.p("jit").join(format!("{n}.jit")), "jitconfig").unwrap();
    }
}

#[test]
fn seat_start_runs_one_container_per_jit_and_logs_each() {
    let b = Box_::new();
    b.touch("loaded");
    jits(&b);
    let (rc, out) = b.run(&apply_script(&b.res("seat-start")));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(b.log().matches("docker run -d").count(), 2);
    assert!(b.p("seatlogs/seat-forjar-spot-01.log").exists());
    assert!(b.p("seatlogs/seat-forjar-spot-02.log").exists());
    let (rc, _) = b.run(&check_script(&b.res("seat-start")));
    let _ = rc;
}

#[test]
fn seat_that_never_reaches_running_is_non_zero_with_log() {
    let b = Box_::new();
    b.touch("loaded");
    b.touch("never");
    jits(&b);
    let (rc, out) = b.run(&apply_script(&b.res("seat-start")));
    assert_ne!(rc, 0);
    assert!(
        out.contains("seat-never-running:seat-forjar-spot-01"),
        "{out}"
    );
    assert!(
        std::fs::read_to_string(b.p("seatlogs/seat-forjar-spot-01.log"))
            .unwrap()
            .contains("runner log line")
    );
}

#[test]
fn docker_run_failure_is_kept_in_the_seat_log() {
    let b = Box_::new();
    b.touch("loaded");
    b.touch("runfail");
    jits(&b);
    let (rc, out) = b.run(&apply_script(&b.res("seat-start")));
    assert_ne!(rc, 0);
    assert!(out.contains("seat-docker-run-failed"), "{out}");
    assert!(
        std::fs::read_to_string(b.p("seatlogs/seat-forjar-spot-01.log"))
            .unwrap()
            .contains("Error response")
    );
}

#[test]
fn seat_start_without_image_or_jit_fails() {
    let b = Box_::new();
    let (rc, out) = b.run(&apply_script(&b.res("seat-start")));
    assert_ne!(rc, 0);
    assert!(out.contains("seat-image-missing"), "{out}");
    b.touch("loaded");
    let (rc, out) = b.run(&apply_script(&b.res("seat-start")));
    assert_ne!(rc, 0);
    assert!(out.contains("seat-jit-missing"), "{out}");
}

#[test]
fn launch_refuses_over_box_cap_with_exit_3_and_never_runs() {
    let b = Box_::new();
    std::fs::write(b.p("live"), "i-1\ti-2\n").unwrap();
    let (rc, out) = b.run(&apply_script(&b.res("launch")));
    assert_eq!(rc, 3, "{out}");
    assert!(!b.log().contains("run-instances"));
}

#[test]
fn launch_refuses_over_usd_cap_and_launches_spot_otherwise() {
    let b = Box_::new();
    std::fs::write(b.p("live"), "i-1\n").unwrap();
    std::fs::write(b.p("prices"), "0.6\n").unwrap();
    let (rc, out) = b.run(&apply_script(&b.res("launch")));
    assert_eq!(rc, 3, "{out}");
    let b = Box_::new();
    let (rc, out) = b.run(&apply_script(&b.res("launch")));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("launched i-launched"));
    let l = b.log();
    assert!(
        l.contains("MarketType=spot")
            && l.contains("one-time")
            && l.contains("Key=Owner,Value=forjar-spot")
    );
    assert!(!l.contains("on-demand"));
}

#[test]
fn launch_with_no_capacity_exits_75() {
    let b = Box_::new();
    b.touch("nocap");
    let (rc, out) = b.run(&apply_script(&b.res("launch")));
    assert_eq!(rc, 75, "{out}");
    assert_eq!(b.log().matches("run-instances").count(), 2);
}

#[test]
fn teardown_script_orders_evidence_before_terminate_and_reads_back() {
    let b = Box_::new();
    let s = apply_script(&b.res("teardown"));
    let ship = s.find("send-command").unwrap();
    let cancel = s.find("cancel-spot-instance-requests").unwrap();
    let term = s.find("terminate-instances").unwrap();
    assert!(ship < cancel && cancel < term);
    assert!(s.contains("state shows") && s.contains("s3://evidence-bucket/spot/"));
    let (rc, out) = b.run(&s);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("state shows 0"));
}

#[test]
fn teardown_without_evidence_dest_is_refused() {
    let b = Box_::new();
    let mut r = b.res("teardown");
    r.spot.evidence_dest = Some("https://x".into());
    assert!(apply_script(&r).contains("REFUSED"));
}

#[test]
fn register_script_deregisters_stale_and_mints_jit() {
    let b = Box_::new();
    let s = apply_script(&b.res("register"));
    for needle in [
        "-X DELETE",
        "generate-jitconfig",
        "labels[]=self-hosted",
        "labels[]=tier2",
        "runner-groups",
        "umask 077",
    ] {
        assert!(s.contains(needle), "missing {needle}");
    }
}

#[test]
fn check_and_state_scripts_cover_every_phase() {
    let b = Box_::new();
    for p in ["launch", "image-load", "seat-start", "register", "teardown"] {
        let r = b.res(p);
        assert!(!check_script(&r).contains("REFUSED"), "{p}");
        assert!(!state_query_script(&r).contains("REFUSED"), "{p}");
    }
    b.touch("loaded");
    let (rc, _) = b.run(&check_script(&b.res("image-load")));
    assert_eq!(rc, 0);
    let (rc, _) = b.run(&check_script(&b.res("teardown")));
    assert_eq!(rc, 0);
    std::fs::write(b.p("live"), "i-1\n").unwrap();
    let (rc, _) = b.run(&check_script(&b.res("teardown")));
    assert_ne!(rc, 0);
    let (rc, out) = b.run(&state_query_script(&b.res("launch")));
    assert_eq!(rc, 0);
    assert!(out.contains("boxes=1"), "{out}");
}

#[test]
fn invalid_declaration_script_is_a_refusal() {
    let mut r = res();
    r.spot.market = Some("on-demand".into());
    for s in [apply_script(&r), check_script(&r), state_query_script(&r)] {
        assert!(s.contains("REFUSED") && s.contains("exit 1"));
    }
}
