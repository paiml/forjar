//! Tests for codegen dispatch — FJ-005, FJ-040, falsification tests.

use super::test_fixtures::*;
use super::*;
use crate::core::types::{Resource, ResourceType};

#[test]
fn test_fj005_check_dispatches_package() {
    let r = make_package();
    let script = check_script(&r).unwrap();
    assert!(script.contains("dpkg"));
}

#[test]
fn test_fj005_check_dispatches_file() {
    let r = make_file();
    let script = check_script(&r).unwrap();
    assert!(script.contains("test -f"));
}

#[test]
fn test_fj005_check_dispatches_service() {
    let r = make_service();
    let script = check_script(&r).unwrap();
    assert!(script.contains("systemctl"));
}

#[test]
fn test_fj005_check_dispatches_mount() {
    let r = make_mount();
    let script = check_script(&r).unwrap();
    // Dispatch-level: only that a mount check was produced.
    assert!(
        script.contains("findmnt") || script.contains("mountpoint"),
        "{script}"
    );
}

#[test]
fn test_fj005_apply_dispatches_package() {
    let r = make_package();
    let script = apply_script(&r).unwrap();
    assert!(script.contains("apt-get install"));
}

#[test]
fn test_fj005_apply_dispatches_file() {
    let r = make_file();
    let script = apply_script(&r).unwrap();
    // Dispatch-level: only that a file WRITE was produced. C8 (GH #296) replaced the
    // heredoc transport with a delimiter-free base64 one.
    assert!(script.contains("| base64 -d > "), "{script}");
}

#[test]
fn test_fj005_apply_dispatches_service() {
    let r = make_service();
    let script = apply_script(&r).unwrap();
    assert!(script.contains("systemctl start"));
}

#[test]
fn test_fj005_apply_dispatches_mount() {
    let r = make_mount();
    let script = apply_script(&r).unwrap();
    assert!(script.contains("mount -t 'nfs'"));
}

#[test]
fn test_fj005_state_query_dispatches() {
    let r = make_package();
    let script = state_query_script(&r).unwrap();
    assert!(script.contains("dpkg-query"));
}

#[test]
fn test_fj005_state_query_file() {
    let mut r = make_package();
    r.resource_type = ResourceType::File;
    r.path = Some("/etc/conf".to_string());
    let script = state_query_script(&r).unwrap();
    assert!(script.contains("stat") || script.contains("/etc/conf"));
}

#[test]
fn test_fj005_state_query_service() {
    let mut r = make_package();
    r.resource_type = ResourceType::Service;
    r.name = Some("nginx".to_string());
    let script = state_query_script(&r).unwrap();
    assert!(script.contains("nginx"));
}

#[test]
fn test_fj005_pepita_supported() {
    let mut r = make_package();
    r.resource_type = ResourceType::Pepita;
    r.name = Some("sandbox".to_string());
    r.netns = true;
    assert!(check_script(&r).is_ok());
    assert!(apply_script(&r).is_ok());
    assert!(state_query_script(&r).is_ok());
}

/// FALSIFY-CD-001: All Phase 1 types produce Ok for all three functions.
#[test]
fn falsify_cd_001_dispatch_completeness() {
    let resources = [make_package(), make_file(), make_service(), make_mount()];
    for r in &resources {
        assert!(
            check_script(r).is_ok(),
            "check_script failed for {:?}",
            r.resource_type
        );
        assert!(
            apply_script(r).is_ok(),
            "apply_script failed for {:?}",
            r.resource_type
        );
        assert!(
            state_query_script(r).is_ok(),
            "state_query_script failed for {:?}",
            r.resource_type
        );
    }
}

#[test]
fn test_fj005_dispatch_user() {
    let mut r = make_package();
    r.resource_type = ResourceType::User;
    r.name = Some("deploy".to_string());
    assert!(check_script(&r).unwrap().contains("deploy"));
    assert!(apply_script(&r).unwrap().contains("useradd"));
    assert!(state_query_script(&r).unwrap().contains("deploy"));
}

