//! forjar#485: apply writes the baseline that drift reads, and the two had
//! their own answer to both halves of "what does that machine hold here?".
//!
//! WHICH MACHINE ANSWERS. `machine_is_local` excludes a container and not a
//! pepita namespace; drift's `reads_the_controller` excludes both. Using the
//! first to decide whether to hash this host left apply reading the controller
//! for a pepita machine while drift asked the namespace — the two sides
//! permanently disagreed rather than merely both wrong. Three review lanes
//! found it independently.
//!
//! HOW THE BYTES ARE READ. The writer ran a plain `cat '<path>'`; drift runs
//! `if [ -d ]; then echo __DIR__; else cat; fi` and, on seeing that marker,
//! digests `ls -la` instead. They disagree on a directory, and on a file whose
//! entire content is the literal `__DIR__` — a permanent, confident mismatch on
//! a resource that is perfectly converged.
//!
//! Both halves now have ONE implementation. The behavioural cases live beside
//! the code they exercise, because `build_resource_details` is crate-private;
//! what a second implementation looks like is checkable from here, and that is
//! what this file pins.

use std::fs;
use std::path::Path;

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()))
}

#[test]
fn the_baseline_writer_uses_the_reader_drift_uses() {
    let helpers = read("src/core/executor/helpers.rs");

    assert!(
        helpers.contains("remote_path_digest"),
        "forjar#485: the remote arm of build_resource_details must go through \
         the ONE reader drift uses. A second read protocol is how the __DIR__ \
         collision got in: the writer digested the literal string and the \
         reader digested an `ls -la` listing, for ever."
    );
    assert!(
        !helpers.contains("cat '{path}'"),
        "forjar#485: the writer has grown its own `cat` script again. Drift \
         answers __DIR__ for a directory and digests a listing instead, so a \
         plain `cat` disagrees with it on a directory and on a file whose \
         content IS that marker."
    );
}

#[test]
fn the_baseline_writer_uses_the_predicate_drift_uses() {
    let helpers = read("src/core/executor/helpers.rs");
    let drift = read("src/tripwire/drift/file.rs");
    let transport = read("src/transport/mod.rs");

    assert!(
        transport.contains("pub fn controller_answers_for"),
        "the one definition must exist in transport, beside the predicate it is \
         stricter than"
    );
    assert!(
        helpers.contains("controller_answers_for"),
        "forjar#485: the baseline writer must ask the shared predicate whether \
         this host answers for that machine. `machine_is_local` is not it: it \
         excludes a container and forgets a pepita namespace, so apply hashed \
         the controller while drift asked the namespace."
    );
    assert!(
        drift.contains("controller_answers_for"),
        "and drift must delegate to the same one, or there are two definitions \
         again and nothing stops them diverging. `machine_is_local`'s own doc \
         asks for this: one definition, one place."
    );

    // The stricter predicate must actually be stricter. If it ever loses the
    // namespace exclusion, the pepita hole reopens silently.
    let body = transport
        .split("pub fn controller_answers_for")
        .nth(1)
        .expect("the definition was just asserted to exist");
    let body = &body[..body.find("\n}").map_or(body.len(), |i| i + 2)];
    for required in [
        "is_container_transport",
        "is_pepita_transport",
        "is_local_addr",
    ] {
        assert!(
            body.contains(required),
            "forjar#485: controller_answers_for no longer consults `{required}`. \
             It must exclude BOTH a container and a namespace and require a \
             local address, because `exec_script` dispatches pepita and \
             container before it ever reaches the local arm.\n{body}"
        );
    }
}
