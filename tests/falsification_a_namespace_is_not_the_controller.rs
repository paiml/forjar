//! forjar#495: three places still decide whether they may read THIS host's
//! filesystem on a machine's behalf using a predicate that forgets a namespace.
//!
//! `transport::machine_is_local` checks `!is_container_transport()` plus a
//! local address. `transport::controller_answers_for` — the definition
//! forjar#485 introduced, which drift and the baseline writer share — also
//! checks `!is_pepita_transport()`, and says why: a pepita machine commonly
//! declares `addr: 127.0.0.1` while its files live inside the namespace, so
//! reading the controller's path of the same name answers about the wrong host.
//!
//! That is the same defect #485 was: measure one filesystem, report the answer
//! as another's. It was fixed for the lock baseline and left at four more call
//! sites, which three review lanes found and this rule closes.
//!
//! THE FOUR SITES ARE NOT THE SAME, and this rule does not pretend they are.
//!
//! Three of them gate a controller-side READ and must use the strict predicate:
//! the build-I/O probe in `core::task::probe`, the pre-plan probe in
//! `core::executor`, and output verification in `core::executor::output_verify`.
//!
//! The fourth, in `transport::exec_script`, must NOT change and is exempted by
//! name below. `exec_script` returns for pepita and for container BEFORE it
//! reaches `machine_is_local`, so at that line the machine is provably neither,
//! the container exclusion is redundant and the missing namespace exclusion is
//! unreachable. Tightening it there would be a change with no behaviour behind
//! it, made to satisfy a rule rather than a defect. The ordering is asserted
//! here so that if it ever stops holding, this exemption fails rather than
//! quietly becoming false.

use std::fs;
use std::path::Path;

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()))
}

/// The same source with comments removed — whole-line, trailing AND block.
///
/// A rule that reads prose is not a rule. The first version searched raw text
/// and then failed on the FIXED tree, because each site now carries a comment
/// saying why the loose predicate is not used: the rule was reading its own
/// explanation. Review then pointed out that stripping only whole-line `//`
/// left two doors open, a trailing comment and a `/* */` block, so all three
/// are stripped now.
///
/// This repository has met that shape three times (RULE 8 of the release
/// workflow gate, the cargo PATH prelude, and here), which is why the first
/// instance was recognised within one run and the remaining two were found by
/// review rather than by a user.
fn code_only(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_block = false;
    for line in src.lines() {
        let mut kept = String::new();
        let bytes: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            if in_block {
                if i + 1 < bytes.len() && bytes[i] == '*' && bytes[i + 1] == '/' {
                    in_block = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if i + 1 < bytes.len() && bytes[i] == '/' && bytes[i + 1] == '*' {
                in_block = true;
                i += 2;
                continue;
            }
            if i + 1 < bytes.len() && bytes[i] == '/' && bytes[i + 1] == '/' {
                break; // whole-line or trailing
            }
            kept.push(bytes[i]);
            i += 1;
        }
        out.push_str(&kept);
        out.push('\n');
    }
    out
}

/// Every `.rs` file under `src/`, so a NEW controller-side read cannot be added
/// somewhere this rule was not told to look.
///
/// The first version named three files. Review was right that this made the
/// rule vacuous against exactly the thing it exists to prevent: a fourth site.
fn all_sources() -> Vec<std::path::PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut v);
    v
}

/// Every site that gates a controller-side read must use the strict predicate.
///
/// The sweep is the WHOLE `src/` tree, and the one file allowed to name the
/// loose predicate is listed by name with its reason.
#[test]
fn a_controller_side_read_is_never_gated_on_the_loose_predicate() {
    // `transport/mod.rs` DEFINES `machine_is_local` and calls it in
    // `exec_script_tracked`, where it is unreachable for a namespace because
    // the dispatcher has already returned. The next case asserts that ordering,
    // so this exemption cannot outlive the reason for it.
    const EXEMPT: [&str; 1] = ["src/transport/mod.rs"];

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let files = all_sources();
    assert!(
        files.len() > 100,
        "the sweep found only {} source file(s), so it is measuring nothing",
        files.len()
    );

    let mut offenders = Vec::new();
    let mut exempt_seen = 0usize;
    for f in &files {
        let rel = f
            .strip_prefix(root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        let Ok(raw) = fs::read_to_string(f) else {
            continue;
        };
        let src = code_only(&raw);
        if !src.contains("machine_is_local") {
            continue;
        }
        // A test that merely names the predicate to check something about it is
        // not a controller-side read.
        if rel.contains("/tests_") || rel.ends_with("test_fixtures.rs") {
            continue;
        }
        if EXEMPT.contains(&rel.as_str()) {
            exempt_seen += 1;
            continue;
        }
        offenders.push(rel);
    }

    assert_eq!(
        exempt_seen,
        EXEMPT.len(),
        "the exemption list names {} file(s) and {exempt_seen} of them still \
         mention the loose predicate. An exemption for something that no longer \
         needs it is dead text: drop the name.",
        EXEMPT.len()
    );
    assert!(
        offenders.is_empty(),
        "forjar#495: {} file(s) gate on `machine_is_local`, which admits a \
         pepita namespace. A namespace has its own rootfs, so reading THIS \
         host's path of the same name answers about the wrong filesystem — the \
         defect forjar#485 fixed for the lock baseline. Use \
         `controller_answers_for`:\n  {}",
        offenders.len(),
        offenders.join("\n  ")
    );
}

/// The one site that must NOT change, and the ordering that makes it safe.
#[test]
fn exec_script_reaches_its_local_check_only_for_a_machine_that_is_neither() {
    let src = read("src/transport/mod.rs");
    // The dispatcher is `exec_script_tracked`; `exec_script` is a thin wrapper.
    let body = src
        .split("fn exec_script_tracked")
        .nth(1)
        .expect("the transport dispatcher must be in transport");
    let body = &body[..body.find("\n}\n").map_or(body.len(), |i| i + 2)];

    // A MENTION IS NOT A DISPATCH. Review noted that a lexical position proves
    // nothing: binding the names above the early returns would satisfy an
    // ordering check while changing what the code does. So each predicate must
    // appear in a line that RETURNS, which a binding does not.
    let lines: Vec<&str> = body.lines().collect();
    let returning = |needle: &str| -> Option<usize> {
        let mut at = 0usize;
        for (i, line) in lines.iter().enumerate() {
            if line.contains(needle) {
                // The guard's body, not just its condition: `if p() {` and
                // `return ...;` sit on different lines.
                let guard = lines[i..(i + 3).min(lines.len())].join(" ");
                if guard.contains("return") {
                    return Some(at);
                }
            }
            at += line.len() + 1;
        }
        None
    };
    let pepita = returning("is_pepita_transport")
        .expect("exec_script must RETURN for a pepita namespace, not merely mention it");
    let container = returning("is_container_transport")
        .expect("exec_script must RETURN for a container, not merely mention it");
    let local = body
        .find("machine_is_local")
        .expect("exec_script must still decide local versus ssh");

    assert!(
        pepita < local && container < local,
        "forjar#495: `exec_script` reaches `machine_is_local` BEFORE returning \
         for a namespace or a container, so that call is no longer unreachable \
         for them and the loose predicate now decides something. Either restore \
         the ordering or switch that site to the strict predicate — the \
         exemption in this file's header depends on the ordering and is now \
         false.\npepita@{pepita} container@{container} local@{local}"
    );
}
