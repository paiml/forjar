//! GH-298: the contract layer counted declarations it never resolved.
//!
//! `contracts/nas-archive-v1.yaml` governs a data-DELETING resource and two of
//! its falsifiers named functions that did not exist. `pv audit` printed
//! "Falsification tests: 14 / No audit findings" and exited 0: it counts the
//! declarations in the YAML and never resolves one against the source.
//! `build.rs` printed "N/N bindings bound" while never opening a contract
//! file. No CI job ran `pv validate` at all.
//!
//! The first fix repaired the contracts the audit had named by hand and added
//! a CI resolver shaped around those same cases, so the defect survived one
//! level up, in the guard:
//!
//!     re.fullmatch(r"([\w./-]+\.rs)::(\w+)", cite.strip())
//!
//! The corpus writes citations in four shapes — `path.rs`, `path.rs::fn`,
//! `path.rs::mod::fn` and `path.rs mod::fn` — and `fullmatch` accepts one.
//! The rest fell through a `continue` and were counted "not resolvable" rather
//! than "not resolved", so the job printed "every resolvable falsifier
//! citation resolves" over a contract with seven dangling ones. It also
//! grepped `src tests benches` GLOBALLY, so all 28 wrong-FILE citations
//! passed, and it read only `falsification_tests[].test`, so 104
//! `enforced_by` and 12 `discharged_by` citations were never resolved at all.
//!
//! This file is the invariant, in Rust: every citation resolves to the exact
//! item it names, in the file it names.
//!
//! DELIBERATE BOUNDARY — READ BEFORE WIDENING. Only the keys in
//! `CITATION_KEYS` are resolved, and that is a decision. Contracts also carry
//! free prose in `description:`, `notes:`, `if_fails:` and
//! `qa_gate.falsification:` which mentions `.rs` files constantly —
//! destroy-undo-roundtrip-v1.yaml's falsification note names
//! `apply.rs::maybe_auto_snapshot` inside an English sentence about how to
//! break the property. A resolver walking every string would light up dozens
//! of such mentions, land red on arrival, and get weakened back into the
//! vacuous pass it replaces. The named keys are the ones whose VALUE IS a
//! citation; prose is out of scope by design.
//!
//! Implemented against the YAML directly rather than by shelling out to `pv`,
//! for the reason `falsification_contracts_parse.rs` gives: a test that needs
//! an external tool installed is a test that silently stops running when the
//! tool is missing, which is how the corpus went unresolved in the first
//! place.

use std::path::{Path, PathBuf};

/// Files under `contracts/` that are deliberately NOT contracts.
///
/// `binding.yaml` is a binding REGISTRY, not a contract; it gets its own
/// resolution test at the bottom of this file.
const NOT_CONTRACTS: &[&str] = &["binding.yaml"];

/// The keys whose value IS a citation. See the boundary note in the header.
const CITATION_KEYS: &[&str] = &[
    "falsification_tests[].test",
    "proof_obligations[].enforced_by",
    "proof_obligations[].discharged_by",
];

/// A resolved reference into the source tree: a repo-relative `.rs` path and,
/// optionally, the item inside it. A trailing `*` on the item is a prefix.
#[derive(Debug)]
struct Citation {
    file: String,
    item: Option<String>,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn contracts_dir() -> PathBuf {
    repo_root().join("contracts")
}

fn contract_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(contracts_dir())
        .expect("contracts/ must exist")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .filter(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            !NOT_CONTRACTS.contains(&name.as_ref())
        })
        .collect();
    files.sort();
    files
}

fn is_path_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-')
}

/// The `.rs` path ending at `end` (exclusive), walking back over path chars.
fn path_ending_at(s: &str, end: usize) -> Option<String> {
    let head = &s[..end];
    let start = head
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_path_char(*c))
        .map(|(i, _)| i)
        .last()?;
    let file = &s[start..end];
    // ".rs" alone, or a token whose stem is empty, is not a path.
    (file.len() > 3).then(|| file.to_string())
}

