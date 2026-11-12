//! PMAT-989: turn "what the build steps changed in a container" into layer entries.
//!
//! Inputs are the runtime's `diff` (which paths changed) and `export` (the whole
//! root filesystem as a tar, where the bytes, modes and link targets are).
//! Nothing here starts a process.
//!
//! Known limit: `LayerEntry` carries no uid/gid, so every captured file lands
//! in the layer owned by root (0:0), whatever it was in the container. To stop
//! that from widening privilege, setuid/setgid bits are STRIPPED from every
//! entry the container did not own as root (uid or gid not 0); each is counted
//! in `ExecLayer::stripped` and reported.

use super::build_runtime::{DiffEntry, DiffKind};
use super::layer_builder::LayerEntry;
use super::overlay_export::whiteouts_to_entries;
use crate::core::types::WhiteoutEntry;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;
use std::path::Path;

/// Directories whose CONTENTS are removed after the steps (the directory stays)
/// and which are left out of the layer: caches, logs and scratch space whose
/// bytes depend on when and where the build ran.
pub const NORMALISED_DIRS: &[&str] = &[
    "/var/lib/apt/lists",
    "/var/cache/apt",
    "/var/log",
    "/tmp",
    "/var/tmp",
    "/run",
    "/root/.cache",
];
/// Single files removed after the steps and left out of the layer.
pub const NORMALISED_FILES: &[&str] = &["/var/cache/ldconfig/aux-cache"];
/// `(directory, name suffix)`: backup files (`*-old`) removed and left out.
pub const NORMALISED_SUFFIXED: &[(&str, &str)] =
    &[("/var/cache/debconf", "-old"), ("/var/lib/dpkg", "-old")];

/// The shell run in the container after the steps: removes every path of the
/// `NORMALISED_*` lists. One list, two uses: this script and `is_normalised`.
pub fn normalise_script() -> String {
    let mut s = String::from("set -e\n");
    for d in NORMALISED_DIRS {
        s.push_str(&format!(
            "if [ -d '{d}' ]; then find '{d}' -mindepth 1 -delete; fi\n"
        ));
    }
    for f in NORMALISED_FILES {
        s.push_str(&format!("rm -f '{f}'\n"));
    }
    for (d, suffix) in NORMALISED_SUFFIXED {
        s.push_str(&format!(
            "if [ -d '{d}' ]; then find '{d}' -maxdepth 1 -name '*{suffix}' -delete; fi\n"
        ));
    }
    s
}

/// Is the absolute path `abs` covered by the normalisation lists?
pub fn is_normalised(abs: &str) -> bool {
    NORMALISED_DIRS
        .iter()
        .any(|d| abs == *d || abs.strip_prefix(d).is_some_and(|r| r.starts_with('/')))
        || NORMALISED_FILES.contains(&abs)
        || NORMALISED_SUFFIXED.iter().any(|(dir, suffix)| {
            abs.strip_prefix(dir)
                .and_then(|r| r.strip_prefix('/'))
                .is_some_and(|name| !name.contains('/') && name.ends_with(suffix))
        })
}

/// What a capture produced.
#[derive(Debug)]
pub struct ExecLayer {
    /// Entries for the layer: added/changed files, symlinks, new directories, whiteouts.
    pub entries: Vec<LayerEntry>,
    /// Entries that are not whiteouts.
    pub files: usize,
    /// Whiteout markers.
    pub whiteouts: usize,
    /// Paths whose setuid/setgid bits were stripped (not owned by root in the container).
    pub stripped: Vec<String>,
}

/// The layer forces owner 0:0, so a setuid/setgid bit on a file the container
/// did not own as root would become a root-owned setuid binary. Strip the bits.
fn drop_unowned_setid(path: &str, mode: u32, h: &tar::Header, stripped: &mut Vec<String>) -> u32 {
    let root = h.uid().ok() == Some(0) && h.gid().ok() == Some(0);
    if root || mode & 0o6000 == 0 {
        return mode;
    }
    stripped.push(format!("/{path}"));
    mode & !0o6000
}

/// Archive path (`./etc/x`, `/etc/x/`) -> layer-relative path (`etc/x`).
fn rel(path: &str) -> String {
    path.trim_start_matches("./")
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_string()
}

