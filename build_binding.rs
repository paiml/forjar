//! Where `build.rs` hands `contracts/binding.yaml` to the vendored
//! `provable_contracts::build_helper::verify_bindings`.
//!
//! That helper prints `cargo:rerun-if-changed=<grandparent of the path>`.
//! For `contracts/binding.yaml` the grandparent is the EMPTY path, which
//! cargo resolves to the package root and scans recursively: `target/`,
//! `.git/` and every file a test or a dogfood gate writes. Any of them newer
//! than the last build-script run re-ran the script and recompiled the root
//! crate. Measured on main run 45351530443: `Compiling forjar` three times in
//! one `dogfood-surface` job (9m01s, 6m34s, 6m22s) with no source change in
//! between; locally, one empty file created under `target/` cost a 4m59s
//! rebuild.
//!
//! The vendored crate stays byte-identical to its registry copy, so the fix
//! is on this side: the helper reads a copy two directories deep inside
//! `OUT_DIR`, whose grandparent holds nothing but that copy. The real file is
//! still watched by `build.rs` itself.

use std::path::{Path, PathBuf};

/// Copy `src` to `<out_dir>/binding-stage/forjar/binding.yaml` and return
/// that path. The copy is rewritten only when its content differs.
///
/// The copy and its two directories take the SOURCE file's mtime. Cargo
/// stamps a build-script run with the time the script STARTED and treats
/// anything newer as changed, so a file the script itself wrote would re-run
/// it once more on the next build (measured: one extra 5-minute compile).
/// The source is watched in its own right, so its mtime adds no staleness.
/// Setting a directory's mtime is best effort (it is not possible through
/// `std` on Windows).
pub fn stage(out_dir: &Path, src: &Path) -> std::io::Result<PathBuf> {
    let top = out_dir.join("binding-stage");
    let dir = top.join("forjar");
    std::fs::create_dir_all(&dir)?;
    let dst = dir.join("binding.yaml");
    let want = std::fs::read(src)?;
    if std::fs::read(&dst).ok().as_deref() != Some(want.as_slice()) {
        std::fs::write(&dst, &want)?;
    }
    let mtime = std::fs::metadata(src)?.modified()?;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&dst)?
        .set_modified(mtime)?;
    for d in [&dir, &top] {
        if let Ok(f) = std::fs::File::open(d) {
            let _ = f.set_modified(mtime);
        }
    }
    Ok(dst)
}

/// True when a `rerun-if-changed` path, resolved against the package root
/// the way cargo does, is the package root itself or one of its ancestors.
/// Such a path makes cargo scan the whole checkout.
pub fn watches_package_root(path: &str, root: &Path) -> bool {
    let p = root.join(path);
    let (Ok(p), Ok(root)) = (p.canonicalize(), root.canonicalize()) else {
        return false;
    };
    root.starts_with(&p)
}
