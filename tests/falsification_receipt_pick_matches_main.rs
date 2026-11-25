//! PMAT-1170 review round 4: `dogfood_receipt_pick` (Arm 5's choice of receipt
//! for a merged PR) must give, for every receipt WITHOUT a canary block at HEAD,
//! exactly the blob Arm 5 chose on main: the copy in the merge commit's tree,
//! else the copy at HEAD. Only a canary block at HEAD may change the answer.
//!
//! The reference below is main's selection, copied from
//! scripts/dogfood/release-check.sh at v1.34.1 (lines 378-384):
//!   blob=""; if git cat-file -e "${oid}:${path}"; then blob=<that>;
//!   elif git cat-file -e "HEAD:${path}"; then blob=<that>; fi
//! Round 7: a canary HEAD copy replaces the merge-commit copy only when main
//! refuses that one for zero refuted alone; a HEAD copy that is NUL-bearing or
//! invalid JSON is no canary receipt, and the merge-commit copy is judged.
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

const CANARY_STUB: &str = r#"{"quorum":{"lanes":["a","b","c"],"judges":3,"refuters_per_claim":3,"claims_refuted":0,"canary":{}},"evidence":{"files":[{}]}}"#;
const NUL_COPY: &str = "{\"quorum\":\u{0}}";
const BAD_JSON: &str = "{not json";

