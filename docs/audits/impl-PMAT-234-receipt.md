# Implementation receipt — PMAT-234 — gate R's verdict says what is pending, and why

verdict: PASS — `scripts/dogfood/release-check.sh` kept ONE flat `pending` string and read it as "the tag is not cut yet", so after every successful release it printed `PASS pre-tag … PENDING until the tag is cut` about a version that was tagged, published, on crates.io and rendered on docs.rs. The notes are two sets now, and either verdict reports the other set rather than dropping it. Two of three review lanes then found a second false statement in the one remaining note, and it reproduced on the real repository.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one script, one test file; no lane may edit a dogfood gate)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (three lanes, 1 PASS 2 FAIL)
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_release_check_says_what_is_pending"  claimed_exit=101(lanes)  rerun_exit=0  log_path=docs/audits/logs/PMAT-234-pretag.log
  cmd="cargo test --test falsification_dogfood_release_check_pr_window"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-234-pretag.log
  cmd="bash scripts/dogfood/release-check.sh"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-234-pretag.log

## The defect, as the real repository printed it

Before:

```
GATE R PASS pre-tag: 19 PR(s) since v1.27.0 (GitHub reports 19 merged in that
window, 0 of them after this HEAD) all carry receipt=ok; PENDING until the tag
is cut: no docs/audits/crux-1.28.0.md: Cargo.toml is still at v1.28.0's version
(1.28.0), no release is being cut
```

v1.28.0 was tagged, released, on crates.io and rendered on docs.rs when that
line was printed. Two claims in it were false and a third was: the release is
not `pre-tag`, nothing is `PENDING until the tag is cut`, and
`docs/audits/crux-1.28.0.md` is present and 13KB long.

After:

```
GATE R PASS v1.28.0 is on main and on origin; GitHub release published
(prerelease=false); crates.io serves forjar 1.28.0; docs.rs built the docs;
19 PR(s) since v1.27.0 (of 19 GitHub reports merged in that window) all carry
receipt=ok; also pending: no docs/audits/crux-1.28.0.md is owed (it is
present): Cargo.toml is still at v1.28.0's version (1.28.0), so no release is
being cut and its reconciliation is not re-run here
```

## The shape of the fix

Arm 6 adds a note after EVERY successful release — `Cargo.toml` is back at the
newest tag's version, so no cut is in flight and no crux document is owed — and
one flat string could not tell that apart from "the tag does not exist".

`note_pretag` is for the arms whose obligation does not exist YET because the
tag does not: the tag, the GitHub release, crates.io, docs.rs. `note_pending`
is for everything else, a note about the tree that is true whether or not a tag
exists, and it never licenses the words `pre-tag` or `PENDING until the tag is
cut`. Either verdict reports the other set as `also pending`, so the sentence
is not bought by dropping a note.

## What the lanes found

Three lanes, one PASS and two FAIL. All three confirmed the six claims put to
them: the bucket assignment is complete over all five call sites; no `fail`
became a note; `also pending` hides nothing and does not sit beside a false
`CRUX present`; the three cases falsify (each lane rebuilt the harness in its
own temp directory and watched the published case go red against `origin/main`
and green at HEAD); the fixture extraction is behaviour-preserving; and the
`cargo search` stub matches the real output shape, which one lane checked by
running `cargo search` itself.

**Two lanes then refuted something the brief had not asked about**: Arm 6's
note asserted the crux document was MISSING, and on this repository it is
present. Re-run here before it was accepted — `ls` says 13,860 bytes, and the
gate said "no docs/audits/crux-1.28.0.md". A verdict line being fixed for
saying false things must not keep one of its own. The note now reports the
document's state as measured, in both directions, and a fourth case drives
both.

## Falsification

`tests/falsification_release_check_says_what_is_pending.rs`, four cases:

1. the published state never says `pre-tag` or `PENDING until the tag is cut`,
   and still reports the remaining note;
2. the pre-tag state STILL says both, with no `also pending` — so the fix
   cannot buy its correctness by deleting the words;
3. a published release whose docs never built is still RED and names
   `doc_status=false`, because the cheapest way for a rewording to become a
   hole is for a FAIL to start reading as a note;
4. the remaining note does not claim a present file is missing, driven with the
   document there and with it removed.

Red/green in `docs/audits/logs/PMAT-234-pretag.log`, against the script as
`origin/main` has it and against the fix, both in the fixture and against the
REAL repository.

## The fixture moved

`tests/falsification_dogfood_release_check_pr_window.rs` was 440 lines against
a 500-line ratchet, so its fixture is now
`tests/release_check_fixture/mod.rs`. All nine existing cases are unchanged and
still pass; three lanes diffed the moved code and reported no behavioural
difference.

The fixture grew the state a FINISHED release leaves: the tag in both the
checkout and origin, `Cargo.toml` back at its version. Arms 3 and 4 shell out
to `cargo search` and `curl` BY NAME with no environment variable to inject, so
the fixture puts a stub of each first on `PATH`. That is the honest injection
point — it is how the script resolves them — and a test that skipped those arms
would not be exercising the state these cases are about.

## Gaps, named

- The `cargo` and `curl` stubs answer one shape each. A real `cargo search`
  that printed a different crate first, or a docs.rs that returned a body with
  no `doc_status` key, are not driven.
- Arm 6's decision to skip the crux reconciliation when no cut is in flight is
  unchanged. This ticket corrected what the gate SAYS about that decision, not
  the decision.
- `gh release view` is stubbed with one release object. A prerelease is
  reported but not asserted on by any case here.

IMPL-PMAT-234-RECEIPT-END