/// The item named immediately after a `.rs`, in either the `::item` form or
/// the space-separated `mod::item` form the corpus also uses.
fn item_after(rest: &str) -> Option<String> {
    let tok = if let Some(after) = rest.strip_prefix("::") {
        after.split([' ', '\t', '\n', ',', ';', ')', '"']).next()?
    } else if rest.starts_with([' ', '\t', '\n']) {
        let next = rest
            .trim_start()
            .split([' ', '\t', '\n', ',', ';', ')', '"'])
            .next()?;
        if !next.contains("::") {
            return None;
        }
        next
    } else {
        return None;
    };
    let seg = tok.rsplit("::").next()?.trim_end_matches('.');
    let stem = seg.strip_suffix('*').unwrap_or(seg);
    let ok = !stem.is_empty()
        && stem.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    ok.then(|| seg.to_string())
}

/// Every citation in `s`. Anchored on `.rs`, so a string with no `.rs` token
/// yields nothing — which is how shell snippets and equation names are
/// skipped without an exemption list.
fn citations_in(s: &str) -> Vec<Citation> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = s[from..].find(".rs") {
        let at = from + rel;
        let end = at + 3;
        from = end;
        // ".rs" must end the token: `x.rsomething` is not a path.
        if s[end..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let Some(file) = path_ending_at(s, end) else {
            continue;
        };
        out.push(Citation {
            item: item_after(&s[end..]),
            file,
        });
    }
    out
}

/// Pull the string at `key` out of a mapping, if it is a string.
fn string_at(node: &serde_yaml_ng::Value, key: &str) -> Option<String> {
    node.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn sequence<'a>(doc: &'a serde_yaml_ng::Value, key: &str) -> &'a [serde_yaml_ng::Value] {
    doc.get(key)
        .and_then(|v| v.as_sequence())
        .map_or(&[], |v| v)
}

/// (contract file name, yaml trail, citation) for every citation in the corpus.
fn collect_citations() -> Vec<(String, String, Citation)> {
    let mut out = Vec::new();
    for path in contract_files() {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let text = std::fs::read_to_string(&path).expect("readable contract");
        let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text)
            .unwrap_or_else(|e| panic!("{name} does not parse as YAML: {e}"));

        for (i, ft) in sequence(&doc, "falsification_tests").iter().enumerate() {
            let id = string_at(ft, "id").unwrap_or_else(|| i.to_string());
            let Some(raw) = string_at(ft, "test") else {
                continue;
            };
            for c in citations_in(&raw) {
                out.push((name.clone(), format!("falsification_tests[{id}].test"), c));
            }
        }
        for (i, po) in sequence(&doc, "proof_obligations").iter().enumerate() {
            for key in ["enforced_by", "discharged_by"] {
                let Some(raw) = string_at(po, key) else {
                    continue;
                };
                for c in citations_in(&raw) {
                    out.push((name.clone(), format!("proof_obligations[{i}].{key}"), c));
                }
            }
        }
    }
    out
}

/// Every `.rs` file under `root`, recursively.
fn rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == ".pmat") {
                    continue;
                }
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out
}

/// Which files DO define `fn <item>(` — the actionable half of a failure
/// message. A citation is wrong far more often than the code is missing.
fn homes_of(item: &str) -> Vec<String> {
    let root = repo_root();
    let needle = format!("fn {item}(");
    let mut out = Vec::new();
    for dir in ["src", "tests", "benches"] {
        for p in rs_files(&root.join(dir)) {
            let Ok(text) = std::fs::read_to_string(&p) else {
                continue;
            };
            if text.contains(&needle) {
                let rel = p.strip_prefix(&root).unwrap_or(&p);
                out.push(rel.display().to_string());
            }
        }
    }
    out.sort();
    out
}

