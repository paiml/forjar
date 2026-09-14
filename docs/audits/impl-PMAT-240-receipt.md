# Implementation receipt — PMAT-240 — the SIGPIPE class, over the tree

verdict: PASS — fifteen pipelines that could return 141 under `set -o pipefail` are closed, and the rule that refuses a new one walks EVERY stage of EVERY pipeline in EVERY script under `scripts/`, not the three files PMAT-239 owned. The rule found three sites that neither the grep nor PMAT-239's own census had ever listed.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=mechanical  route=self  w=100.00  basis=absent  (fifteen shell edits and one rule)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_no_script_pipes_into_an_early_exit (in a clone, over main's scripts)"  claimed_exit=-  rerun_exit=101  log_path=docs/audits/logs/PMAT-240-sigpipe-class.log
  cmd="cargo test --test falsification_no_script_pipes_into_an_early_exit"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-240-sigpipe-class.log
  cmd="scripts/dogfood/{comply,docs,contracts,crux-reconcile,tagged,release-check}.sh"  claimed_exit=-  rerun_exit=0  log_path=docs/audits/logs/PMAT-240-sigpipe-class.log

## The class

A pipeline whose right-hand side can exit before its left-hand side finishes —
`grep -q`, `grep -m`, `head`, `jq -e` — makes the left side take SIGPIPE. Under
`set -o pipefail` the pipeline returns **141**, and a gate reading its own
output reports UNMEASURED at random. Gate T did that on about one run in three.

## Why the rule matters more than the fifteen fixes

PMAT-239 fixed three instances and wrote a rule covering **three files**. Its
own census found eighteen more sites; thirteen were still there when this
ticket opened.

**This session hit the class twice more while closing it.** Gate T's new T9 arm
shipped with `printf | awk '… exit'` and died 141 on its first run — in the
file that already carried PMAT-239's rule. And a proof log written with
`tee "$LOG" | head -14` landed as 9 lines instead of 459, which three review
lanes refused a receipt over. Both by someone who had just read the rule.

A rule that covers three files is a rule you can walk out of.

## What the rule found that the census did not

Two corrections to the rule, each of which turned up sites nobody had listed:

**Every script, not only those whose text contains `pipefail`.** That filter
skipped `scripts/dogfood/lib/releases.sh` — a LIBRARY, sourced by gates that do
set it, so it inherits pipefail and dies identically. Two fixed sites were in
it and the rule would have caught neither. Pipefail is not the whole hazard
anyway: without it the left side still takes SIGPIPE and its output is still
truncated, silently, which is exactly what happened to the proof log.

**Every stage, not just the one after the first pipe.** `printf | sed | head -1`
has a safe middle and a fatal end. Three more sites:

| site | shape |
|---|---|
| `crux-reconcile.sh:74` | `printf \| sed -n \| head -1` |
| `publish-from-tag.sh:88` | `git show \| manifest_version \| head -n1` |
| `release-check.sh:195` | `git tag \| grep -vxF \| head -1` |

The second is in the script that publishes to crates.io, where a 141 is a
release that stops with no reason given.

## The fifteen, by shape

- **five** `printf | grep -q` / `printf | jq -e` → here-strings
  (`docs.sh`, `contracts.sh`, `cb200-ratchet.sh`, `crux-reconcile.sh`,
  `lib/releases.sh`);
- **three** `sed Cargo.toml | head -1` → one awk that exits on its own read
  (`release-check.sh`, `lib/binary.sh`, `crux-reconcile.sh`);
- **four** `git tag … | head -1` and the two three-stage ones → a capture plus a
  parameter expansion, or one awk over a captured list;
- **two** `printf | sed | head -1` → one awk over a here-string;
- **one** `sed file | head -12` → `head -12 file | sed`, so head reads the file
  and sed consumes everything head produced.

## One rewrite is not a preservation, and says so

A review lane measured it: the old `sed -n 's/^version = "\(.*\)"$/\1/p'`
anchors the closing quote at end of line, so

```
version = "1.29.0" # a trailing comment
```

— valid TOML — yields **nothing**, and the script fails with "cannot read
version from Cargo.toml". The awk stops at the closing quote and reads
`1.29.0`.

That is a **behaviour change**, in the right direction, and it is recorded as a
change rather than filed under "preserves behaviour" in all three scripts that
carry it. Nothing in the tree pins it — see the gaps.

## The exception is written, or it is not an exception

A pipeline that is genuinely safe carries `# sigpipe-ok: <reason>` on its line.
An exception nobody wrote down is the same as no rule.

**And a trailing comment is not code.** The rule's first version fired on
`… ' Cargo.toml)"  # … \`sed file | head -1\`` — a comment describing the
pipeline that had just been removed. A rule that reads prose pushes people to
stop explaining their fixes.

## Falsification

`tests/falsification_no_script_pipes_into_an_early_exit.rs`, three cases: the
walk itself, and one asserting the census is repository-wide — it names the
three files PMAT-239 owned AND the library its scope missed, checks that
library still sets no `pipefail` of its own, and plants a pipeline in the
MIDDLE of its text — asserting the reported LINE NUMBER, because a lane pointed
out that planting at the end would pass under a rule that read only a file's
last line.

Plus a third case carrying **the three shapes three review lanes walked out
through**, each one line of shell: a `#` inside a string that hid the rest of
the pipeline, a `|` inside a string that was flagged as one, and a line
continuation that split a pipeline in two. And the safe shapes that must stay
safe: `||`, the early-exit command as the LEFTMOST stage, a written
`# sigpipe-ok:`, and a whole-line comment.

A rule's holes are the part worth regression-testing. The fixes are one
function and the next person to touch it will not have read the round.

Red in a scratch clone with its own `CARGO_TARGET_DIR`, over main's `scripts/`:
**15 pipelines**, named with file, line and text. Green here. Six gates pass at
HEAD, all in `docs/audits/logs/PMAT-240-sigpipe-class.log` — **the gates were
not re-run against main's scripts**, so "either way" would have been a claim
the log does not support, and a review lane said so about the first draft of
this sentence.

The first clone ran a STALE test binary from the shared target directory and
reported 10; the rebuild with its own target directory reported 15. A proof
that reuses a cached binary is measuring the cache.

## Gaps, named

- The predicate is textual. A pipeline built by `eval`, or one whose command
  name arrives in a variable, is invisible to it. Quoting, comments inside
  strings and line continuations are handled, each because a review lane walked
  out through it first.
- **The `sed` → `awk` version readers are a behaviour change and nothing pins
  it.** A manifest with a trailing comment on its version line now reads where
  it used to fail. Three scripts carry the change and none has a case for it;
  the right home is a falsifier for the version reader itself, which this
  ticket did not write.
- `scripts/ledger-replay.sh` still carries one bashrs SEC011 error, unrelated
  and accounted for by gate B's `BASHRS_ERROR_CEILING=1`.
- Nothing checks the same shape outside `scripts/` — `Makefile` recipes and
  `.github/workflows` run pipelines too, and neither is walked.

IMPL-PMAT-240-RECEIPT-END
