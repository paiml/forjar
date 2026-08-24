//! forjar#310: `>` and `chmod`/`chown` FOLLOW a symlink, so a symlink at a
//! managed path made `apply` write the declared content and mode onto an
//! arbitrary file — with forjar's privileges, which on the paiml fleet is root.
//!
//! Measured before the fix: a managed path swapped for a link to
//! `victim/important.conf` left that file reading `SECRET=managed-payload` with
//! its mode changed 664 -> 600, while apply printed `1 converged`.
//! `policy.deny_paths` does not stop it, because the path forjar was ASKED to
//! write really is allowed — the redirection happens below that check.
//!
//! Pre-#307 the converged case could not be reached: the old drift gate refused
//! the apply outright, protecting the victim BY ACCIDENT. #307 removed that
//! refusal and the latent defect became reachable. It is pre-existing either way.
//!
//! `state: file` DECLARES a regular file at that path, so converging means
//! REPLACING the link, not following it.
//!
//! Every assertion is on the VICTIM's bytes and mode — never on apply's summary
//! line, which said `1 converged` throughout.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

const PAYLOAD: &str = "SECRET=managed-payload\n";
const VICTIM: &str = "ORIGINAL-VICTIM-DATA\n";

struct Sb {
    dir: tempfile::TempDir,
}

impl Sb {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::create_dir_all(dir.path().join("victim")).unwrap();
        fs::write(dir.path().join("src.txt"), PAYLOAD).unwrap();
        Self { dir }
    }
    fn p(&self, r: &str) -> std::path::PathBuf {
        self.dir.path().join(r)
    }
    fn cfg(&self) -> std::path::PathBuf {
        let c = self.p("f.yaml");
        fs::write(
            &c,
            format!(
                "version: \"1.0\"\nname: t\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\nresources:\n  f: {{ type: file, machine: local, path: {}, source: {}, mode: \"0600\" }}\n",
                self.p("managed.conf").display(),
                self.p("src.txt").display()
            ),
        )
        .unwrap();
        c
    }
    fn apply(&self, cfg: &Path) {
        let _ = Command::new(forjar())
            .args([
                "apply",
                "-f",
                cfg.to_str().unwrap(),
                "--state-dir",
                self.p("state").to_str().unwrap(),
                "--yes",
            ])
            .output()
            .expect("forjar failed to start");
    }
}

#[test]
fn apply_does_not_write_through_a_symlink_at_a_managed_path() {
    let sb = Sb::new();
    let cfg = sb.cfg();
    let victim = sb.p("victim/important.conf");
    fs::write(&victim, VICTIM).unwrap();
    fs::set_permissions(&victim, fs::Permissions::from_mode(0o664)).unwrap();

    sb.apply(&cfg); // converge normally

    // Swap the managed path for a symlink pointing at the victim.
    fs::remove_file(sb.p("managed.conf")).ok();
    std::os::unix::fs::symlink(&victim, sb.p("managed.conf")).unwrap();

    sb.apply(&cfg);

    assert_eq!(
        fs::read_to_string(&victim).unwrap(),
        VICTIM,
        "apply wrote the managed content THROUGH the symlink onto an unrelated file"
    );
    assert_eq!(
        fs::metadata(&victim).unwrap().permissions().mode() & 0o777,
        0o664,
        "apply chmod'd THROUGH the symlink onto an unrelated file"
    );

    // The control, and it is what stops "refuse to do anything" passing: the
    // managed path must still end up correct — a REGULAR file with the
    // declared bytes, because that is what `state: file` declares.
    let managed = sb.p("managed.conf");
    assert!(
        !fs::symlink_metadata(&managed)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the symlink at the managed path was not replaced"
    );
    assert_eq!(
        fs::read_to_string(&managed).unwrap(),
        PAYLOAD,
        "the managed path does not hold the declared content"
    );
}

#[test]
fn a_dangling_symlink_at_a_managed_path_is_not_a_create_primitive() {
    // Worse than the overwrite: `>` CREATES the target, so a dangling link is
    // a write primitive at any path it names.
    let sb = Sb::new();
    let cfg = sb.cfg();
    sb.apply(&cfg);

    let target = sb.p("victim/newfile");
    fs::remove_file(sb.p("managed.conf")).ok();
    std::os::unix::fs::symlink(&target, sb.p("managed.conf")).unwrap();

    sb.apply(&cfg);

    assert!(
        !target.exists(),
        "apply CREATED a file at the dangling symlink's target — a write \
         primitive at an arbitrary path"
    );
    assert_eq!(
        fs::read_to_string(sb.p("managed.conf")).unwrap(),
        PAYLOAD,
        "the managed path does not hold the declared content"
    );
}