/// Keep only the deleted paths whose ancestors are not deleted too: one
/// whiteout for a directory hides everything under it.
fn top_level_deletes(deleted: &BTreeSet<String>) -> Vec<String> {
    deleted
        .iter()
        .filter(|p| {
            let mut cur = p.as_str();
            while let Some((parent, _)) = cur.rsplit_once('/') {
                if deleted.contains(parent) {
                    return false;
                }
                cur = parent;
            }
            true
        })
        .cloned()
        .collect()
}

/// Sort the diff into wanted (added/changed) and deleted relative paths,
/// dropping everything the normalisation lists cover.
fn classify(diff: &[DiffEntry]) -> (BTreeMap<String, DiffKind>, BTreeSet<String>) {
    let mut want = BTreeMap::new();
    let mut deleted = BTreeSet::new();
    for d in diff {
        let path = rel(&d.path);
        if path.is_empty() || is_normalised(&format!("/{path}")) {
            continue;
        }
        if d.kind == DiffKind::Deleted {
            deleted.insert(path);
        } else {
            want.insert(path, d.kind.clone());
        }
    }
    deleted.retain(|p| !want.contains_key(p));
    (want, deleted)
}

fn open_tar(path: &Path) -> Result<tar::Archive<std::io::BufReader<std::fs::File>>, String> {
    let f =
        std::fs::File::open(path).map_err(|e| format!("open export {}: {e}", path.display()))?;
    Ok(tar::Archive::new(std::io::BufReader::new(f)))
}

