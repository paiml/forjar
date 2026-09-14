# PMAT-540 — adjudicated claims

One round of three sandboxed agy quorum lanes: 3/3 FAIL, 22 findings. Five
confirmations and seven refutations, every one re-measured before it was acted
on.

The arm held. Four sentences about it did not, and one of them had made the test
fixture unable to catch the only regression the suite exists to prevent.

## CONFIRMED

1. [failure-paths] That every way the arm can fail to measure is UNMEASURED
   rather than a silent skip.
   - evidence: a lane worked each path: a failing `git log -1` sets `mrc` and
     fails by name; a merge commit absent from the clone makes
     `git merge-base --is-ancestor` exit 128, which is `> 1` and caught; a
     literal `"null"` oid passes the emptiness test and then fails the same way.
     `tests/falsification_gate_a_the_pr_and_its_commits_name_one_ticket.rs:181`
     drives the shape that must stay strict.

2. [is-ancestor] That `git merge-base --is-ancestor "$merge" "$TRAILER_FLOOR"`
   has its arguments in the order the comment claims.
   - evidence: a lane confirmed the first argument is the candidate ancestor, so
     the question asked is "did this PR merge at or before the floor". It also
     named the property that makes a floor on a DIFFERENT branch wrong -- it
     would exempt everything up to the fork point -- which is why the floor is a
     commit on main and is asserted to resolve at
     `tests/falsification_gate_a_the_pr_and_its_commits_name_one_ticket.rs:205`.

3. [alias-reaches] That `declare_alias` reaches the code path the alias case
   assumes.
   - evidence: a lane traced it: the helper rewrites `docs/roadmaps/roadmap.yaml`
     and COMMITS it, and gate A reads the registry at HEAD, so the row is the one
     `dogfood_resolve_id` consults. The case at
     `tests/falsification_gate_a_the_pr_and_its_commits_name_one_ticket.rs:141`
     therefore passes for the reason it claims rather than by accident.

4. [table] That the log's 30-PR table sums to what the receipt says.
   - evidence: a lane re-derived it row by row with `awk`/`sort`/`uniq -c`: 30
     rows, no duplicates, no blanks, 29 `ok` and 1 `MISMATCH`. It separately
     confirmed `b4719737` is #532's merge commit, that git's parser returns
     nothing for it and `sed` returns PMAT-531, and that PMAT-219's row declares
     `alias:PMAT-218` while #496's branch says PMAT-218.

5. [receipt-shape] That the receipt satisfies gate A's own shape requirement.
   - evidence: measured with `tail -1` and `grep -c '^verdict:'` rather than read
     for: the last line is `IMPL-PMAT-540-RECEIPT-END` and exactly one line
     matches `^verdict:`.

## REFUTED

1. [squash-shape] That "a squash message ends with whatever bullets GitHub
   assembled".
   - evidence: a lane read the real message. GitHub appends its OWN last
     paragraph -- a `---------` separator and a `Co-authored-by:` block -- and
     THAT is what pushes the `Pmat-Ticket:` lines out of git's view.
     `%(trailers)` on `b4719737` returns the `Co-authored-by:` line alone.
   - corrected: the sentence is replaced in the script, the test module, the
     receipt and the log, each now naming the separator and the block.

2. [fixture-shape] That the fixture reproduces the shape that makes git blind.
   - evidence: THE FINDING OF THE ROUND. `squash_claiming` ended at the trailer
     block, so git parsed it happily -- measured by building the message and
     asking `%(trailers:key=Pmat-Ticket,valueonly=true)`, which returned the
     ticket. Every case would therefore have passed over an arm "simplified" to
     use git's parser, which is the one regression the suite exists to prevent,
     and no test could have said so.
   - corrected: the fixture now carries the separator, and two cases pin it at
     `tests/falsification_gate_a_the_pr_and_its_commits_name_one_ticket.rs:247`
     and `:271`, each asserting FIRST that git's parser returns nothing. A fourth
     mutation -- read with git's parser -- now kills three cases; before the fix
     it would have killed none.

3. [count] That the floor comment's arithmetic matches the log.
   - evidence: a lane read `scripts/dogfood/harness.sh` and the commit message of
     f7711482 and found "28 agree, #532 is the mismatch, and #496 LOOKS like
     one", against a log that measures 29 agreeing over 30 rows. #496 agrees
     after alias resolution and belongs in the 29, so 28 + 1 never summed to 30.
   - corrected: the comment now says 29 agree, 1 mismatch and none without a
     trailer, and says that #496 counts among the 29.

4. [log-reference] That §3 of the log shows a floor at main's tip.
   - evidence: a lane opened the section the receipt cited. §3 contrasts the
     shipped floor with a floor this repository does not carry; the five-PR
     figure came from a measurement taken while choosing the floor and is not in
     that section.
   - corrected: the receipt says where the figure came from and stops citing a
     section that shows something else.

5. [promise] That gate A enforces "the PR is filed under a ticket its own merge
   commit CLAIMS", as CLAUDE.md and the skill promised.
   - evidence: a lane read the promise, then found `if [ -n "$resolved" ]`, which
     passes a merge commit claiming nothing at all. The guard is right -- an
     unlabelled commit is the commit-msg hook's finding and CB-2113's -- so the
     promise was the thing that was wrong.
   - corrected: both documents now say the merge commit must not CONTRADICT the
     ticket, the script states the exemption in its own comment, and the receipt
     names it first under "Gaps, named". The case at
     `tests/falsification_gate_a_the_pr_and_its_commits_name_one_ticket.rs:127`
     is what holds it.

6. [unguarded-pipeline] That every command in the arm can fail safely.
   - evidence: a lane found the `claims="$(printf | sed | tr | tr | tr | tr)"`
     assignment with no `|| rc=$?`. Under `set -euo pipefail` a failure anywhere
     in it exits the script with no `GATE A` line at all -- a death rather than a
     verdict, which this file's own header refuses.
   - corrected: guarded, and a failure now fails by name as UNMEASURED.

7. [two-tickets] That the arm closes the misattribution.
   - evidence: a lane constructed the shape that survives it -- a merge commit
     claiming `PMAT-520 PMAT-531` under a PR filed as PMAT-520 passes, because
     the filed id IS among the claims, while PMAT-531's work is still credited to
     the wrong window. It also observed that a PR and its commits agreeing on a
     lie pass: the arm measures agreement, not truth.
   - corrected: both are named in "Gaps, named" rather than implied. Neither is
     fixable by this arm -- a release cut legitimately carries several tickets --
     and saying so is the whole of what can honestly be done here.

## What the round cost and what it bought

Three lanes, one round, about fifteen minutes of agy. It found a fixture that
could not catch the regression its own ticket is about, an unguarded pipeline
that would have killed the gate without a verdict, a documented promise stronger
than the code, an arithmetic slip, and a citation to the wrong log section. The
suite could not have found any of them.
