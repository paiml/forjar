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

/// The same source with comment lines removed.
///
/// A rule that reads prose is not a rule. The first version of this file
/// searched raw text for `machine_is_local` and then failed on the fixed tree,
/// because the fix leaves comments at each site saying WHY the loose predicate
/// is not used — the rule was reading its own explanation. This repository has
/// hit that exact shape twice before (RULE 8 of the release-workflow gate, and
/// the cargo PATH prelude), which is why the mistake was recognisable here
/// within one run instead of shipping.
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every site that gates a controller-side read must use the strict predicate.
#[test]
fn a_controller_side_read_is_never_gated_on_the_loose_predicate() {
    for (file, what) in [
        (
            "src/core/task/probe.rs",
            "the build-I/O probe hashes declared inputs and outputs on the controller",
        ),
        (
            "src/core/executor/mod.rs",
            "the pre-plan probe does the same before planning",
        ),
        (
            "src/core/executor/output_verify.rs",
            "output verification stats declared artifacts on the controller",
        ),
    ] {
        let src = code_only(&read(file));
        assert!(
            !src.contains("machine_is_local"),
            "forjar#495: {file} gates a controller-side read on \
             `machine_is_local`, which admits a pepita namespace. {what}, so for \
             a namespaced machine it measures this host and reports the answer \
             as the target's — the defect forjar#485 fixed for the lock \
             baseline. Use `controller_answers_for`."
        );
        assert!(
            src.contains("controller_answers_for"),
            "forjar#495: {file} no longer names a predicate at all. It must ask \
             `controller_answers_for` before reading this host on a machine's \
             behalf."
        );
    }
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

    let pepita = body
        .find("is_pepita_transport")
        .expect("exec_script must dispatch pepita");
    let container = body
        .find("is_container_transport")
        .expect("exec_script must dispatch container");
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
