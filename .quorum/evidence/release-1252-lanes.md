# Quorum lanes — PMAT-159 / v1.25.2

Eleven agy invocations, all read-only (--sandbox, no writes requested). Every lane
reported an empty changes[] array. Wave 1 ran against the pre-refactor branch; the
branch was then finalised at two commits and waves 2, 3 and 4 were briefed with the
final layout and told to re-verify and correct every wave-1 citation.

## Wave 1 — three claim lanes plus the independent review, in parallel

- claim lane A — the gate change: is widening CIT_RE to the root files, and letting a
  file the branch ADDS anchor, a weakening of the forgery defence? Returned A1-A6.
- claim lane B — the release commit and the CI step: manifest/lock/changelog/tag
  coherence, measured with git diff --stat, plus a D1 workflow review. Returned B1-B6.
- claim lane C — falsifier honesty: what each of the four new tests actually pins, and
  whether --locked rather than the test body is the enforcing mechanism. Returned C1-C6.
- the independent /teamwork-preview review, briefed on the same four questions.

## Wave 2 — three refuters, one of which had to be run twice

- R1 and R2 each attacked all twenty-one claims and returned verdicts with their own
  citations. Between them they overturned four lane claims and narrowed nine.
- R3 was briefed as refuter AND crux surveyor. ITS FIRST RUN RETURNED NOTHING USABLE:
  twenty-six items carrying claim HEADLINES only, with no verdict, no reasoning and no
  citation on any of them. The crux survey it was carrying was reassigned to judge J3
  rather than dropped, and the judges were told to break every R1/R2 tie themselves
  because no third refuter existed at the time they ran.
- R3 was then RERUN in wave 4 against a brief carrying the final file layout, a
  restriction to the twenty-one claims, and a hard requirement that every item carry a
  verdict, a reason and a citation the lane had opened itself. The rerun returned
  twenty-one real adjudications: ten SUSTAINED, seven NARROWED, four REFUTED.

## Wave 3 — three judges

- J1, J2 and J3 each adjudicated all twenty-one claims independently, were given both
  refuters verbatim and were told that where R1 and R2 disagreed there was no third
  refuter to break the tie and they had to open the file themselves.
- All three returned the same twenty-one verdicts — nineteen CONFIRMED, two REFUTED —
  and all three returned PASS with no blocker. J3 additionally carried the crux survey.
- J2 or J3 built a throwaway git repository inside the worktree to test the rename
  scenario of claim A2 empirically, and wrote a two-byte file to settle the newline
  arithmetic of claim A6. Both artefacts were untracked, were outside the diff, and
  were removed; the worktree was verified clean and at the same HEAD afterwards.

## Wave 4 — the R3 rerun, and one judge to weigh it

- The R3 rerun REFUTED three claims the panel had unanimously CONFIRMED: A2, A6 and B1.
  A late refutation that contradicts a finished panel cannot simply be filed, so a
  fourth judge J4 was run over exactly those three claims, given every argument on the
  record verbatim — the lane claim, R1, R2, R3 and all three panel verdicts — plus the
  measured length of every citable file, and told to verify each cited line itself.
- J4 upheld the panel on all three and the tally is unchanged at nineteen CONFIRMED and
  two REFUTED. Its findings on the R3 rerun are the reason this section exists: R3's
  A2 and A6 arguments merely repeat attacks R1 had already made and the panel had
  already narrowed out, and both cite lines that DO NOT EXIST — line 530 and line 624
  of an added test file that is exactly 409 lines long. Either would have been fatal
  rather than merely unanchored, because a citation past the end of a file makes the
  gate die by name. R3's B1 argument is simply false: it asserts the forjar package
  version string is one line lower in the lockfile than it is, and Cargo.lock:1171
  carries that version at both the merge-base and HEAD.
- So refuters-per-claim is THREE by count and this is stated with its caveat rather
  than as a clean number: two of the three R3 citations J4 checked do not exist, and no
  R3 argument changed a single verdict. The count is honest; the third refuter's
  contribution to the outcome was nil.

## A measured caveat about the sandbox

The lanes ran with agys --sandbox flag and none was granted write permission, yet THREE
scratch artefacts appeared inside the repository across the run: the two the judges left
in wave 3, and a 42 KB copy of the branch diff that the rerun refuter wrote into the
worktree root in wave 4 after being told in its own brief to write nothing. --sandbox is
not a write barrier and should not be reported as one. All three were untracked, none
was inside the diff, all were removed, and the worktree pointer file was checked after
every wave and never moved.
