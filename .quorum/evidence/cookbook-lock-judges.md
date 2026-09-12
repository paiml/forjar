# PMAT-537 — adjudicated claims

One round of three sandboxed agy quorum lanes: 3/3 FAIL. **All six numbered
claims were confirmed by all three lanes** — the first time in this release run
— and all three still refused, on the standing instruction.

That is the instruction earning its place for the fourth round running. A change
can be right in every respect a reviewer was asked about and still ship a false
sentence about itself.

## CONFIRMED

1. [parser] That the lock parser reads forjar's OWN version in every shape, or
   refuses.
   - evidence: attacked by all three with `forjar-something` as a package name,
     a `version` line preceding its `name`, forjar appearing inside another
     package's dependencies list before its own entry, CRLF, and two forjar
     entries. The `awk` walk keys on an exact `name = "forjar"` line and resets
     on each `[[package]]`, so a prefix match and a dependencies mention are
     both ignored and the first true entry wins.
   - evidence: driven from the other side at
     tests/falsification_release_cookbook_is_part_of_the_release.rs:1, where a
     `serde` entry pinned `9.9.9` precedes forjar's and the gate still reads
     forjar's.

2. [decision] That requiring the lock to pin the released version EXACTLY is
   the right reading, and that its cost is stated where a reader finds it.
   - evidence: all three weighed the alternatives the ticket named — a ratchet
     on the lock, or the requirement alone — and none preferred them. The cost
     is real and is in the arm's own comment and in the skill: a cut now waits
     on a second repository, and a patch release the cookbook has not been
     bumped to will make the cut red.
   - evidence: that cost is the point rather than a side effect. The weakest
     reading is what produced twenty-seven releases of a link pointing at
     something never built against them.

3. [fail-closed] That every unmeasurable direction fails closed and says which
   it is.
   - evidence: a missing lock, an unreadable one and one with no forjar entry
     each produce a named UNMEASURED failure rather than a pass. All three
     lanes looked for an input that slips through and none found one.

4. [cases] That the five new cases falsify, and that the stub's two-file
   dispatch cannot make a case measure the wrong file.
   - evidence: the stub answers `*Cargo.lock*` and everything else separately,
     so a lock case cannot be reading the manifest — which is precisely the
     confusion this ticket is about, and would have been invisible if the stub
     had answered both with one body.
     tests/falsification_release_cookbook_is_part_of_the_release.rs:1.

5. [bump] That the cookbook bump is real.
   - evidence: verified from GitHub by all three. paiml/forjar-cookbook#20
     merged as `0be3e1ec`; its `Cargo.lock` pins forjar 1.29.0 and its
     `Cargo.toml` requires `1.29`. Nothing else changed — the diff is those two
     files.

6. [ordering] That re-pointing v1.29.0's row after the tag is defensible rather
   than rewriting history.
   - evidence: all three accepted the argument and none called it rewriting.
     The old row recorded a qualification that never happened; the new one
     records one that did, and the receipt says plainly that the qualification
     is later than the release it describes and that this is true of one
     release only.

## REFUTED

1. [log] That `docs/audits/logs/PMAT-537-cookbook-lock.log` contains the red
   and green gate outputs the receipt quotes from it.
   - evidence: refuted 3/3, and they read the FILE rather than the claim. It
     was **9 lines** and contained neither output — it ended mid-sentence at
     `(gate T reads the ledger at HEAD, so the red half is the commit before
     this one)`.
   - evidence: the cause is `tee "$L" | head -14` in the script that wrote it.
     `head` closes the pipe after fourteen lines, `tee` takes SIGPIPE, and the
     file stops where the terminal output did. That is the SIGPIPE class this
     repository has a rule about and an open ticket for (PMAT-240), in a proof
     script, written by the person who wrote the rule.
   - corrected: regenerated with no pipe — 459 lines carrying both gate
     verdicts and the ten cases — and the red half now runs in a scratch clone,
     because gate T reads the ledger at HEAD and committing the defect in this
     working tree has disturbed it four times in this session.