#[test]
fn test_fj005_dispatch_docker() {
    let mut r = make_package();
    r.resource_type = ResourceType::Docker;
    r.image = Some("nginx:latest".to_string());
    r.name = Some("web".to_string());
    assert!(check_script(&r).unwrap().contains("docker"));
    assert!(apply_script(&r).unwrap().contains("docker"));
    assert!(state_query_script(&r).unwrap().contains("docker"));
}

#[test]
fn test_fj005_dispatch_cron() {
    let mut r = make_package();
    r.resource_type = ResourceType::Cron;
    r.schedule = Some("0 2 * * *".to_string());
    r.command = Some("/opt/backup.sh".to_string());
    assert!(check_script(&r).unwrap().contains("crontab"));
    assert!(apply_script(&r).unwrap().contains("backup.sh"));
    assert!(state_query_script(&r).unwrap().contains("crontab"));
}

#[test]
fn test_fj005_dispatch_network() {
    let mut r = make_package();
    r.resource_type = ResourceType::Network;
    r.port = Some("443".to_string());
    r.action = Some("allow".to_string());
    assert!(check_script(&r).unwrap().contains("ufw"));
    assert!(apply_script(&r).unwrap().contains("ufw"));
    assert!(state_query_script(&r).unwrap().contains("ufw"));
}

#[test]
fn test_fj005_apply_scripts_contain_pipefail() {
    let resources = [make_package(), make_file(), make_service(), make_mount()];
    for r in &resources {
        let script = apply_script(r).unwrap();
        assert!(
            script.contains("set -euo pipefail"),
            "apply script for {:?} must contain pipefail",
            r.resource_type
        );
    }
}

#[test]
fn test_fj040_pepita_codegen_check() {
    let mut r = make_package();
    r.resource_type = ResourceType::Pepita;
    r.name = Some("jail".to_string());
    r.chroot_dir = Some("/var/jail".to_string());
    let script = check_script(&r).unwrap();
    assert!(script.contains("chroot:present:jail"));
}

#[test]
fn test_fj005_recipe_type_not_dispatchable() {
    let mut r = make_package();
    r.resource_type = ResourceType::Recipe;
    assert!(
        check_script(&r).is_err(),
        "recipe type should not be directly dispatchable"
    );
    assert!(apply_script(&r).is_err());
    assert!(state_query_script(&r).is_err());
}

/// FALSIFY-CD-002: Dispatch is symmetric — same types handled by all three functions.
#[test]
fn falsify_cd_002_dispatch_symmetry() {
    let all_types = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::Docker,
        ResourceType::User,
        ResourceType::Network,
        ResourceType::Cron,
        ResourceType::Pepita,
    ];

    for rt in &all_types {
        let mut r = make_package();
        r.resource_type = rt.clone();

        let check_ok = check_script(&r).is_ok();
        let apply_ok = apply_script(&r).is_ok();
        let query_ok = state_query_script(&r).is_ok();

        assert_eq!(
            check_ok, apply_ok,
            "check/apply asymmetry for {rt:?}: check={check_ok}, apply={apply_ok}"
        );
        assert_eq!(
            apply_ok, query_ok,
            "apply/query asymmetry for {rt:?}: apply={apply_ok}, query={query_ok}"
        );
    }
}

#[test]
fn test_fj040_pepita_codegen_apply_netns() {
    let mut r = make_package();
    r.resource_type = ResourceType::Pepita;
    r.name = Some("net-sandbox".to_string());
    r.netns = true;
    let script = apply_script(&r).unwrap();
    assert!(script.contains("ip netns add 'forjar-net-sandbox'"));
}

// ── FJ-2722 (PMAT-199): `state: absent` must not RUN the thing ──────────────

/// `forjar destroy` converges every resource to `state: absent`. Task, build
/// and wasm_bundle handlers ignore `state`, so destroy EXECUTED the command —
/// running a build or a deploy as its way of "removing" it, and reporting
/// success. These types describe an action, and an action has no absent form.
#[test]
fn absent_action_types_generate_a_noop_not_the_command() {
    for ty in [
        ResourceType::Task,
        ResourceType::Build,
        ResourceType::WasmBundle,
    ] {
        let mut r = Resource {
            resource_type: ty.clone(),
            command: Some("rm -rf / --no-preserve-root".to_string()),
            ..Default::default()
        };
        r.state = Some("absent".to_string());
        r.target = Some("/usr/local/bin/thing".to_string());
        r.path = Some("/srv/app.wasm".to_string());

        let script = apply_script(&r).expect("generates");
        assert!(
            !script.contains("rm -rf /"),
            "destroying a {ty} must not run its command:\n{script}"
        );
        assert!(
            script.contains("no absent form"),
            "{ty} should explain itself:\n{script}"
        );
    }
}

