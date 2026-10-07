//! forjar#694: a `type: package` resource must RUN its `completion_check`.
//!
//! The field was accepted, validated and then ignored: `check.sh` asked the
//! provider (`dpkg -l`, `cargo install --list`) and the declared check
//! appeared in no generated script. An author writes `completion_check`
//! exactly when presence is not enough -- a JRE that is installed but is not
//! the java that runs -- so the box was graded on presence while the config
//! read as if the tool were executed.
//!
//! Like GH-254 for tasks, the scripts are EXECUTED here, not pattern-matched,
//! except where executing would need the real provider. `dpkg` is a stub on
//! PATH that reports every package installed, so the provider half of each
//! script passes and the verdict is the completion_check's alone.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use forjar::resources::{package, package_check};
use std::path::Path;
use std::process::Command;

fn pkg(provider: &str, completion_check: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("local".to_string()),
        provider: Some(provider.to_string()),
        packages: vec!["openjdk-17-jre-headless".to_string()],
        completion_check: completion_check.map(str::to_string),
        ..Default::default()
    }
}

/// A PATH whose `dpkg` says every package is installed (`ii`) and whose
/// `dpkg-query` prints a fixed version.
fn stub_bin() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let write = |name: &str, body: &str| {
        let p = dir.path().join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).expect("write stub");
        let mut perm = std::fs::metadata(&p).expect("stat stub").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perm, 0o755);
        std::fs::set_permissions(&p, perm).expect("chmod stub");
    };
    write("dpkg", "printf 'ii  %s 17 amd64 stub\\n' \"$2\"");
    write("dpkg-query", "echo openjdk-17-jre-headless=17");
    dir
}

fn run(script: &str, bin: &Path) -> std::process::Output {
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new("bash")
        .arg("-c")
        .arg(script)
        .env("PATH", path)
        .output()
        .expect("bash must run")
}

#[test]
fn check_sh_fails_when_the_package_is_present_but_the_completion_check_fails() {
    let bin = stub_bin();
    let failing = package_check::check_script(&pkg("apt", Some("false")));
    let out = run(&failing, bin.path());
    assert!(
        !out.status.success(),
        "dpkg says installed, the declared completion_check says no: the check must NOT pass.\nscript:\n{failing}"
    );

    let passing = package_check::check_script(&pkg("apt", Some("true")));
    let out = run(&passing, bin.path());
    assert!(
        out.status.success(),
        "installed and the completion_check holds: the check must pass.\nscript:\n{passing}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn apply_sh_fails_when_the_completion_check_still_fails_afterwards() {
    let bin = stub_bin();
    let failing = package::apply_script(&pkg("apt", Some("false")));
    let out = run(&failing, bin.path());
    assert!(
        !out.status.success(),
        "an apply that leaves the completion_check false has not converged.\nscript:\n{failing}"
    );

    let passing = package::apply_script(&pkg("apt", Some("true")));
    let out = run(&passing, bin.path());
    assert!(
        out.status.success(),
        "an apply whose completion_check holds converged.\nscript:\n{passing}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn the_state_query_changes_when_the_completion_check_changes() {
    let bin = stub_bin();
    let pass = run(
        &package::state_query_script(&pkg("apt", Some("true"))),
        bin.path(),
    );
    let fail = run(
        &package::state_query_script(&pkg("apt", Some("false"))),
        bin.path(),
    );
    assert_ne!(
        pass.stdout, fail.stdout,
        "drift hashes the state query; a completion_check that stopped holding must change it"
    );
}

#[test]
fn every_provider_emits_the_declared_check_in_check_and_apply() {
    // The issue's repro: `grep -c 'java -version'` over the generated scripts
    // returned 0 for apt and for cargo. Executing cargo, uv or brew would need
    // the real tool, so for those this asserts the check is emitted; the apt
    // tests above prove an emitted check decides the verdict.
    let check = "java -version 2>&1 | grep -cE '^openjdk version \"17\\.' >/dev/null";
    for provider in ["apt", "cargo", "uv", "brew"] {
        let r = pkg(provider, Some(check));
        for (what, script) in [
            ("check", package_check::check_script(&r)),
            ("apply", package::apply_script(&r)),
            ("state_query", package::state_query_script(&r)),
        ] {
            assert!(
                script.contains("java -version"),
                "provider {provider}: the {what} script never runs the declared completion_check:\n{script}"
            );
        }
    }
}

#[test]
fn without_a_completion_check_the_scripts_are_unchanged() {
    // The field is opt-in: a package with no completion_check must emit no
    // completion_check line, so no existing lock entry's digest moves.
    let r = pkg("apt", None);
    for script in [
        package_check::check_script(&r),
        package::apply_script(&r),
        package::state_query_script(&r),
    ] {
        assert!(
            !script.contains("completion_check"),
            "a package without a completion_check grew a completion_check line:\n{script}"
        );
    }
}

#[test]
fn every_script_carrying_a_completion_check_passes_the_apply_gate() {
    // forjar runs bashrs over every script it ships (I8); a check that makes
    // the generated script fail that gate would turn a declared package into
    // an aborted apply. Both a `|` block scalar (trailing newline) and a `>-`
    // folded one (a loop collapsed onto one line) are tried.
    //
    // The verdict is relative to the same script WITHOUT a completion_check:
    // the cargo apply script already carries its own I8 findings, and this
    // test is about what the completion_check adds, which must be nothing.
    let checks = [
        "java -version 2>&1 | grep -cE '^openjdk version \"17\\.' >/dev/null\n",
        "sh -c 'for b in java keytool; do command -v \"$b\" >/dev/null || exit 1; done; exit 0'",
    ];
    let scripts = |r: &Resource| {
        [
            ("check", package_check::check_script(r)),
            ("apply", package::apply_script(r)),
            ("state_query", package::state_query_script(r)),
        ]
    };
    for check in checks {
        for provider in ["apt", "cargo", "uv", "brew"] {
            let baseline = scripts(&pkg(provider, None));
            let with_check = scripts(&pkg(provider, Some(check)));
            for ((what, before), (_, after)) in baseline.iter().zip(with_check.iter()) {
                let before = forjar::core::purifier::validate_script(before).err();
                let after_v = forjar::core::purifier::validate_script(after).err();
                assert_eq!(
                    after_v, before,
                    "provider {provider}: the completion_check changed the {what} script's I8 verdict\n{after}"
                );
            }
        }
    }
}