/// Build the layer entries from the runtime's diff and the exported rootfs tar.
///
/// Fails closed: a changed path missing from the export, or an entry of a type
/// a layer cannot carry (device, fifo), is an error rather than a skip.
pub fn capture(diff: &[DiffEntry], export_tar: &Path) -> Result<ExecLayer, String> {
    let (want, deleted) = classify(diff);
    let mut entries: Vec<LayerEntry> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut links: Vec<(String, String, u32)> = Vec::new();
    let mut stripped: Vec<String> = Vec::new();

    let mut ar = open_tar(export_tar)?;
    for e in ar.entries().map_err(|e| format!("read export: {e}"))? {
        let mut e = e.map_err(|e| format!("read export entry: {e}"))?;
        let path = rel(&e
            .path()
            .map_err(|e| format!("export path: {e}"))?
            .to_string_lossy());
        let Some(kind) = want.get(&path) else {
            continue;
        };
        let mode = e
            .header()
            .mode()
            .map_err(|e| format!("mode of {path}: {e}"))?
            & 0o7777;
        let mode = drop_unowned_setid(&path, mode, e.header(), &mut stripped);
        use tar::EntryType as T;
        match e.header().entry_type() {
            T::Regular | T::Continuous => {
                let mut content = Vec::with_capacity(e.size() as usize);
                e.read_to_end(&mut content)
                    .map_err(|e| format!("read {path}: {e}"))?;
                entries.push(LayerEntry::file(&path, &content, mode));
            }
            T::Symlink => {
                let target = e
                    .link_name()
                    .map_err(|e| format!("link target of {path}: {e}"))?
                    .ok_or_else(|| format!("symlink {path} has no target"))?;
                entries.push(LayerEntry::symlink(&path, &target.to_string_lossy(), mode));
            }
            T::Link => {
                let target = e
                    .link_name()
                    .map_err(|e| format!("link target of {path}: {e}"))?
                    .ok_or_else(|| format!("hardlink {path} has no target"))?;
                links.push((path.clone(), rel(&target.to_string_lossy()), mode));
            }
            T::Directory => {
                if *kind == DiffKind::Added {
                    entries.push(LayerEntry::dir(&path, mode));
                }
            }
            other => {
                return Err(format!(
                    "{path} is a {other:?} entry; an image layer built from a container \
                     step carries only regular files, symlinks, hardlinks and directories"
                ));
            }
        }
        seen.insert(path);
    }
    resolve_hardlinks(&links, export_tar, &mut entries)?;

    let missing: Vec<&String> = want.keys().filter(|p| !seen.contains(*p)).collect();
    if !missing.is_empty() {
        return Err(format!(
            "the runtime reports {} changed path(s) that are not in its export, e.g. {}",
            missing.len(),
            missing
                .iter()
                .take(3)
                .map(|p| format!("/{p}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let whiteouts: Vec<WhiteoutEntry> = top_level_deletes(&deleted)
        .into_iter()
        .map(|path| WhiteoutEntry::FileDelete { path })
        .collect();
    let files = entries.len();
    entries.extend(whiteouts_to_entries(&whiteouts));
    Ok(ExecLayer {
        entries,
        files,
        whiteouts: whiteouts.len(),
        stripped,
    })
}

/// A hardlink is stored as a regular file with its target's content. The
/// target may be unchanged (so not in `entries`): read it from the export.
fn resolve_hardlinks(
    links: &[(String, String, u32)],
    export_tar: &Path,
    entries: &mut Vec<LayerEntry>,
) -> Result<(), String> {
    if links.is_empty() {
        return Ok(());
    }
    let mut contents: HashMap<String, Vec<u8>> = HashMap::new();
    let wanted: BTreeSet<&str> = links.iter().map(|(_, t, _)| t.as_str()).collect();
    let mut ar = open_tar(export_tar)?;
    for e in ar.entries().map_err(|e| format!("read export: {e}"))? {
        let mut e = e.map_err(|e| format!("read export entry: {e}"))?;
        let path = rel(&e
            .path()
            .map_err(|e| format!("export path: {e}"))?
            .to_string_lossy());
        if wanted.contains(path.as_str()) && e.header().entry_type().is_file() {
            let mut buf = Vec::new();
            e.read_to_end(&mut buf)
                .map_err(|e| format!("read {path}: {e}"))?;
            contents.insert(path, buf);
        }
    }
    for (path, target, mode) in links {
        let content = contents
            .get(target)
            .ok_or_else(|| format!("hardlink /{path} points at /{target}, not found in export"))?;
        entries.push(LayerEntry::file(path, content, *mode));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(uid: u64, gid: u64) -> tar::Header {
        let mut h = tar::Header::new_gnu();
        h.set_uid(uid);
        h.set_gid(gid);
        h
    }

    #[test]
    fn setid_bits_are_stripped_unless_the_entry_is_root_owned() {
        let mut out = Vec::new();
        assert_eq!(
            drop_unowned_setid("a", 0o6755, &hdr(1000, 1000), &mut out),
            0o755
        );
        assert_eq!(
            drop_unowned_setid("b", 0o4755, &hdr(0, 1000), &mut out),
            0o755
        );
        assert_eq!(
            drop_unowned_setid("c", 0o2755, &hdr(1000, 0), &mut out),
            0o755
        );
        assert_eq!(out, vec!["/a", "/b", "/c"]);
        assert_eq!(
            drop_unowned_setid("d", 0o4755, &hdr(0, 0), &mut out),
            0o4755
        );
        assert_eq!(
            drop_unowned_setid("e", 0o1755, &hdr(1000, 1000), &mut out),
            0o1755
        );
        assert_eq!(
            out.len(),
            3,
            "root-owned and sticky-only entries are untouched"
        );
    }

    #[test]
    fn normalised_paths_are_matched_by_prefix_file_and_suffix() {
        for p in [
            "/var/lib/apt/lists",
            "/var/lib/apt/lists/deb.debian.org_x",
            "/tmp/x",
            "/var/log/dpkg.log",
            "/var/cache/ldconfig/aux-cache",
            "/var/lib/dpkg/status-old",
            "/var/cache/debconf/config.dat-old",
            "/root/.cache/pip/x",
        ] {
            assert!(is_normalised(p), "{p}");
        }
        for p in [
            "/var/lib/dpkg/status",
            "/var/lib/dpkg/info/jq.list",
            "/var/lib/dpkg/info/x-old",
            "/tmpfile",
            "/usr/bin/jq",
            "/var/lib/apt/extended_states",
        ] {
            assert!(!is_normalised(p), "{p}");
        }
    }

    #[test]
    fn normalise_script_names_every_listed_path() {
        let s = normalise_script();
        for d in NORMALISED_DIRS.iter().chain(NORMALISED_FILES) {
            assert!(s.contains(d), "{d}");
        }
        for (d, suffix) in NORMALISED_SUFFIXED {
            assert!(s.contains(d) && s.contains(suffix));
        }
    }

    #[test]
    fn a_deleted_directory_hides_its_deleted_children() {
        let d: BTreeSet<String> = ["a", "a/b", "a/b/c", "x/y"].map(String::from).into();
        assert_eq!(top_level_deletes(&d), vec!["a".to_string(), "x/y".into()]);
    }
}