#[test]
fn absent_state_still_destroys_types_that_have_an_absent_form() {
    // The guard must be narrow: a file with `state: absent` must still be
    // removed, or destroy would silently stop working entirely.
    let mut r = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    r.path = Some("/tmp/forjar-absent-guard".to_string());
    r.state = Some("absent".to_string());

    let script = apply_script(&r).expect("generates");
    assert!(
        script.contains("rm -rf '/tmp/forjar-absent-guard'"),
        "file resources must still honour state: absent:\n{script}"
    );
}

#[test]
fn action_types_are_unaffected_when_state_is_not_absent() {
    let r = Resource {
        resource_type: ResourceType::Task,
        command: Some("cc -c a.c".to_string()),
        ..Default::default()
    };
    let script = apply_script(&r).expect("generates");
    assert!(
        script.contains("cc -c a.c"),
        "a normal task must still run its command:\n{script}"
    );
}

#[test]
fn unresolved_secret_never_reaches_a_generated_script() {
    // `resolve_or_fallback` returns the UNRESOLVED resource when the secrets
    // provider fails, by design. Before this guard, a `file` whose content held
    // `{{secrets.NAME}}` generated a heredoc that wrote that literal string to
    // disk as though it were the credential — behind only a stderr warning.
    // `backup_sync` was the sole guarded type; this is the generic chokepoint.
    let r = Resource {
        resource_type: ResourceType::File,
        path: Some("/tmp/creds".to_string()),
        content: Some("API_KEY={{secrets.some-token}}".to_string()),
        ..Default::default()
    };
    let err = apply_script(&r).expect_err("must refuse to generate the script");
    assert!(
        err.contains("unresolved secret template"),
        "unexpected error: {err}"
    );
}

#[test]
fn a_resource_with_no_secrets_still_generates() {
    // The guard must not be a blanket refusal: ordinary resources are unaffected.
    let r = Resource {
        resource_type: ResourceType::File,
        path: Some("/tmp/plain".to_string()),
        content: Some("hello".to_string()),
        ..Default::default()
    };
    let script = apply_script(&r).expect("must still generate");
    assert!(script.contains("/tmp/plain"), "unexpected script: {script}");
}

/// FALSIFY-CD-001/002 over the WHOLE enum, not a hand-picked subset.
///
/// The two falsification tests above enumerate 4 and 9 types respectively, so
/// twelve resource types had no completeness or symmetry coverage at all. The
/// dispatch table routes each type once, which makes symmetry structural —
/// this pins completeness to match: every type except `recipe` generates all
/// three scripts, and `recipe` refuses all three.
#[test]
fn falsify_cd_001_002_every_resource_type_dispatches() {
    let all = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Docker,
        ResourceType::Pepita,
        ResourceType::Network,
        ResourceType::Cron,
        ResourceType::Recipe,
        ResourceType::Model,
        ResourceType::Gpu,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
        ResourceType::GithubRelease,
        ResourceType::OverlayInterface,
        ResourceType::DiskBudget,
        ResourceType::BackupSync,
        ResourceType::NasArchive,
    ];

    for rt in all {
        let r = Resource {
            resource_type: rt.clone(),
            ..Default::default()
        };
        let dispatchable = rt != ResourceType::Recipe;

        assert_eq!(
            check_script(&r).is_ok(),
            dispatchable,
            "check_script({rt:?}) dispatchability"
        );
        assert_eq!(
            apply_script(&r).is_ok(),
            dispatchable,
            "apply_script({rt:?}) dispatchability"
        );
        assert_eq!(
            state_query_script(&r).is_ok(),
            dispatchable,
            "state_query_script({rt:?}) dispatchability"
        );
    }
}
