//! README.md is a graded document; these hold its grade to its inputs (forjar#1022, PMAT-1018).
//!
//! `pv-readme --check` grades README.md against `contracts/readme-root.yaml`
//! and the COMMITTED `contracts/census.json`. The nightly LAB job
//! (`readme-shape-lab.yml`) runs it with a pinned verifier. These tests need
//! no pv and run on every `cargo test`, so three ways the grade could go green
//! while it is false are caught here:
//!
//!   1. `readme:contractCount` without `minCount: 1`: delete `contract_count:`
//!      from the frontmatter and RDM-INV-002 holds vacuously;
//!   2. a stale census: a contract added to `contracts/` without regenerating
//!      `census.json` leaves pv-readme comparing the README to an old number;
//!   3. the README count drifting from the census.
//!
//! The file rule below is pv's own (`collect_yaml_files` in
//! aprender-contracts): every `*.yaml` under the directory, recursively, except
//! `binding.yaml` and the `kaizen`, `legacy` and `pipelines` subdirectories.

use std::fs;
use std::path::{Path, PathBuf};

use serde_yaml_ng::Value;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    fs::read_to_string(repo().join(rel)).unwrap_or_else(|e| panic!("{rel} must be readable: {e}"))
}

/// Contract files under `dir`, by pv's census file rule.
fn contract_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{} must list: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if !matches!(name, "kaizen" | "legacy" | "pipelines") {
                contract_files(&path, out);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("yaml")
            && name != "binding.yaml"
        {
            out.push(path);
        }
    }
}

fn count_contracts(dir: &Path) -> u64 {
    let mut files = Vec::new();
    contract_files(dir, &mut files);
    files.len() as u64
}

fn census_n_files(census_json: &str) -> u64 {
    let v: serde_json::Value = serde_json::from_str(census_json).expect("census.json must parse");
    v.get("n_files")
        .and_then(serde_json::Value::as_u64)
        .expect("census.json must carry n_files")
}

/// The README's YAML frontmatter, the block between the first two `---` lines.
fn frontmatter(readme: &str) -> Option<Value> {
    let rest = readme.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    serde_yaml_ng::from_str(&rest[..end]).ok()
}

/// The readme-root shape's property rows, keyed by `path`.
fn shape_property<'a>(contract: &'a Value, path: &str) -> Option<&'a Value> {
    contract
        .get("shape")?
        .get("properties")?
        .as_sequence()?
        .iter()
        .find(|p| p.get("path").and_then(Value::as_str) == Some(path))
}

/// The frontmatter key a shape path grades must be required, or its invariant
/// holds over an absent value. RDM-INV-002 is the count; it is `minCount: 1`.
#[test]
fn the_contract_count_is_required_by_the_shape() {
    let contract: Value =
        serde_yaml_ng::from_str(&read("contracts/readme-root.yaml")).expect("contract must parse");
    let row = shape_property(&contract, "readme:contractCount")
        .expect("readme-root.yaml's shape must grade readme:contractCount");
    let min = row.get("minCount").and_then(Value::as_u64).unwrap_or(0);
    assert!(
        min >= 1,
        "readme:contractCount has minCount {min}: deleting `contract_count:` from README.md \
         makes RDM-INV-002 hold vacuously and pv-readme exit 0 (agy lane 2, PR #1023)"
    );
    // The rule is structural, not one row's footnote: every row whose
    // invariant compares a README fact to something else must be required.
    for path in [
        "readme:schemaVersion",
        "readme:kind",
        "readme:entrypoint",
        "readme:verifiedCommand",
    ] {
        let row =
            shape_property(&contract, path).unwrap_or_else(|| panic!("shape must grade {path}"));
        let min = row.get("minCount").and_then(Value::as_u64).unwrap_or(0);
        assert!(
            min >= 1,
            "{path} has minCount {min}; the shape must require it"
        );
    }
}

/// The committed census counts the contracts that are actually in contracts/.
#[test]
fn the_committed_census_counts_contracts_dir() {
    let live = count_contracts(&repo().join("contracts"));
    let committed = census_n_files(&read("contracts/census.json"));
    assert_eq!(
        committed, live,
        "contracts/census.json says n_files {committed} but contracts/ holds {live} contract \
         file(s); pv-readme grades README.md against the committed number, so regenerate it: \
         pv census --format json contracts > contracts/census.json"
    );
}