#[test]
fn there_are_citations_to_resolve() {
    // Guards the guard, and it is the load-bearing test in this file. The
    // defect being fixed was a resolver that measured 77 of 211 citations and
    // reported success. A future narrowing of `citations_in` — the exact
    // regression that produced GH-298 — turns THIS red instead of turning the
    // rest of the suite silently green.
    let n = collect_citations().len();
    assert!(
        n >= 200,
        "only {n} citations were extracted from {} contracts; there were 211 \
         when this was written. `citations_in` has been narrowed and the tests \
         below are now passing by not looking — which is the defect this file \
         exists to prevent, not a green build. If citations were genuinely \
         REMOVED from the corpus, lower this number deliberately and say why",
        contract_files().len()
    );
}

#[test]
fn the_parser_reads_every_citation_shape_the_corpus_uses() {
    // The count above is a weak guard on its own: once the corpus is repaired
    // most citations are the easy `path.rs::fn` form, so the exact narrowing
    // that caused GH-298 — accepting only that form — barely moves the number.
    // This pins the GRAMMAR instead, case by case, so a resolver that stops
    // understanding a shape fails here whatever the corpus happens to contain.
    type Shape<'a> = (&'a str, Option<&'a str>);
    let cases: &[(&str, &[Shape])] = &[
        // Bare file: the old regex demanded `::fn`, so it never checked that
        // a cited FILE exists at all.
        ("tests/e.rs", &[("tests/e.rs", None)]),
        // The one shape the old regex did accept.
        ("src/a.rs::foo", &[("src/a.rs", Some("foo"))]),
        // Module path in the middle; `fullmatch` rejected these outright.
        ("src/a.rs::tests::foo", &[("src/a.rs", Some("foo"))]),
        // Space-separated module path — verb-surface-v1.yaml wrote two.
        ("src/a.rs tests::foo", &[("src/a.rs", Some("foo"))]),
        // Trailing prose must not swallow the item...
        ("src/a.rs::foo (and why)", &[("src/a.rs", Some("foo"))]),
        // ...but prose after a BARE file is not an item: `determinism` is a
        // word, and calling it a function name invents a phantom.
        ("src/a.rs determinism tests", &[("src/a.rs", None)]),
        // Compound: both halves resolve, not just the first.
        (
            "src/a.rs::foo; src/b.rs::bar",
            &[("src/a.rs", Some("foo")), ("src/b.rs", Some("bar"))],
        ),
        // Globs resolve as a prefix, never skipped.
        ("tests/c.rs::d_*", &[("tests/c.rs", Some("d_*"))]),
        // No `.rs` token: an equation name or a shell snippet.
        ("cargo test --lib backup_sync", &[]),
        ("receipt_completeness", &[]),
    ];

    for (input, expected) in cases {
        let got: Vec<(String, Option<String>)> = citations_in(input)
            .into_iter()
            .map(|c| (c.file, c.item))
            .collect();
        let want: Vec<(String, Option<String>)> = expected
            .iter()
            .map(|(f, i)| (f.to_string(), i.map(str::to_string)))
            .collect();
        assert_eq!(got, want, "citations_in({input:?})");
    }
}

