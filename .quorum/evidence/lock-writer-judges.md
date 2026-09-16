# PMAT-565 — adjudicated claims

One round of three sandboxed agy quorum lanes: 2 FAIL, 1 PASS, not agreed.
Five confirmations and four refutations, every one re-measured on this host
before it was acted on. The stamp held; the branch's claim that it sat on the
ONLY writer did not, and the one lane that said so was the one that was wrong.

## CONFIRMED

1. [stamp] That `save_lock` writes the real writer into `generator` whatever
   the in-memory struct carried, and moves the value it replaces into
   `created_by` exactly once, the first time (all three lanes; lane 1 graded it
   measured).
   - evidence: `src/core/state/mod.rs:58` — `stamped_for_write` sets
     `created_by` only when None, then `generator = writer_stamp()`;
     `src/core/state/mod.rs:79` is the write. The fixture at
     `tests/falsification_lock_names_its_writer.rs:50` writes a fake first
     writer and reads the real one back; mutation M1 (serialise the raw lock)
     kills five of six cases.

2. [empty-generator] That a lock whose `generator` is the empty string keeps
   `created_by` None rather than recording an empty creator, and that
   `lock-audit`'s `starts_with("forjar")` check passes after every write
   because the stamped writer always begins with `forjar` (lane 1 measured).
   - evidence: the `!out.generator.is_empty()` guard at
     `src/core/state/mod.rs:58`; the written generator is always
     `forjar <version>`.

3. [compat] That an older forjar parses the file (no `deny_unknown_fields`
   on `StateLock`), the `.b3` sidecar stays consistent under restamp, and
   encrypted locks are untouched (lanes 1 and 2).
   - evidence: `src/core/types/state_types.rs:100` carries serde default
     and skip_serializing_if; restamp writes through `save_lock`, which
     writes the sidecar, asserted at
     `tests/falsification_lock_names_its_writer.rs:213`.

4. [literals] That none of the 140 scripted `created_by: None` insertions
   landed inside a GlobalLock or StackStamp literal, both of which also carry a
   `generator` line the script keyed on (lanes 1 and 2 spot-checked).
   - evidence: neither struct has the field, so a misplacement is a compile
     error, and `cargo check --all-targets` is clean.

5. [falsifiers] That each falsifier in `contracts/lock-names-its-writer-v1.yaml`
   names a mutation that turns its cited test red — which lanes 1 and 2 reasoned
   from the test bodies, and which was run here rather than accepted.
   - evidence: M1–M4 in the pmat digest.

## REFUTED

1. [one-writer] That "every path writes through save_lock — apply's
   finalize, --refresh, repair, restamp" (this author, in the contract as
   first written, and "the one writer every path goes through" in the
   CHANGELOG).
   - corrected: lanes 1 and 3 read `src/cli/lock_repair.rs` and found the
     minimal repaired lock and the normalised form written with a bare
     `fs::write`; lane 1 added `lock-migrate`. All three go through the
     writer now — `src/cli/lock_repair.rs:44`, `src/cli/lock_repair.rs:106`,
     `src/cli/lock_audit.rs:333` — with a falsifier at
     `tests/falsification_lock_names_its_writer.rs:298` that also checks the
     `.b3` sidecar those paths used to leave stale for the next apply to
     refuse on. `lock-restore` and `lock-tag` copy bytes and are named as the
     exception in the contract.

2. [restamp-scope] That `forjar lock --restamp` "rewrites every lock under a
   state dir", as the CHANGELOG paragraph at `CHANGELOG.md:10` said when this
   author first wrote it (lane 1, graded measured).
   - corrected: it walks `<state_dir>/<machine>/state.lock.yaml`, one level.
     Lane 1 named the three things that leaves out — a nested state dir,
     `forjar.lock.yaml` (restamped by every apply through `state::stamp`),
     and encrypted `.yaml.age` locks. The CHANGELOG, the book and the
     contract now say exactly that; the walk itself is unchanged, because a
     nested dir is a state dir of its own and is restamped by naming it.

3. [no-bypass] That "no direct serde_yaml_ng + fs::write exists in src/ for
   StateLock" — the claim lane 2, the only lane with a distinct model id,
   graded measured and passed the branch on.
   - corrected: it did, at the three sites above. The two lanes that shared
     a model id found it; the distinct one missed it — the opposite of the
     PMAT-564 round, where the two resamples were wrong together. Recorded
     because the duplicate-model caveat cuts both ways.

4. [no-writes] That dispatching the review round once was enough and that an
   interruption part-way through could not leave a partial round behind (this
   author's assumption in dispatching it).
   - corrected: the first dispatch was interrupted by the operator before
     any lane launched; a second dispatch with a fresh `--not-before` ran the
     round. Nothing from the first reached disk; the re-dispatch is the one
     the numbers above describe. Named so the receipt's single round is not
     read as a single attempt.

5. [fixed-once-means-fixed] That the every-write claim held after the FIRST
   round's repair of lock-repair and lock-migrate — the contract clause, the
   receipt's verdict and the suite all said so, and all three were wrong about
   three more verbs that were rewriting a StateLock by hand.
   - evidence: the merge-rail round cited `src/cli/destroy.rs:93`
     (`cleanup_succeeded_entries` serialising and writing a pruned lock, then
     re-sealing by hand), `src/cli/lock_lifecycle.rs:185` (lock-defrag MIRRORING
     save_lock through a local helper that never grew the writer stamp) and
     `src/cli/lock_merge.rs:61` with two siblings (a bare write that also
     produced NO `.b3` sidecar, so a merged state dir failed the next apply's
     integrity check). All three call `save_lock` now and the mirror is deleted.
   - corrected: `tests/falsification_lock_names_its_writer.rs:368` and `:404`
     drive lock-defrag and lock-merge through the binary and were RED on the
     unfixed source in a scratch clone, with `forjar 0.0.0-fake-first-writer`
     surviving the rewrite; `cleanup_succeeded_entries_writes_through_the_writer`
     covers the third in-crate, since the function is `pub(crate)`. The contract
     clause and the receipt verdict now name all five verbs and say which round
     found which, because a claim is worth exactly what its cases cover.
