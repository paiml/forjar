//! PMAT-989: executed layers, with a fake container runtime.

use super::exec_layer::{base_fetch_hint, build_exec_layer, ExecRequest, ExecStep};
use super::fake_build_runtime::{FakeNode, FakeRuntime};
use super::layer_builder::LayerEntry;
use std::path::Path;

fn ids() -> Vec<String> {
    vec!["sha256:aa".into(), "sha256:bb".into()]
}

fn steps() -> Vec<ExecStep> {
    vec![
        ExecStep {
            name: "packages".into(),
            shell: "bash".into(),
            script: "apt-get install -y jq".into(),
        },
        ExecStep {
            name: "command".into(),
            shell: "sh".into(),
            script: "echo hi > /etc/greeting".into(),
        },
    ]
}

fn run(rt: &FakeRuntime) -> Result<super::exec_capture::ExecLayer, String> {
    let ids = ids();
    let steps = steps();
    build_exec_layer(
        rt,
        &ExecRequest {
            base_ref: "debian:bookworm-slim",
            base_dir: Path::new("state/images/debian_bookworm-slim"),
            base_diff_ids: &ids,
            steps: &steps,
        },
    )
}

fn find<'a>(l: &'a super::exec_capture::ExecLayer, p: &str) -> Option<&'a LayerEntry> {
    l.entries.iter().find(|e| e.path.trim_end_matches('/') == p)
}

fn rich_runtime() -> FakeRuntime {
    let mut rt = FakeRuntime::new(ids());
    rt.diff = vec![
        ('A', "/usr/bin/jq"),
        ('C', "/etc/greeting"),
        ('A', "/usr/lib/jq"),
        ('A', "/usr/bin/jqlink"),
        ('C', "/etc"),
        ('D', "/etc/old.conf"),
        ('A', "/var/lib/apt/lists/deb_x"),
        ('A', "/var/log/apt/history.log"),
        ('A', "/tmp/scratch"),
        ('A', "/var/lib/dpkg/status-old"),
        ('C', "/var/lib/dpkg/status"),
    ];
    rt.nodes = std::rc::Rc::new(vec![
        FakeNode::File("./usr/bin/jq", b"ELF-jq".to_vec(), 0o755),
        FakeNode::File("./etc/greeting", b"hi\n".to_vec(), 0o644),
        FakeNode::Dir("./usr/lib/jq"),
        FakeNode::Symlink("./usr/bin/jqlink", "jq"),
        FakeNode::Dir("./etc"),
        FakeNode::File("./var/lib/apt/lists/deb_x", b"junk".to_vec(), 0o644),
        FakeNode::File("./var/log/apt/history.log", b"junk".to_vec(), 0o644),
        FakeNode::File("./tmp/scratch", b"junk".to_vec(), 0o644),
        FakeNode::File("./var/lib/dpkg/status-old", b"junk".to_vec(), 0o644),
        FakeNode::File("./var/lib/dpkg/status", b"Package: jq\n".to_vec(), 0o644),
    ]);
    rt
}

#[test]
fn captures_added_and_changed_files_symlinks_and_deletions() {
    let rt = rich_runtime();
    let l = run(&rt).unwrap();
    let jq = find(&l, "usr/bin/jq").expect("added file");
    assert_eq!(jq.content, b"ELF-jq");
    assert_eq!(jq.mode, 0o755);
    assert_eq!(find(&l, "etc/greeting").unwrap().content, b"hi\n");
    assert_eq!(
        find(&l, "usr/bin/jqlink").unwrap().link_target.as_deref(),
        Some("jq")
    );
    assert!(find(&l, "usr/lib/jq").unwrap().is_dir, "added directory");
    assert!(
        find(&l, "etc").is_none(),
        "a changed directory is not captured"
    );
    assert!(
        find(&l, "var/lib/dpkg/status").is_some(),
        "dpkg db is captured"
    );
    assert!(find(&l, "etc/.wh.old.conf").is_some(), "OCI whiteout for D");
    assert_eq!(l.whiteouts, 1);
    assert_eq!(l.files, l.entries.len() - 1);
}

#[test]
fn caches_logs_and_scratch_are_not_in_the_layer() {
    let l = run(&rich_runtime()).unwrap();
    for e in &l.entries {
        for bad in ["var/lib/apt/lists", "var/log", "tmp/", "status-old"] {
            assert!(!e.path.contains(bad), "{} leaked into the layer", e.path);
        }
    }
}

