# PMAT-522 — why there was no lane round

Three sandboxed agy quorum lanes have reviewed every other ticket in this
window. This one had none, deliberately, and the reason is worth stating
rather than leaving as an absence:

- The defect was **already measured**, by the operator, on the machine it
  happened to, with numbers no lane could have produced from a diff: 9,740
  processes, 5,781 copies of the script, load 3,026, CPU pressure 83%.
- It was a **live hazard on a shared machine**. The box was recovered by hand
  before this session knew anything had happened. Dispatching three lanes —
  each of which spawns processes — onto the same machine to review a fork-storm
  fix is a poor use of the twenty minutes immediately after one.
- The fix is **four assertions against observable behaviour**, each of which
  runs the real script on the real machine. There is no design question for a
  lane to attack that the cases do not already answer.

What a round would have added is an adversary looking for the shape the author
did not consider. That role is filled here by the operator, who found the whole
thing; and by the judges digest, which records this session's own three failed
attempts at the cap rather than only the one that worked.

The gap is named in the receipt: pmat's own missing recursion guard is upstream
and unreviewed here, and `ulimit -u` remains per-user rather than per-tree.
