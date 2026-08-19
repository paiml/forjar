//! GH-239: the store must be usable by a process that is not root.
//!
//! `STORE_BASE` was a compile-time constant, `/var/lib/forjar/store`, with no
//! environment or config override, and `store_entry_path()` — the public path
//! API — could not be pointed anywhere else. `/var/lib` is root-owned on every
//! mainstream distribution, so an ordinary user got:
//!
//! ```text
//! $ forjar store list
//! error: read /var/lib/forjar/store: No such file or directory (os error 2)
//! $ mkdir -p /var/lib/forjar/store
//! mkdir: cannot create directory '/var/lib/forjar': Permission denied
//! ```
//!
//! with no next move. That is every non-root caller: CI, library consumers, and
//! anyone evaluating forjar without handing it root.
//!
//! These run the real binary, because the claim is about what a user can do.

use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

#[test]
fn store_list_works_in_a_directory_the_caller_owns() {
    // The reproduction from the issue, inverted: given a store root the caller
    // can actually write, `store list` must succeed rather than fail on a
    // root-owned path it was never able to reach.
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    std::fs::create_dir_all(&store).unwrap();

    let out = forjar()
        .arg("store")
        .arg("list")
        .env("FORJAR_STORE", &store)
        .output()
        .expect("forjar must run");

    assert!(
        out.status.success(),
        "store list failed for a writable store root:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn the_store_env_var_is_not_ignored() {
    // The precise assertion the issue's repro failed on:
    //   "FORJAR_STORE env is ignored: set=Err(NotPresent)"
    // An explicit root must win over both the system default and the per-user
    // fallback, so that a misconfiguration surfaces on the path the operator
    // named instead of being silently redirected.
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("explicit-store");
    std::fs::create_dir_all(&store).unwrap();

    let out = forjar()
        .arg("store")
        .arg("list")
        .arg("--json")
        .env("FORJAR_STORE", &store)
        .output()
        .expect("forjar must run");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        !combined.contains("/var/lib/forjar/store"),
        "FORJAR_STORE was ignored — output still names the root-owned default:\n{combined}"
    );
}

#[test]
fn an_unprivileged_process_does_not_resolve_to_the_root_owned_default() {
    // With no FORJAR_STORE and no write access to /var/lib, the resolved root
    // must fall back to the caller's own data directory. If /var/lib/forjar IS
    // writable here (CI running as root, or a real system install), the system
    // path is correct and the fallback must NOT engage — so the assertion is
    // conditional on what this host actually allows, not on an assumption.
    let system_writable = std::fs::create_dir_all("/var/lib/forjar/store").is_ok();

    let home = tempfile::tempdir().unwrap();
    let out = forjar()
        .arg("store")
        .arg("list")
        .env_remove("FORJAR_STORE")
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("data"))
        .output()
        .expect("forjar must run");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    if system_writable {
        // Privileged/system install: unchanged behaviour is the requirement.
        assert!(
            out.status.success(),
            "system store is writable but store list failed:\n{combined}"
        );
    } else {
        assert!(
            !combined.contains("/var/lib/forjar/store"),
            "unprivileged process still resolved to the root-owned default, \
             which is the GH-239 dead end:\n{combined}"
        );
    }
}

#[test]
fn store_gc_reaches_the_same_root_as_store_list() {
    // `gc` and `list` took the store path by separate routes. If they disagree,
    // `gc --dry-run` reports on a directory the user never writes to — a
    // reclaim tool that observes something other than what it defends.
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    std::fs::create_dir_all(&store).unwrap();

    for args in [vec!["store", "list"], vec!["store", "gc", "--dry-run"]] {
        let out = forjar()
            .args(&args)
            .env("FORJAR_STORE", &store)
            .output()
            .expect("forjar must run");
        assert!(
            out.status.success(),
            "`forjar {}` failed against a writable store root:\nstdout: {}\nstderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn an_absent_store_reads_as_empty_not_as_an_error() {
    // The first run. The store is created by the first import, not by listing
    // it, so `list` and `gc` on a not-yet-existing store must report an empty
    // store. Reporting ENOENT failed the operator at exactly the moment they
    // were checking whether the store was safe to enable.
    let dir = tempfile::tempdir().unwrap();
    let never_created = dir.path().join("no-store-here");
    assert!(!never_created.exists());

    for args in [vec!["store", "list"], vec!["store", "gc", "--dry-run"]] {
        let out = forjar()
            .args(&args)
            .env("FORJAR_STORE", &never_created)
            .output()
            .expect("forjar must run");
        assert!(
            out.status.success(),
            "`forjar {}` treated an absent store as an error:\nstdout: {}\nstderr: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // And listing must not have created it as a side effect: a read is a read.
    assert!(
        !never_created.exists(),
        "listing an absent store created it; reads must not mutate the store"
    );
}

#[test]
fn a_permission_error_is_still_an_error() {
    // The ENOENT-is-empty rule must not swallow the case the operator has to
    // know about. Guarded on the probe actually being unreadable, so this does
    // not silently pass when run as root.
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    let mut perms = std::fs::metadata(&locked).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o000);
    std::fs::set_permissions(&locked, perms).unwrap();

    let readable = std::fs::read_dir(&locked).is_ok();
    if readable {
        // Running as root, or on a filesystem that ignores mode bits.
        return;
    }

    let out = forjar()
        .args(["store", "list"])
        .env("FORJAR_STORE", &locked)
        .output()
        .expect("forjar must run");
    assert!(
        !out.status.success(),
        "an unreadable store must be an error, not silently empty"
    );
}
