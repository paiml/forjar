# Implementation receipt — PMAT-537 — the cookbook's lock is the measurement

verdict: PASS — gate T reads the named cookbook commit's `Cargo.lock` as well as its `Cargo.toml` and refuses a row whose cookbook does not BUILD the released version; paiml/forjar-cookbook is bumped to forjar 1.29.0 and merged; and v1.29.0's ledger row now names a commit that was actually compiled against it, which no release before this one was.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one gate arm, one fixture split, one second-repository PR)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="bash scripts/dogfood/tagged.sh"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-537-cookbook-lock.log
  cmd="cargo test --test falsification_release_cookbook_is_part_of_the_release"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-537-cookbook-lock.log
  cmd="cargo build/clippy/test --workspace --locked (paiml/forjar-cookbook#20)"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-537-cookbook-lock.log

## The defect, in two lines

On `7c100454` — the exact commit v1.29.0's ledger row named:

| file | says |
|---|---|
| `Cargo.toml` | `forjar = { version = "1.2", default-features = false }` |
| `Cargo.lock` | `name = "forjar"` / `version = "1.2.1"` |

`^1.2` admits 1.29.0, so gate T printed `cookbook 7c100454 requires forjar 1.2
ok`. What that cookbook compiles is forjar **1.2.1** — twenty-seven minor
versions behind the release it was recorded as qualifying.

**PMAT-241 built the link and the link pointed at something never built against
the release.** The requirement is a range; cargo compiles the lock. That is the
same half-true record PMAT-235 and PMAT-236 exist to refuse, produced by the
arm written to close a different half-truth.

It matters beyond a stale pin: `cookbook-qualify`, the scoring bridge and every
recipe score the cookbook produces are compiled against whatever its lockfile
pins, so those scores were measured against 1.2.1 while this repository shipped
1.3 through 1.29.

## Which of three readings, and why it is stated

"Every tagged release updates forjar-cookbook" can mean the lock pins the
released version; or the lock is merely not older than the previous release; or
the requirement admits the release. **The last is what existed and is what
produced this.** The gate takes the FIRST, and says so in its own text.

It is the only reading under which "was qualified against" is true of the thing
that was actually compiled — and it puts a real obligation on a second
repository, which is why the skill and `CLAUDE.md` now say the cookbook is
bumped and pushed **as part of the cut, before the tag**, rather than implying
it.

## The ordering, said plainly

For 1.29.0 it happened the other way round. The release was tagged and
published, the cookbook was bumped against what crates.io serves, and the row
was corrected to the commit that resulted. **The old row recorded a
qualification that never happened; the new one records one that did**, and the
qualification is later than the release it describes. That is true of this
release only, and the rule that prevents a second is now a gate rather than a
sentence.

## What the cookbook bump measured

paiml/forjar-cookbook#20, merged as `0be3e1ec`:

| check | result |
|---|---|
| `cargo build --workspace --locked` | clean |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | silent |
| `cargo test --workspace --locked --no-fail-fast` | 6 + 12 + 72 + 55 passed, 0 failed, 0 ignored |
| its own CI | 15 checks green |
| source changes needed | **none** |

Twenty-seven minor versions with no source change is worth recording in both
directions: it is evidence that forjar's public surface held, and evidence that
nothing would have noticed if it had not.

## Falsification

`tests/falsification_release_cookbook_is_part_of_the_release.rs` gains five
cases: a lock pinning the release is green; **the exact shape `7c100454` was
in** is red and names both versions and the reason; a lock AHEAD of the release
is red too, because being ahead is a different error rather than a lesser one;
a missing lock and a lock with no forjar entry are each UNMEASURED; and
forjar's entry is forjar's whatever package name precedes it in the file.

Red and green against the real repository in
`docs/audits/logs/PMAT-537-cookbook-lock.log`, 459 lines: with the old commit
in the row, `GATE T FAIL … LOCKS forjar 1.2.1 and the release is 1.29.0`; with
the new one, `GATE T v1.29.0 cookbook 0be3e1ec … requires forjar 1.29 and locks
1.29.0 ok`; and the ten cases green.

**The first version of that log was 9 lines and contained neither.** Three
review lanes read it and all three refused the receipt for claiming it held
outputs it did not. The cause was `tee "$L" | head -14` in the script that
wrote it: `head` closes the pipe, `tee` takes SIGPIPE, and the file stops where
the terminal output did. That is the SIGPIPE class this repository has a rule
about and an open ticket for (PMAT-240), in a proof script, written by the
person who wrote the rule. The log is regenerated with no pipe, and the red
half runs in a scratch clone because committing the defect here has disturbed
this working tree four times in this session.

The fixture's stub had to learn to answer the two files separately — one that
returned the manifest for both would have made every lock case measure the
wrong file, which is the confusion this ticket is about. It moves to
`tests/release_goal_fixture/stub.rs` at the 500-line ratchet.

## Gaps, named

- The arm reads the cookbook's ROOT `Cargo.lock`. A workspace member with its
  own lock is not read; this cookbook has one lock for three crates, so the
  question does not arise here and would if it changed.
- The lock must equal the release EXACTLY. A patch release that the cookbook
  has not yet been bumped to will make the cut red, which is the intended cost
  and is now paid on every cut rather than never.
- Nothing measures that the cookbook's tests actually RUN against the pinned
  version at release time — the bump's own CI did, once, by hand. Making that
  part of `make dogfood-release` would need the cookbook checked out in the
  gate, which gate D already does for its configs and this arm does not.

IMPL-PMAT-537-RECEIPT-END