/// The comparison above can fail: one contract more, census not regenerated.
#[test]
fn a_contract_added_without_a_new_census_is_caught() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("contracts");
    fs::create_dir(&dir).expect("mkdir");
    let mut files = Vec::new();
    contract_files(&repo().join("contracts"), &mut files);
    for f in &files {
        fs::copy(f, dir.join(f.file_name().expect("file name"))).expect("copy contract");
    }
    fs::write(dir.join("binding.yaml"), "crates: []\nbindings: []\n").expect("write sidecar");
    let committed = census_n_files(&read("contracts/census.json"));
    assert_eq!(
        count_contracts(&dir),
        committed,
        "a copy plus a sidecar must count the same"
    );

    fs::write(
        dir.join("readme-root-falsifier-extra.yaml"),
        read("contracts/readme-root.yaml"),
    )
    .expect("plant extra contract");
    assert_ne!(
        count_contracts(&dir),
        committed,
        "one contract file more left the count unchanged: the census check is vacuous"
    );
}

/// The count README.md prints is the census, in its frontmatter and its body.
#[test]
fn the_readme_count_is_the_census() {
    let readme = read("README.md");
    let committed = census_n_files(&read("contracts/census.json"));
    let fm = frontmatter(&readme).expect("README.md must open with YAML frontmatter");
    let stated = fm
        .get("contract_count")
        .and_then(Value::as_u64)
        .expect("README.md frontmatter must state contract_count");
    assert_eq!(
        stated, committed,
        "README.md states {stated} contracts, census {committed}"
    );

    let body = readme
        .split("<!-- CONTRACT_COUNT_START -->")
        .nth(1)
        .and_then(|s| s.split("<!-- CONTRACT_COUNT_END -->").next())
        .expect("README.md must carry the CONTRACT_COUNT markers");
    assert_eq!(
        body.trim(),
        committed.to_string(),
        "README.md body count is not the census"
    );
}

/// The planted README without `contract_count:` has no count to grade: the
/// frontmatter parser above sees it absent, which is what minCount refuses.
#[test]
fn a_readme_without_its_count_states_none() {
    let readme = read("README.md");
    let planted: String = readme
        .lines()
        .filter(|l| !l.starts_with("contract_count:"))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_ne!(
        planted, readme,
        "the contract_count removal changed nothing"
    );
    let fm = frontmatter(&planted).expect("the planted README keeps its frontmatter");
    assert!(
        fm.get("contract_count").is_none(),
        "contract_count survived its removal"
    );
}

/// The LAB job re-takes the census rather than trusting the committed one, and
/// the shell falsifier plants the defects this file names.
#[test]
fn the_lab_job_and_falsifier_cover_the_census_and_the_count() {
    let wf: Value = serde_yaml_ng::from_str(&read(".github/workflows/readme-shape-lab.yml"))
        .expect("workflow must parse");
    let runs: Vec<String> = wf["jobs"]["readme-shape"]["steps"]
        .as_sequence()
        .expect("readme-shape must have steps")
        .iter()
        .filter_map(|s| s.get("run").and_then(Value::as_str).map(str::to_owned))
        .collect();
    assert!(
        runs.iter()
            .any(|r| r.contains("pv census --format json contracts")
                && r.contains("contracts/census.json")),
        "readme-shape-lab.yml must compare a live `pv census` with contracts/census.json"
    );
    assert!(
        runs.iter().any(|r| {
            let t: Vec<&str> = r.split_whitespace().collect();
            t.windows(2).any(|w| w == ["--bin", "pv-readme"])
                && t.windows(2).any(|w| w == ["--bin", "pv"])
        }),
        "readme-shape-lab.yml must install the pinned pv beside pv-readme"
    );

    let sh = read("tests/readme-shape-falsifier.sh");
    for needle in [
        "grep -v '^contract_count:' README.md",
        "census_matches \"contracts/census.json is the live census\" same contracts",
        "readme-root-falsifier-extra.yaml",
    ] {
        assert!(
            sh.contains(needle),
            "tests/readme-shape-falsifier.sh lost the plant `{needle}`"
        );
    }
}
