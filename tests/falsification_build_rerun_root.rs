//! A build script must not ask cargo to watch the package root.
//!
//! `cargo::rerun-if-changed=<dir>` makes cargo scan `<dir>` recursively. The
//! vendored `verify_bindings` printed the grandparent of
//! `contracts/binding.yaml`, the empty path, which cargo reads as the package
//! root: `target/`, `.git/` and every file a gate writes. `dogfood-surface`
//! compiled the root crate three times a job for it (main run 45351530443:
//! 9m01s, 6m34s, 6m22s).

#[path = "../build_binding.rs"]
mod build_binding;

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The `output` file of this profile's newest forjar build-script run.
fn newest_build_output() -> PathBuf {
    let exe = std::env::current_exe().expect("test exe");
    let build = exe
        .parent()
        .and_then(Path::parent)
        .expect("<profile>/deps/<exe>")
        .join("build");
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("read {}: {e}", build.display()))
        .filter_map(Result::ok)
        .map(|e| e.path().join("output"))
        .filter(|p| {
            std::fs::read_to_string(p).is_ok_and(|t| t.contains("rustc-env=FORJAR_GIT_SHA="))
        })
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    found.sort();
    let (_, newest) = found
        .pop()
        .unwrap_or_else(|| panic!("no forjar build-script output under {}", build.display()));
    newest
}

#[test]
fn the_bug_shape_is_recognised() {
    let r = root();
    // What the helper printed for `contracts/binding.yaml`.
    let grandparent = Path::new("contracts/binding.yaml")
        .parent()
        .and_then(Path::parent)
        .expect("grandparent");
    assert_eq!(grandparent, Path::new(""));
    assert!(build_binding::watches_package_root("", &r));
    assert!(build_binding::watches_package_root(".", &r));
    assert!(build_binding::watches_package_root("..", &r));
    assert!(build_binding::watches_package_root(
        &r.to_string_lossy(),
        &r
    ));
    assert!(!build_binding::watches_package_root("contracts/", &r));
    assert!(!build_binding::watches_package_root(
        "contracts/binding.yaml",
        &r
    ));
}

#[test]
fn staged_binding_is_a_faithful_copy_whose_grandparent_holds_only_it() {
    let out = std::env::temp_dir().join(format!("fj-stage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let src = root().join("contracts/binding.yaml");
    let staged = build_binding::stage(&out, &src).expect("stage");
    assert_eq!(
        std::fs::read(&staged).unwrap(),
        std::fs::read(&src).unwrap()
    );
    let watched = staged.parent().and_then(Path::parent).expect("grandparent");
    assert!(watched.starts_with(&out) && watched != out);
    assert!(!build_binding::watches_package_root(
        &watched.to_string_lossy(),
        &root()
    ));
    // Cargo stamps a run with its START time: nothing staged may be newer
    // than the source, or the script re-runs itself on the next build.
    let src_mtime = std::fs::metadata(&src).unwrap().modified().unwrap();
    let mtime = |p: &Path| std::fs::metadata(p).unwrap().modified().unwrap();
    assert_eq!(mtime(&staged), src_mtime);
    build_binding::stage(&out, &src).expect("stage again");
    assert_eq!(mtime(&staged), src_mtime);
    #[cfg(unix)]
    {
        assert_eq!(mtime(staged.parent().unwrap()), src_mtime);
        assert_eq!(mtime(watched), src_mtime);
    }
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn the_build_script_watches_no_package_root_and_no_missing_path() {
    let r = root();
    let output = newest_build_output();
    let text = std::fs::read_to_string(&output).expect("read build output");
    let mut seen = 0usize;
    for line in text.lines() {
        let Some(path) = line
            .strip_prefix("cargo::rerun-if-changed=")
            .or_else(|| line.strip_prefix("cargo:rerun-if-changed="))
        else {
            continue;
        };
        seen += 1;
        assert!(
            !build_binding::watches_package_root(path, &r),
            "{}: `{line}` makes cargo scan the whole checkout",
            output.display()
        );
        assert!(
            r.join(path).exists(),
            "{}: `{line}` names a missing path, which cargo counts as changed on every build",
            output.display()
        );
    }
    assert!(
        seen >= 3,
        "{}: only {seen} rerun-if-changed line(s) read",
        output.display()
    );
}