const OID_COPIES: [(&str, Option<&str>); 7] = [
    ("canary pre-merge", Some(CANARY_STUB)),
    ("absent", None),
    ("empty", Some("")),
    ("waived with canary null", Some(WAIVED_CANARY_NULL)),
    ("waived", Some(WAIVED)),
    ("old zero-refuted", Some(OLD_ZERO)),
    ("plain with a changed body", Some(CHANGED)),
];
const HEAD_COPIES: [(&str, Option<&str>); 7] = [
    ("canary receipt", Some(CANARY_STUB)),
    ("NUL byte", Some(NUL_COPY)),
    ("invalid JSON", Some(BAD_JSON)),
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
    match head_copy {
        Some(b) => fx::put(root, ".quorum/pr.json", b),
        None => {
            if oid_copy.is_some() {
                fx::git(root, &["rm", "-q", ".quorum/pr.json"]);
            }
        }
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

/// What the pick must give: main's choice, except that a HEAD copy with a canary
/// block replaces the merge-commit copy only when there is none or main refuses
/// that one for zero refuted alone (or it is a pre-merge canary receipt), and a
/// NUL-bearing HEAD copy with no merge-commit copy is a sentinel, not bytes.
fn expected(lib: &Path, on: &str, o: Option<&str>, hn: &str, h: Option<&str>) -> String {
    let main = pick(lib, o, h, "main");
    match (hn, on) {
        ("canary receipt", "absent" | "old zero-refuted" | "canary pre-merge") => {
            h.unwrap_or_default().to_string()
        }
        ("NUL byte", "absent") => "receipt holds a NUL byte".to_string(),
        _ => main,
    }
}

fn mismatches(lib: &Path) -> Vec<String> {
    let mut bad = Vec::new();
    for (on, o) in OID_COPIES {
        for (hn, h) in HEAD_COPIES {
            let want = expected(lib, on, o, hn, h);
            let got = pick(lib, o, h, "new");
            if want != got {
                bad.push(format!(
                    "merge-commit copy {on}, HEAD copy {hn}: expected {want:?}, now {got:?}"
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
const MUTANTS: [(&str, &str, &str); 6] = [
    (
        "a canary HEAD copy replaces any merge-commit copy",
        "            canary | \"quorum.claims_refuted = \"*) ;;",
        "            *) ;;",
    ),
    (
        "a NUL HEAD copy is refused over a valid merge-commit copy",
        "      if [ \"$have_blob\" -eq 1 ]; then\n        printf '%s' \"$blob\"\n      else\n        printf '%s' \"$later\"\n      fi\n      return 0",
        "      printf '%s' \"$later\"\n      return 0",
    ),
    (
        "an invalid-JSON HEAD copy is refused over a valid merge-commit copy",
        "      1 | 2 | 5)",
        "      1)",
    ),
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
        "  fi\n  if [ \"$have_blob\" -eq 1 ]; then\n    printf '%s' \"$blob\"",
        "  fi\n  if [ -n \"$blob\" ]; then\n    printf '%s' \"$blob\"",
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

/// Round 7: a receipt with no canary block is read with the tools main needs
/// (git, jq, cat, tr, wc), never grep -P or mktemp. `dogfood_git_blob` and the
/// pick run with a PATH that holds none of the others; a NUL is still refused
/// (rc 3), and a missing or silent tr or wc is "not judged" (rc 2), never ok.
fn blob_rc(copy: &str, drop: &[&str], silent: &[&str]) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    std::fs::create_dir_all(&root).expect("repo");
    fx::git(&root, &["init", "-q", "-b", "main"]);
    for kv in [
        ("core.hooksPath", "/dev/null"),
        ("commit.gpgsign", "false"),
        ("user.email", "t@example.com"),
        ("user.name", "t"),
    ] {
        fx::git(&root, &["config", kv.0, kv.1]);
    }
    fx::put(&root, "rec.json", copy);
    fx::git(&root, &["add", "-A"]);
    fx::git(&root, &["commit", "-qm", "r"]);
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    for t in [
        "git", "jq", "cat", "tr", "wc", "sh", "rm", "dirname", "env", "head", "sed",
    ] {
        if drop.contains(&t) {
            continue;
        }
        if silent.contains(&t) {
            let p = bin.join(t);
            std::fs::write(&p, "#!/bin/sh\nexit 0\n").expect("shim");
            let mut m = std::fs::metadata(&p).expect("meta").permissions();
            std::os::unix::fs::PermissionsExt::set_mode(&mut m, 0o755);
            std::fs::set_permissions(&p, m).expect("chmod");
            continue;
        }
        let src = ["/usr/bin", "/bin", "/usr/local/bin"]
            .iter()
            .map(|d| Path::new(d).join(t))
            .find(|p| p.exists());
        if let Some(src) = src {
            std::os::unix::fs::symlink(src, bin.join(t)).expect("link");
        }
    }
    let lib = fx::manifest().join("scripts/dogfood/lib/receipt.sh");
    let call = r#". "$1"; rc=0; out="$(dogfood_git_blob HEAD:rec.json)" || rc=$?; echo "rc=$rc len=${#out}""#;
    let out = Command::new("/bin/bash")
        .args(["-c", call, "x"])
        .arg(&lib)
        .env("PATH", &bin)
        .current_dir(&root)
        .output()
        .expect("bash");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn a_receipt_without_a_canary_block_needs_neither_grep_nor_mktemp() {
    let plain = STUB;
    let want = format!("rc=0 len={}", plain.len());
    assert_eq!(blob_rc(plain, &[], &[]), want);
    // grep and mktemp are simply not on the PATH (blob_rc never links them).
    assert_eq!(
        blob_rc("a\u{0}b", &[], &[]),
        "rc=3 len=0",
        "a NUL is refused"
    );
    assert_eq!(
        blob_rc(plain, &["tr"], &[]),
        "rc=2 len=0",
        "no tr: not judged"
    );
    assert_eq!(
        blob_rc(plain, &["wc"], &[]),
        "rc=2 len=0",
        "no wc: not judged"
    );
    assert_eq!(
        blob_rc(plain, &[], &["tr"]),
        "rc=2 len=0",
        "a silent tr: not judged"
    );
    assert_eq!(
        blob_rc(plain, &[], &["wc"]),
        "rc=2 len=0",
        "a silent wc: not judged"
    );
    assert_eq!(
        blob_rc("a\u{0}b", &[], &["tr"]),
        "rc=2 len=0",
        "a silent tr never passes a NUL"
    );
}
