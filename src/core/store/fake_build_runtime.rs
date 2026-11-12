//! PMAT-989: a recording `BuildRuntime` for unit tests (no container involved).
//!
//! The tar it "exports" is built from the entries a test lists, so the capture
//! logic is exercised against real tar bytes.

use super::build_runtime::{BuildRuntime, DiffEntry, DiffKind, ExecOutcome};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

/// One entry of the fake export tar.
pub(crate) enum FakeNode {
    File(&'static str, Vec<u8>, u32),
    Dir(&'static str),
    Symlink(&'static str, &'static str),
    Hardlink(&'static str, &'static str),
    Fifo(&'static str),
}

/// Shared, recording fake. Clones see the same call log.
#[derive(Clone)]
pub(crate) struct FakeRuntime {
    pub calls: Rc<RefCell<Vec<String>>>,
    /// What the runtime reports for the base image; `Err` = image absent.
    pub image_ids: Result<Vec<String>, String>,
    /// A step whose script contains this text exits 1 with `boom` on stderr.
    pub fail_on: Option<&'static str>,
    pub diff: Vec<(char, &'static str)>,
    pub nodes: Rc<Vec<FakeNode>>,
}

impl FakeRuntime {
    pub(crate) fn new(image_ids: Vec<String>) -> Self {
        Self {
            calls: Rc::default(),
            image_ids: Ok(image_ids),
            fail_on: None,
            diff: vec![],
            nodes: Rc::default(),
        }
    }

    pub(crate) fn log(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }

    pub(crate) fn count(&self, prefix: &str) -> usize {
        self.calls
            .borrow()
            .iter()
            .filter(|c| c.starts_with(prefix))
            .count()
    }

    fn tar_bytes(&self) -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        for n in self.nodes.iter() {
            let mut h = tar::Header::new_gnu();
            h.set_uid(1000);
            h.set_gid(1000);
            h.set_mtime(1_700_000_000);
            match n {
                FakeNode::File(p, c, m) => {
                    h.set_size(c.len() as u64);
                    h.set_mode(*m);
                    h.set_entry_type(tar::EntryType::Regular);
                    b.append_data(&mut h, p, &c[..]).unwrap();
                }
                FakeNode::Dir(p) => {
                    h.set_size(0);
                    h.set_mode(0o755);
                    h.set_entry_type(tar::EntryType::Directory);
                    b.append_data(&mut h, p, std::io::empty()).unwrap();
                }
                FakeNode::Symlink(p, t) => {
                    h.set_size(0);
                    h.set_mode(0o777);
                    h.set_entry_type(tar::EntryType::Symlink);
                    b.append_link(&mut h, p, t).unwrap();
                }
                FakeNode::Hardlink(p, t) => {
                    h.set_size(0);
                    h.set_mode(0o755);
                    h.set_entry_type(tar::EntryType::Link);
                    b.append_link(&mut h, p, t).unwrap();
                }
                FakeNode::Fifo(p) => {
                    h.set_size(0);
                    h.set_mode(0o644);
                    h.set_entry_type(tar::EntryType::Fifo);
                    b.append_data(&mut h, p, std::io::empty()).unwrap();
                }
            }
        }
        b.into_inner().unwrap()
    }
}

impl BuildRuntime for FakeRuntime {
    fn name(&self) -> String {
        "fakecontainer".into()
    }
    fn image_diff_ids(&self, image: &str) -> Result<Vec<String>, String> {
        self.calls.borrow_mut().push(format!("inspect {image}"));
        self.image_ids.clone()
    }
    fn start(&self, image: &str) -> Result<String, String> {
        self.calls.borrow_mut().push(format!("start {image}"));
        Ok("cid1".into())
    }
    fn exec(&self, _id: &str, shell: &str, script: &str) -> Result<ExecOutcome, String> {
        self.calls.borrow_mut().push(format!("exec {shell}"));
        let code = i32::from(self.fail_on.is_some_and(|f| script.contains(f)));
        Ok(ExecOutcome {
            code,
            tail: if code == 0 {
                String::new()
            } else {
                "boom".into()
            },
        })
    }
    fn diff(&self, _id: &str) -> Result<Vec<DiffEntry>, String> {
        self.calls.borrow_mut().push("diff".into());
        Ok(self
            .diff
            .iter()
            .map(|(k, p)| DiffEntry {
                kind: match k {
                    'A' => DiffKind::Added,
                    'C' => DiffKind::Changed,
                    _ => DiffKind::Deleted,
                },
                path: (*p).to_string(),
            })
            .collect())
    }
    fn export(&self, _id: &str, dest: &Path) -> Result<(), String> {
        self.calls.borrow_mut().push("export".into());
        std::fs::write(dest, self.tar_bytes()).map_err(|e| e.to_string())
    }
    fn remove(&self, id: &str) {
        self.calls.borrow_mut().push(format!("remove {id}"));
    }
}
