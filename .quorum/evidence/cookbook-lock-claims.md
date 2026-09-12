# PMAT-537 — the claims put to the lanes

Six claims plus the standing instruction, one round of three sandboxed lanes.
**All six claims CONFIRMED by all three lanes** — the first round in this
release run where the numbered claims all held — and all three still returned
FAIL, on the instruction.

1. **The lock parser reads forjar's own version in every shape**, or refuses.
   Attacked with `forjar-something`, a `version` before its `name`, forjar
   appearing in a dependencies list first, CRLF, and two forjar entries.
   CONFIRMED 3/3.

2. **The three-readings decision is sound and honestly stated** — the lock must
   pin the released version exactly, and the cost is named where a reader finds
   it. CONFIRMED 3/3.

3. **Every unmeasurable direction fails closed and says which** — a missing
   lock, an unreadable one, one with no forjar. CONFIRMED 3/3.

4. **The five new cases falsify**, and the stub's two-file dispatch cannot make
   a case measure the wrong file.
   `tests/falsification_release_cookbook_is_part_of_the_release.rs:1`.
   CONFIRMED 3/3.

5. **The cookbook bump is real** — paiml/forjar-cookbook#20 merged as
   `0be3e1ec`, whose `Cargo.lock` pins forjar 1.29.0 and whose `Cargo.toml`
   requires `1.29`, with nothing else changed. CONFIRMED 3/3 from GitHub.

6. **Re-pointing v1.29.0's row after the tag is defensible**, not history
   rewriting. CONFIRMED 3/3.

**And the instruction found the one defect**: the receipt described a log file
that did not contain what it said. REFUTED 3/3.

The adjudicated tally in `cookbook-lock-judges.md` is **6 CONFIRMED, 1
REFUTED**.
