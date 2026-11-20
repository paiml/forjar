//! PMAT-1170 review round 4: `dogfood_receipt_pick` (Arm 5's choice of receipt
//! for a merged PR) must give, for every receipt WITHOUT a canary block at HEAD,
//! exactly the blob Arm 5 chose on main: the copy in the merge commit's tree,
//! else the copy at HEAD. Only a canary block at HEAD may change the answer.
//!
//! The reference below is main's selection, copied from
//! scripts/dogfood/release-check.sh at v1.34.1 (lines 378-384):
//!   blob=""; if git cat-file -e "${oid}:${path}"; then blob=<that>;
//!   elif git cat-file -e "HEAD:${path}"; then blob=<that>; fi
//! The matrix is every combination of a merge-commit copy and a HEAD copy, so
//! the lane's three repros (waived + stub at HEAD, stub only, changed receipt at
//! HEAD) and its two edge cases (an empty copy, `"canary": null` beside a
//! waiver) are rows of it, not hand-picked.
#![cfg(unix)]

#[path = "canary_receipts_fixture/mod.rs"]
mod fx;

use std::path::Path;
use std::process::Command;

const REFERENCE: &str = r#"
main_pick() {
  local oid="$1" path="$2" blob=""
  if git cat-file -e "${oid}:${path}" 2>/dev/null; then
    blob="$(git cat-file -p "${oid}:${path}")"
  elif git cat-file -e "HEAD:${path}" 2>/dev/null; then
    blob="$(git cat-file -p "HEAD:${path}")"
  fi
  printf '%s' "$blob"
}
"#;

const WAIVED_CANARY_NULL: &str = r#"{"waived":{"reason":"emergency"},"quorum":{"lanes":["a","b","c"],"judges":3,"refuters_per_claim":3,"claims_refuted":0,"canary":null},"evidence":{"files":[{}]}}"#;
const WAIVED: &str = r#"{"waived":{"reason":"emergency"},"quorum":{"lanes":["a","b","c"],"judges":3,"refuters_per_claim":3,"claims_refuted":0},"evidence":{"files":[{}]}}"#;
const OLD_ZERO: &str = r#"{"quorum":{"lanes":["a","b","c"],"judges":3,"refuters_per_claim":3,"claims_refuted":0},"evidence":{"files":[{}]}}"#;
const STUB: &str = r#"{"quorum":{"lanes":["a","b","c"],"judges":3,"refuters_per_claim":3,"claims_refuted":1},"evidence":{"files":[{}]}}"#;
const CHANGED: &str = r#"{"quorum":{"lanes":["x","y","z"],"judges":3,"refuters_per_claim":3,"claims_refuted":2},"evidence":{"files":[{}]}}"#;

const OID_COPIES: [(&str, Option<&str>); 6] = [
    ("absent", None),
    ("empty", Some("")),
    ("waived with canary null", Some(WAIVED_CANARY_NULL)),
    ("waived", Some(WAIVED)),
    ("old zero-refuted", Some(OLD_ZERO)),
    ("plain with a changed body", Some(CHANGED)),
];
const HEAD_COPIES: [(&str, Option<&str>); 4] = [
    ("absent", None),
    ("empty", Some("")),
    ("unbound stub", Some(STUB)),
    ("changed receipt", Some(CHANGED)),
];

fn pick(lib: &Path, oid_copy: Option<&str>, head_copy: Option<&str>, which: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fx::git(root, &["init", "-q", "-b", "main"]);
    for kv in [
        ("core.hooksPath", "/dev/null"),
        ("commit.gpgsign", "false"),
        ("user.email", "t@example.com"),
        ("user.name", "t"),
    ] {
        fx::git(root, &["config", kv.0, kv.1]);
    }
    fx::put(root, "README.md", "x\n");
    if let Some(b) = oid_copy {
        fx::put(root, ".quorum/pr.json", b);
    }
    fx::git(root, &["add", "-A"]);
    fx::git(root, &["commit", "-qm", "merge commit"]);
    let oid = fx::git(root, &["rev-parse", "HEAD"]);
    if let Some(b) = head_copy {
        fx::put(root, ".quorum/pr.json", b);
    }
    fx::put(root, "later.txt", "y\n");
    fx::git(root, &["add", "-A"]);
    fx::git(root, &["commit", "-qm", "later"]);
    let call = format!(
        r#". "$1"; {REFERENCE} if [ "$2" = main ]; then main_pick "$3" .quorum/pr.json; else dogfood_receipt_pick "$3" .quorum/pr.json; fi"#
    );
    let out = Command::new("bash")
        .args(["-c", &call, "x"])
        .arg(lib)
        .arg(which)
        .arg(&oid)
        .current_dir(root)
        .output()
        .expect("bash");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn mismatches(lib: &Path) -> Vec<String> {
    let mut bad = Vec::new();
    for (on, o) in OID_COPIES {
        for (hn, h) in HEAD_COPIES {
            let want = pick(lib, o, h, "main");
            let got = pick(lib, o, h, "new");
            if want != got {
                bad.push(format!(
                    "merge-commit copy {on}, HEAD copy {hn}: main chose {want:?}, now {got:?}"
                ));
            }
        }
    }
    bad
}

#[test]
fn without_a_canary_block_at_head_the_pick_is_what_main_chose() {
    let lib = fx::manifest().join("scripts/dogfood/lib/receipt.sh");
    let bad = mismatches(&lib);
    eprintln!("pick parity rows {}", OID_COPIES.len() * HEAD_COPIES.len());
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// (name, text in lib/receipt.sh, its mutation). Each must break the parity.
const MUTANTS: [(&str, &str, &str); 3] = [
    (
        "decide on the merge-commit copy, as the first version did",
        "printf '%s' \"$later\" | jq -e '(.quorum | type == \"object\") and (.quorum | has(\"canary\"))'",
        "printf '%s' \"$blob\" | jq -e '(.quorum | type == \"object\") and (.quorum | has(\"canary\"))'",
    ),
    (
        "take any non-empty HEAD copy",
        "printf '%s' \"$later\" | jq -e '(.quorum | type == \"object\") and (.quorum | has(\"canary\"))'",
        "printf '%s' \"$later\" | jq -e 'true'",
    ),
    (
        "fall through to HEAD when the merge-commit copy is empty",
        "  if [ \"$have_blob\" -eq 1 ]; then",
        "  if [ -n \"$blob\" ]; then",
    ),
];

#[test]
fn every_mutant_of_the_pick_breaks_the_parity() {
    eprintln!("pick mutants {}", MUTANTS.len());
    let real = fx::manifest().join("scripts/dogfood/lib/receipt.sh");
    let src = std::fs::read_to_string(&real).expect("lib");
    for (name, from, to) in MUTANTS {
        assert_eq!(src.matches(from).count(), 1, "'{name}' must match once");
        let dir = tempfile::tempdir().expect("tempdir");
        let lib = dir.path().join("receipt.sh");
        std::fs::write(&lib, src.replace(from, to)).expect("write");
        assert!(
            !mismatches(&lib).is_empty(),
            "mutant '{name}' survived: the pick still equals main on every row"
        );
    }
}
