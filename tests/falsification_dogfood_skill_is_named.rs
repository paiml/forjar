//! PMAT-163: the repo's dogfood skill must be named `forjar-dogfood`.
//!
//! # The defect this pins
//!
//! Claude Code resolves a skill by the `name:` in its frontmatter, and a
//! user-scope skill wins over a repo-scope one of the same name. The repo
//! shipped `.claude/skills/dogfood/SKILL.md` with `name: dogfood`, and
//! `~/.claude/skills/dogfood/` exists on the machine that wrote it. So the
//! release gate a contributor believed they were running was, silently, a
//! different document — no error, no warning, no diff.
//!
//! Shadowing is invisible by construction: both skills load, one is chosen, and
//! nothing prints which. The only defence is a NAME THAT CANNOT COLLIDE, and
//! the only way that stays true is a test that fails when the name reverts.
//!
//! # What is asserted
//!
//! 1. `.claude/skills/forjar-dogfood/SKILL.md` exists.
//! 2. Its frontmatter's FIRST line is exactly `name: forjar-dogfood` — first,
//!    because that is the line a rewrite drops first and the one a reader
//!    checks first.
//! 3. No skill in the tree is named `dogfood`, and the shadowable directory
//!    `.claude/skills/dogfood/` is gone. A rename that leaves the old copy
//!    behind has not removed the collision.
//! 4. Exactly one skill declares `name: forjar-dogfood`.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn skills_dir() -> PathBuf {
    repo_root().join(".claude/skills")
}

/// Every `SKILL.md` under `.claude/skills/`, one level deep.
fn skill_files() -> Vec<PathBuf> {
    let dir = skills_dir();
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("no {} — the repo skill tree is gone: {e}", dir.display()));
    for e in entries {
        let p = e.expect("dir entry").path();
        let skill = p.join("SKILL.md");
        if skill.is_file() {
            out.push(skill);
        }
    }
    out.sort();
    out
}

/// The lines between the opening `---` and the closing `---`.
fn frontmatter(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let mut lines = text.lines();
    let opener = lines.next().unwrap_or_default();
    assert_eq!(
        opener.trim(),
        "---",
        "{} does not open with a YAML frontmatter fence, so it declares no name at all",
        path.display()
    );
    lines
        .take_while(|l| l.trim() != "---")
        .map(str::to_string)
        .collect()
}

/// The declared `name:` of a skill, if it declares one.
fn declared_name(path: &Path) -> Option<String> {
    frontmatter(path).into_iter().find_map(|l| {
        l.strip_prefix("name:")
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    })
}

#[test]
fn the_forjar_dogfood_skill_exists() {
    let p = skills_dir().join("forjar-dogfood/SKILL.md");
    assert!(
        p.is_file(),
        "{} is missing — the release dogfood gate has no skill to run",
        p.display()
    );
}

#[test]
fn its_first_frontmatter_line_declares_the_unshadowable_name() {
    let p = skills_dir().join("forjar-dogfood/SKILL.md");
    let fm = frontmatter(&p);
    let first = fm
        .first()
        .unwrap_or_else(|| panic!("{} has an empty frontmatter", p.display()));
    assert_eq!(
        first.trim(),
        "name: forjar-dogfood",
        "the first frontmatter line of {} must be `name: forjar-dogfood`; \
         a skill with no name (or the old `dogfood`) is silently shadowed by \
         the user-scope skill of the same name and nothing prints which one ran",
        p.display()
    );
}

#[test]
fn nothing_in_the_tree_is_still_called_dogfood() {
    let shadowable = skills_dir().join("dogfood");
    assert!(
        !shadowable.exists(),
        "{} still exists — a rename that leaves the old copy behind has not \
         removed the collision it was meant to remove",
        shadowable.display()
    );
    for f in skill_files() {
        assert_ne!(
            declared_name(&f).as_deref(),
            Some("dogfood"),
            "{} declares the shadowable name `dogfood`",
            f.display()
        );
    }
}

#[test]
fn exactly_one_skill_claims_the_name() {
    let files = skill_files();
    assert!(
        !files.is_empty(),
        "no SKILL.md found under {} — every assertion here would be vacuous",
        skills_dir().display()
    );
    let claimants: Vec<_> = files
        .iter()
        .filter(|f| declared_name(f).as_deref() == Some("forjar-dogfood"))
        .collect();
    assert_eq!(
        claimants.len(),
        1,
        "expected exactly one skill named `forjar-dogfood`, found {}: {:?}",
        claimants.len(),
        claimants
    );
}
