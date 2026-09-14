# PMAT-520 — the claims put to the lanes

Six claims plus one standing instruction, one round of three sandboxed lanes,
all three FAIL. Every claim is anchored in a file at the branch tip.

The standing instruction is the one that earned the round: **find a sentence in
the CHANGELOG, the crux audit or the dogfood receipt that a reader could check
and find false.** Four of the five findings came from it rather than from the
numbered claims.

1. **The CHANGELOG describes what shipped**, bullet by bullet against
   `git log v1.28.0..HEAD`. REFUTED 3/3 on the counts: thirteen PRs and sixteen
   tickets claimed, twelve and fifteen merged.
   `tests/falsification_crux_gate_reads_the_release_section.rs:1` drives the
   gate that reads this section.

2. **The numbers are real** — 9,740 processes and load 3,026; 226 processes and
   2,352 threads; the five ceilings; 98 cookbook configs; 96.43% coverage; ten
   tags since the cookbook's master. REFUTED 2/3, on the CB-21xx figures
   reading as ceilings rather than finding counts.

3. **The crux audit's rows are honest**, and the four WORSE rows are the right
   four. CONFIRMED 2/3; REFUTED 1/3 on row 1's claim that gate T "refuses the
   tag".

4. **The dogfood receipt's verdict is supported**, including its roster-
   instability finding. CONFIRMED 2/3; the third's refutation is arithmetic
   that does not reproduce.

5. **Nothing is missing that a cut needs.** REFUTED 3/3 — `README.md:96` still
   read `forjar = "1.28"`.

6. **The falsification is real** — the crux document as the reverted hunk.
   REFUTED 3/3, and the refutation is about the BRIEF: the lanes were told the
   receipt "will name" it, and looked for it in the dogfood receipt, which is
   not where it lives.
   `tests/falsification_crux_gate_reads_the_release_section.rs:1`.

The adjudicated tally in `release-1.29.0-judges.md` is **2 CONFIRMED, 5
REFUTED**, counting a claim once rather than once per lane.