#[test]
fn every_cited_file_exists() {
    let root = repo_root();
    let mut offenders = Vec::new();
    for (contract, trail, c) in collect_citations() {
        if root.join(&c.file).is_file() {
            continue;
        }
        let hint = rs_files(&root.join("src"))
            .into_iter()
            .chain(rs_files(&root.join("tests")))
            .filter_map(|p| p.strip_prefix(&root).ok().map(|r| r.display().to_string()))
            .find(|r| r.ends_with(&format!("/{}", c.file)));
        let hint = hint.map_or(String::new(), |h| format!(" (did you mean {h}?)"));
        offenders.push(format!(
            "{contract} {trail} cites `{}` — no such file{hint}",
            c.file
        ));
    }
    assert!(
        offenders.is_empty(),
        "contracts cite files that do not exist. A citation is the audit \
         trail from a claimed property to its evidence; one that does not \
         land reads as enforcement and is not:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn every_cited_item_is_in_the_file_that_is_cited() {
    // This is what defeats the global grep. The CI resolver this replaces
    // searched all of src/, tests/ and benches/ for the function name, so
    // every one of the 28 wrong-FILE citations in the corpus passed it.
    let root = repo_root();
    let mut offenders = Vec::new();
    for (contract, trail, c) in collect_citations() {
        let Some(item) = c.item.as_deref() else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(root.join(&c.file)) else {
            continue; // absent files are `every_cited_file_exists`'s finding
        };
        let found = match item.strip_suffix('*') {
            Some(prefix) => text.contains(&format!("fn {prefix}")),
            None => text.contains(&format!("fn {item}(")),
        };
        if found {
            continue;
        }
        let homes = homes_of(item.trim_end_matches('*'));
        let where_ = if homes.is_empty() {
            "it exists nowhere in src/, tests/ or benches/".to_string()
        } else {
            format!("it lives in {}", homes.join(", "))
        };
        offenders.push(format!(
            "{contract} {trail} cites `{}::{item}` — {where_}",
            c.file
        ));
    }
    assert!(
        offenders.is_empty(),
        "contracts cite code that is not where they say it is:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn every_qa_gate_check_names_a_real_test_target() {
    // `qa_gate.check` is the command a human runs to decide whether the gate
    // passes. verb-surface-v1.yaml named three `--test` targets that had never
    // existed, so the documented way to check that contract was `error: no
    // test target named ...`. Nothing in the repo looked at qa_gate at all.
    let root = repo_root();
    let mut offenders = Vec::new();
    for path in contract_files() {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let text = std::fs::read_to_string(&path).expect("readable contract");
        let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("parses");
        let Some(check) = doc.get("qa_gate").and_then(|g| string_at(g, "check")) else {
            continue;
        };
        let mut words = check.split_whitespace();
        while let Some(w) = words.next() {
            if w != "--test" {
                continue;
            }
            let Some(target) = words.next() else { continue };
            if root.join("tests").join(format!("{target}.rs")).is_file() {
                continue;
            }
            offenders.push(format!(
                "{name} qa_gate.check runs `--test {target}` — tests/{target}.rs does not exist"
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "a qa_gate that cannot be run is not a gate:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn every_binding_names_an_equation_its_contract_defines() {
    // `verify_bindings` reads only `status:` out of binding.yaml and opens no
    // contract file, so a binding may name an equation its contract does not
    // define and still count toward "43/43 bindings bound". One did:
    // apply-receipt-v1's `receipt_deletion`. build.rs now checks this too, but
    // a build is cacheable and a test is not.
    let registry = contracts_dir().join("binding.yaml");
    let text = std::fs::read_to_string(&registry).expect("binding.yaml must exist");
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("binding.yaml parses");
    let bindings = sequence(&doc, "bindings");
    assert!(
        !bindings.is_empty(),
        "binding.yaml declares no bindings — the registry is the thing being \
         checked, so an empty one is a finding, not a pass"
    );

    let mut offenders = Vec::new();
    for b in bindings {
        let Some(contract) = string_at(b, "contract") else {
            continue;
        };
        let Some(equation) = string_at(b, "equation") else {
            continue;
        };
        let path = contracts_dir().join(&contract);
        let Ok(body) = std::fs::read_to_string(&path) else {
            offenders.push(format!(
                "binding for `{equation}` names {contract}, which does not exist"
            ));
            continue;
        };
        let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&body).expect("contract parses");
        if doc
            .get("equations")
            .and_then(|e| e.get(&equation))
            .is_some()
        {
            continue;
        }
        offenders.push(format!(
            "{contract} does not define equation `{equation}`, but a binding claims to implement it"
        ));
    }
    assert!(
        offenders.is_empty(),
        "bindings are counted as bound without being resolved:\n  {}",
        offenders.join("\n  ")
    );
}

#[test]
fn the_resolvable_key_set_is_written_down() {
    // The boundary in the header is a decision that must survive being read by
    // someone in a hurry. If a key is added to `collect_citations` without
    // being added here, the documented scope and the enforced scope diverge —
    // which is the shape of every defect in GH-298.
    assert_eq!(
        CITATION_KEYS.len(),
        3,
        "CITATION_KEYS documents the resolvable scope; update the header's \
         boundary note in the same change"
    );
}
