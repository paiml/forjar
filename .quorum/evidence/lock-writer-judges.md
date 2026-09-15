# PMAT-565 — adjudicated claims

One round of three sandboxed agy quorum lanes: 2 FAIL, 1 PASS, not agreed.
Five confirmations and four refutations, every one re-measured on this host
before it was acted on. The stamp held; the branch's claim that it sat on the
ONLY writer did not, and the one lane that said so was the one that was wrong.

## CONFIRMED

1. [stamp] That `save_lock` writes the real writer whatever the struct
   carried, and keeps the replaced value as the creator once (all three
   lanes; lane 1 measured).
   - evidence: `src/core/state/mod.rs:58` — `stamped_for_write` sets
     `created_by` only when None, then `generator = writer_stamp()`;
     `src/core/state/mod.rs:79` is the write. The fixture at
     `tests/falsification_lock_names_its_writer.rs:50` writes a fake first
     writer and reads the real one back; mutation M1 (serialise the raw lock)
     kills five of six cases.

2. [empty-generator] That an empty generator leaves `created_by` None and
   `lock-audit`'s `starts_with("forjar")` passes on every write (lane 1
   measured).
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

4. [literals] That none of the scripted `created_by: None` insertions landed
   in a GlobalLock or StackStamp literal (lanes 1 and 2).
   - evidence: neither struct has the field, so a misplacement is a compile
     error, and `cargo check --all-targets` is clean.

5. [falsifiers] That each contract falsifier's mutation turns its cited test
   red (lanes 1 and 2 reasoned; run here).
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

2. [restamp-scope] That restamp "rewrites every lock under a state dir"
   (this author, `CHANGELOG.md:10` as first written).
   - corrected: it walks `<state_dir>/<machine>/state.lock.yaml`, one level.
     Lane 1 named the three things that leaves out — a nested state dir,
     `forjar.lock.yaml` (restamped by every apply through `state::stamp`),
     and encrypted `.yaml.age` locks. The CHANGELOG, the book and the
     contract now say exactly that; the walk itself is unchanged, because a
     nested dir is a state dir of its own and is restamped by naming it.

3. [no-bypass] That "no direct serde_yaml_ng + fs::write exists in src/ for
   StateLock" (lane 2, the one distinct model, graded measured).
   - corrected: it did, at the three sites above. The two lanes that shared
     a model id found it; the distinct one missed it — the opposite of the
     PMAT-564 round, where the two resamples were wrong together. Recorded
     because the duplicate-model caveat cuts both ways.

4. [no-writes] That the review could not be interrupted without cost (this
   author's assumption in dispatching it).
   - corrected: the first dispatch was interrupted by the operator before
     any lane launched; a second dispatch with a fresh `--not-before` ran the
     round. Nothing from the first reached disk; the re-dispatch is the one
     the numbers above describe. Named so the receipt's single round is not
     read as a single attempt.
