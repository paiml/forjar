# PMAT-241 — the lanes, the briefs, and what the rounds cost

## Round one — the design

Three sandboxed agy quorum lanes on the ticket as first implemented: the
`cookbook:` field, the `sed` requirement parser, and `dogfood_semver_ge` as the
admission test. 3/3 FAIL. Five refutations, listed in the judges digest as
items 1, 6, 7 and 8 (8 carries two wording refutations).

## Round two — the implementation

Three sandboxed lanes, width 3, on the diff at `9f25f006` against base
`f98b388d`, pinned with `--not-before` so a stale lane set from another ticket
cannot be reduced into this one. 3/3 FAIL, all three returning, lane exits
0/0/0, durations 559s, 489s and 496s.

The brief fixed the ORDER of the attack rather than asking for a review:

1. find an input where the rule disagrees with real Cargo, checking against
   `cargo` or the `semver` crate in the lane's OWN temp directory;
2. find a real-world `Cargo.toml` shape the parser silently misreads — named
   candidates: an inline table spanning lines, `forjar.workspace = true`, a
   `[target.'cfg(unix)'.dependencies]` section, a comment, a `forjar-something`
   key, leading whitespace, CRLF;
3. attack the test's own extraction: can it pass while the gate is broken, or
   fail while the gate is fine;
4. verify the ten-tag count from the repository's own tags;
5. check containment: `crc`, the fall-off-the-end path, and `fail`.

Four of the five produced findings, and the one that mattered most is the one
the ticket had not thought to ask — that the operator is half of what the
requirement means. Item 2's named candidate list produced the comment finding
from all three lanes; the other candidates on that list are still open and are
named as gaps in the receipt rather than closed silently.

## The no-write rule

Every brief now OPENS with it, because a lane in an earlier ticket's round
committed to the live repository and left HEAD on a scratch branch, capturing
five later commits into a push. This round's repository was verified clean
after it: `git status --porcelain` showed only the orchestrator's own untracked
logs, and HEAD was still the commit the round was dispatched on.

## The orchestrator's own failure in this round

Between the two rounds, a red proof ran `git checkout HEAD -- <path>` to
restore two scripts while their fixes were still UNCOMMITTED. HEAD was the
commit before them, so the restore reverted the work. It was replayed from the
session's own edit scripts and the proof re-run from a committed state. The
rule this repository already learned once, and broke again: commit before the
red proof, and check out the NAMED PARENT to go back, never `HEAD`.
