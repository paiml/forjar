//! forjar#658: a failed `mktemp` must not turn `$_STAGING/bin` into `/bin`.
//!
//! The cargo install body was safe only because the script opens with
//! `set -euo pipefail`. errexit is suspended wherever a status is tested, so
//! run as `apply || report` a failed `mktemp -d` left `_STAGING` empty and the
//! script carried on: `"$_STAGING/bin"` read as `/bin`, which exists and is
//! not empty, so the "produced no binaries" check passed, the system `/bin`
//! was cached under the crate's key, and every file in it was installed into
//! `$CARGO_HOME/bin`. The next apply would report `cache-hit`.
//!
//! This EXECUTES the emitted script, in exactly that status-tested context,
//! with `mktemp -d` stubbed to fail, and asserts nothing reached the cache or
//! `$CARGO_HOME/bin`.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::path::Path;
use std::process::Command;

fn write_exec(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(path, perms).unwrap();
}

fn entries(dir: &Path) -> usize {
    std::fs::read_dir(dir).map(|d| d.count()).unwrap_or(0)
}

/// Run the cargo apply script for `crate_name` as the body of a function
/// whose status is tested, so errexit is off, with `mktemp -d` made to fail
/// when `fail_mktemp` is set. Returns (cache entries, `$CARGO_HOME/bin`
/// entries, stderr).
fn run(fail_mktemp: bool) -> (usize, usize, String) {
    let root = tempfile::tempdir().unwrap();
    let stubs = root.path().join("stubs");
    let cargo_home = root.path().join("cargo-home");
    let cache = root.path().join("cache");
    std::fs::create_dir_all(&stubs).unwrap();
    std::fs::create_dir_all(cargo_home.join("bin")).unwrap();
    std::fs::create_dir_all(&cache).unwrap();

    // `cargo install --root R` builds nothing; with a real R it lays down one
    // binary, which is what a working install looks like.
    write_exec(
        &stubs.join("cargo"),
        "#!/bin/sh\nif [ \"$1\" = install ] && [ \"$2\" = --list ]; then exit 0; fi\n\
         root=\nwhile [ $# -gt 0 ]; do [ \"$1\" = --root ] && root=$2; shift; done\n\
         [ -n \"$root\" ] || exit 0\nmkdir -p \"$root/bin\"\n\
         printf '#!/bin/sh\\necho widget 1.0.0\\n' > \"$root/bin/widget\"\n\
         chmod 755 \"$root/bin/widget\"\n",
    );
    let real = Command::new("sh")
        .args(["-c", "command -v mktemp"])
        .output()
        .unwrap();
    let real = String::from_utf8(real.stdout).unwrap();
    let fail = if fail_mktemp {
        "case \"$*\" in *forjar-cargo*) exit 1 ;; esac\n"
    } else {
        ""
    };
    write_exec(
        &stubs.join("mktemp"),
        &format!("#!/bin/sh\n{fail}exec {} \"$@\"\n", real.trim()),
    );

    let resource = Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("local".to_string()),
        provider: Some("cargo".to_string()),
        packages: vec!["widget".to_string()],
        ..Default::default()
    };
    let script = forjar::resources::package::apply_script(&resource);
    let wrapped = format!("apply() {{\n{script}\n}}\napply || echo 'apply reported failure' >&2\n");

    let out = Command::new("bash")
        .arg("-c")
        .arg(wrapped)
        .env("PATH", format!("{}:/usr/bin:/bin", stubs.display()))
        .env("HOME", root.path())
        .env("CARGO_HOME", &cargo_home)
        .env("FORJAR_CACHE_DIR", &cache)
        .env("CARGO_BUILD_JOBS", "1")
        .output()
        .expect("bash must run");
    (
        entries(&cache),
        entries(&cargo_home.join("bin")),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn control_a_working_staging_dir_installs_the_crate() {
    // Without this, a script that installs nothing at all would pass the
    // assertion below.
    let (cache, bins, stderr) = run(false);
    assert_eq!(cache, 1, "one cache entry for the crate; stderr:\n{stderr}");
    assert!(
        bins >= 1,
        "the crate's binary must be installed; stderr:\n{stderr}"
    );
}

#[test]
fn a_failed_mktemp_installs_and_caches_nothing() {
    let (cache, bins, stderr) = run(true);
    assert_eq!(
        cache, 0,
        "a failed mktemp must not populate the cache (it cached /bin); stderr:\n{stderr}"
    );
    assert_eq!(
        bins, 0,
        "a failed mktemp must install nothing (it installed /bin); stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("cannot create a staging dir"),
        "the failure must be named; stderr:\n{stderr}"
    );
}
