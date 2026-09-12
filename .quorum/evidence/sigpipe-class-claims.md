# PMAT-240 — the claims put to the lanes

Six claims plus the standing instruction, one round of three sandboxed lanes,
3/3 FAIL. Five of the six refuted, and every refutation reproduced.

1. **Every rewrite preserves behaviour.** REFUTED — the `sed` → `awk` version
   readers do not, measurably.
2. **The `awk` reads what the `sed` did**, tested against a `[dependencies]`
   section carrying its own `version = "..."`. CONFIRMED — both read the same
   wrong thing there, which is a pre-existing property rather than a new one.
3. **The rule cannot be walked out of.** REFUTED, three ways, each one line of
   shell.
   `tests/falsification_no_script_pipes_into_an_early_exit.rs:1`.
4. **The rule does not fire on safe shapes.** REFUTED — a quoted pipe, and
   `cmd || head -1`.
   `tests/falsification_no_script_pipes_into_an_early_exit.rs:1`.
5. **The two cases are not vacuous.** REFUTED — the planted pipeline was at the
   END of the file.
6. **The receipt's numbers are right.** CONFIRMED — a lane re-derived 15
   against `origin/main` independently.

**And the instruction**: REFUTED — the receipt claimed six gates passed "either
way" over a log holding one run.

The adjudicated tally in `sigpipe-class-judges.md` is **2 CONFIRMED, 7
REFUTED** — the seven include `||`, which was found while fixing the other
three rather than put to a lane, and is adjudicated here because it was a hole.
