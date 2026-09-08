//! paiml/infra#208: the cargo drift observable must EXECUTE differently when a
//! crate's binaries are gone.
//!
//! Every other assertion about this observable reads the script's TEXT; this
//! one runs it, because the defect being fixed was that the text looked
//! perfectly correct while reporting `installed` over an empty bin directory.
//! Measured 2026-08-24 on intel: `cargo-kani` and `kani` both absent,
//! `cargo install --list` still naming them, and `forjar drift` across the
//! whole machine reporting "No drift detected" eight times.
//!
//! Moved out of `src/resources/tests_package_b.rs` for forjar#489: the
//! observable now repairs PATH from `${CARGO_HOME:-$HOME/.cargo}/bin` before
//! it asks cargo anything, so the test has to say where its FAKE toolchain
//! lives — and that file was at the repo's 500-line ceiling.

use forjar::core::types::{MachineTarget, Resource, ResourceType};

#[cfg(unix)]
#[test]
fn cargo_observable_reports_a_deleted_binary_as_gone() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().expect("tempdir");
    let bin = tmp.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();

    // A fake `cargo` whose `install --list` reports a crate with two binaries,
    // exactly as the real one formats it.
    let fake_cargo = bin.join("cargo");
    std::fs::write(
        &fake_cargo,
        "#!/bin/sh\nprintf 'demo-crate v1.0.0:\\n    demo-one\\n    demo-two\\n'\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake_cargo, std::fs::Permissions::from_mode(0o755)).unwrap();

    let r = Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("local".to_string()),
        provider: Some("cargo".to_string()),
        packages: vec!["demo-crate".to_string()],
        ..Default::default()
    };
    let script = forjar::resources::package::state_query_script(&r);

    let run = |extra_bins: &[&str]| -> String {
        for b in extra_bins {
            let p = bin.join(b);
            std::fs::write(&p, "#!/bin/sh\nexit 0\n").unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // The fake bin dir FIRST so our `cargo` shadows the real one and the
        // demo binaries resolve — but the system dirs must stay, or `awk`,
        // `grep` and `sh` itself disappear and the test measures nothing.
        // (First cut set PATH to the fake dir alone and died on ENOENT for
        // `sh` — a test that cannot run is not a passing test.)
        let path = format!("{}:/usr/bin:/bin", bin.to_str().unwrap());
        // CARGO_HOME must name the fake toolchain too (forjar#489). The
        // observable now PREPENDS `${CARGO_HOME:-$HOME/.cargo}/bin` to PATH —
        // the same repair the install action makes, so that a check cannot
        // resolve a different cargo than the apply that wrote the crate. With
        // CARGO_HOME unset this process's real ~/.cargo/bin would land ahead
        // of `bin` and the REAL cargo would answer `install --list`, which
        // reports `demo-crate=MISSING` and measures nothing.
        let out = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(&script)
            .env("PATH", &path)
            .env("CARGO_HOME", tmp.path())
            .output()
            .expect("run observable");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    // Both binaries absent: registered, but nothing on disk.
    let gone = run(&[]);
    assert!(
        gone.contains("demo-one:GONE") && gone.contains("demo-two:GONE"),
        "a registered crate with NO binaries must not read as healthy, got: {gone}"
    );

    // Both present.
    let ok = run(&["demo-one", "demo-two"]);
    assert!(
        ok.contains("demo-one:ok") && ok.contains("demo-two:ok"),
        "a fully installed crate must read as ok, got: {ok}"
    );

    // THE POINT: the two states must be DISTINGUISHABLE. The old observable
    // emitted the identical string for both, which is why a fleet-wide binary
    // deletion produced zero drift findings.
    assert_ne!(
        gone, ok,
        "the observable produces the SAME output whether the binaries exist or \
         not — it cannot generate a drift signal"
    );
}