#[test]
fn steps_run_in_order_then_normalise_then_capture_and_the_container_is_removed() {
    let rt = rich_runtime();
    run(&rt).unwrap();
    assert_eq!(
        rt.log(),
        [
            "inspect debian:bookworm-slim",
            "start debian:bookworm-slim",
            "exec bash",
            "exec sh",
            "exec sh",
            "diff",
            "export",
            "remove cid1"
        ]
    );
}

#[test]
fn a_base_the_runtime_does_not_hold_is_an_error_with_the_fetch_commands() {
    let mut rt = FakeRuntime::new(vec!["sha256:aa".into(), "sha256:XX".into()]);
    let err = run(&rt).unwrap_err();
    assert!(err.contains("layer 1"), "{err}");
    assert!(
        err.contains("sha256:XX") && err.contains("sha256:bb"),
        "{err}"
    );
    assert!(err.contains("docker pull debian:bookworm-slim"), "{err}");
    assert!(
        err.contains("docker save debian:bookworm-slim | tar -x -C"),
        "{err}"
    );
    assert_eq!(rt.count("start"), 0, "nothing is started on a mismatch");

    rt.image_ids = Err("No such image".into());
    let err = run(&rt).unwrap_err();
    assert!(
        err.contains("never pulls implicitly") && err.contains("docker pull"),
        "{err}"
    );
    assert_eq!(rt.count("start"), 0);
}

#[test]
fn a_shorter_runtime_image_names_the_missing_layer() {
    let rt = FakeRuntime::new(vec!["sha256:aa".into()]);
    let err = run(&rt).unwrap_err();
    assert!(err.contains("layer 1") && err.contains("(none)"), "{err}");
}

#[test]
fn a_failing_step_is_an_error_naming_the_step_and_the_container_is_removed() {
    let mut rt = rich_runtime();
    rt.fail_on = Some("echo hi");
    let err = run(&rt).unwrap_err();
    assert!(
        err.contains("'command'") && err.contains("exited 1") && err.contains("boom"),
        "{err}"
    );
    assert_eq!(rt.count("remove"), 1, "container removed on failure");
    assert_eq!(rt.count("export"), 0, "nothing captured after a failure");
}

#[test]
fn a_changed_path_missing_from_the_export_fails_closed() {
    let mut rt = rich_runtime();
    rt.diff.push(('A', "/usr/bin/ghost"));
    let err = run(&rt).unwrap_err();
    assert!(
        err.contains("/usr/bin/ghost") && err.contains("not in its export"),
        "{err}"
    );
}

#[test]
fn a_fifo_in_the_changed_set_fails_closed() {
    let mut rt = FakeRuntime::new(ids());
    rt.diff = vec![('A', "/run2/pipe")];
    rt.nodes = std::rc::Rc::new(vec![FakeNode::Fifo("./run2/pipe")]);
    let err = run(&rt).unwrap_err();
    assert!(err.contains("run2/pipe"), "{err}");
}

#[test]
fn a_hardlink_is_captured_as_a_file_with_its_targets_content() {
    let mut rt = FakeRuntime::new(ids());
    rt.diff = vec![('A', "/usr/bin/b")];
    rt.nodes = std::rc::Rc::new(vec![
        FakeNode::File("./usr/bin/a", b"same".to_vec(), 0o755),
        FakeNode::Hardlink("./usr/bin/b", "./usr/bin/a"),
    ]);
    let l = run(&rt).unwrap();
    assert_eq!(find(&l, "usr/bin/b").unwrap().content, b"same");
}

#[test]
fn a_deleted_directory_gets_one_whiteout_not_one_per_child() {
    let mut rt = FakeRuntime::new(ids());
    rt.diff = vec![
        ('D', "/opt/app"),
        ('D', "/opt/app/bin"),
        ('D', "/opt/app/bin/x"),
    ];
    let l = run(&rt).unwrap();
    assert_eq!(l.whiteouts, 1);
    assert!(find(&l, "opt/.wh.app").is_some());
}

#[test]
fn the_same_inputs_give_the_same_entries() {
    let a = run(&rich_runtime()).unwrap();
    let b = run(&rich_runtime()).unwrap();
    let key = |l: &super::exec_capture::ExecLayer| {
        let mut v: Vec<_> = l
            .entries
            .iter()
            .map(|e| (e.path.clone(), e.content.clone(), e.mode))
            .collect();
        v.sort();
        v
    };
    assert_eq!(key(&a), key(&b));
}

#[test]
fn the_fetch_hint_has_both_skopeo_and_docker_save() {
    let h = base_fetch_hint("ubuntu:22.04", Path::new("d"));
    assert!(h.contains("skopeo copy docker://ubuntu:22.04 oci:d"));
    assert!(h.contains("docker save ubuntu:22.04 | tar -x -C d"));
}
