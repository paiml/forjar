# PMAT-240 — adjudicated claims

One round of three sandboxed agy quorum lanes: 3/3 FAIL. Six refutations, every
one re-measured before it was acted on and every one reproduced.

This is the round that most earned its cost. The change was fifteen mechanical
shell edits and one rule; the edits were fine and **the rule had four holes**,
each openable with one line of shell.

## CONFIRMED

1. [count] That the receipt's numbers are right — the census listed twelve, the
   rule found three more, fifteen in total.
   - evidence: a lane re-derived the count against `origin/main` independently
     and measured exactly 15, naming the same files. The three the census had
     never listed are `crux-reconcile.sh:74`, `publish-from-tag.sh:88` and
     `release-check.sh:195`, each a three-stage pipeline whose fatal end is not
     the stage after the first pipe.

2. [dependencies] That the `awk` reads what the `sed` did when a
   `[dependencies]` section carries its own `version = "…"`.
   - evidence: a lane built that manifest and ran both. They read the SAME
     WRONG THING — the dependency's version, because neither is section-aware.
     That is a pre-existing property of the reader rather than something this
     change introduced, and it is worth knowing: the cookbook arm's own parser
     was made section-aware for exactly this reason in PMAT-241, and these
     three readers were not.

## REFUTED

1. [strings-hide] That a `#` in a line is a comment.
   - evidence: `cat f | grep " # " | head -1` — the `#` is inside a string, and
     cutting the line there took the `head` stage with it. The pipeline
     vanished from the rule entirely. One line of shell.
   - corrected: the line is walked character by character; a `#` ends it only
     outside quotes, and
     tests/falsification_no_script_pipes_into_an_early_exit.rs:1 carries the
     shape as a case.

2. [strings-fire] That a `|` in a line is a pipe.
   - evidence: `echo " | head "` was reported as a fatal pipeline. A rule that
     fires on text which runs nothing is a rule people learn to ignore, and
     this one would have fired on any script printing a usage string with a
     pipe in it.
   - corrected: only a `|` outside quotes splits a stage, and
     tests/falsification_no_script_pipes_into_an_early_exit.rs:1 asserts this
     exact line is NOT reported.

3. [continuation] That a pipeline is one line.
   - evidence: `cat f \` on one line and `| head -1` on the next is one
     pipeline written on two, and the rule saw two lines with no pipe between
     them.
   - corrected: continuations are joined before any line is analysed, driven at
     tests/falsification_no_script_pipes_into_an_early_exit.rs:1.

4. [or-operator] That `||` is two pipes.
   - evidence: found while fixing the above. `cmd || head -1` split into three
     stages with an empty one between, and the `head` after it read as a
     right-hand side. It is a FALLBACK command: nothing pipes into it.
   - corrected: `||` is consumed whole, so the command after it is never a
     stage; it is one of the four safe shapes
     tests/falsification_no_script_pipes_into_an_early_exit.rs:1 asserts stay
     safe.

5. [vacuous] That the library case proves the library is walked.
   - evidence: the planted pipeline went at the END of the file, where a rule
     that read only a file's last line would pass. The case proved almost
     nothing.
   - corrected: planted in the MIDDLE, and the case asserts the reported LINE
     NUMBER — which only a rule that actually walks the file can get right.

6. [preservation] That every rewrite preserves behaviour.
   - evidence: measured by a lane and reproduced here. The old
     `sed -n 's/^version = "\(.*\)"$/\1/p'` anchors the closing quote at end of
     line, so `version = "1.29.0" # a trailing comment` — valid TOML — yields
     NOTHING and the script fails with "cannot read version from Cargo.toml".
     The awk stops at the closing quote and reads `1.29.0`.
   - corrected: not the code — the awk's behaviour is the better one — but the
     CLAIM. All three scripts carrying the change now say it is a change and
     why, and the receipt names it as an unpinned gap rather than filing it
     under "preserves behaviour".

7. [log] That six gates passed "either way".
   - evidence: refuted by reading the log rather than the sentence. The gates
     were run at HEAD only; the log holds one run. **Third time in this release
     a receipt has been caught describing a file that did not contain what it
     claimed**, after a 9-line proof log in PMAT-537 and a truncated Gate R
     quote in PMAT-533.
   - corrected: the sentence says what was run, and says explicitly that the
     gates were not re-run against main's scripts.
