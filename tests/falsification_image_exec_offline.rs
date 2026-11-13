//! PMAT-989 (IMG-001 3/N) falsifier that runs WITHOUT a container runtime, so the
//! quorum gate's `cargo test --test` executes real assertions (the live F1/F2 in
//! `falsification_image_exec.rs` need `--features container-test`).
//!
//! F3: an executed layer is owned by 0:0, so a setuid/setgid bit on an entry the
//! container did not own as root must not survive capture.

use forjar::core::store::build_runtime::{DiffEntry, DiffKind};
use forjar::core::store::exec_capture::capture;

fn export_tar(path: &std::path::Path, files: &[(&str, u32, u64)]) {
    let mut b = tar::Builder::new(Vec::new());
    for (name, mode, uid) in files {
        let mut h = tar::Header::new_gnu();
        h.set_size(1);
        h.set_mode(*mode);
        h.set_uid(*uid);
        h.set_gid(*uid);
        h.set_entry_type(tar::EntryType::Regular);
        b.append_data(&mut h, name, &b"x"[..]).unwrap();
    }
    std::fs::write(path, b.into_inner().unwrap()).unwrap();
}

#[test]
fn f3_setid_bits_of_non_root_entries_are_stripped_and_root_entries_kept() {
    let dir = tempfile::tempdir().unwrap();
    let tar = dir.path().join("export.tar");
    export_tar(
        &tar,
        &[
            ("usr/bin/user-su", 0o6755, 1000),
            ("usr/bin/root-su", 0o4755, 0),
        ],
    );
    let diff: Vec<DiffEntry> = ["/usr/bin/user-su", "/usr/bin/root-su"]
        .iter()
        .map(|p| DiffEntry {
            kind: DiffKind::Added,
            path: (*p).into(),
        })
        .collect();
    let layer = capture(&diff, &tar).unwrap();
    let mode = |p: &str| layer.entries.iter().find(|e| e.path == p).unwrap().mode;
    assert_eq!(
        mode("usr/bin/user-su"),
        0o755,
        "setuid/setgid must be stripped"
    );
    assert_eq!(mode("usr/bin/root-su"), 0o4755, "root-owned setuid is kept");
    assert_eq!(layer.stripped, vec!["/usr/bin/user-su".to_string()]);
}
