# Quorum evidence — PMAT-220 — adjudicated claims

## CONFIRMED

1. [filesystem] A NAMESPACE IS NOT THIS HOST — hashing the controller's path of the same name answers about the wrong filesystem.
- evidence: `PepitaConfig` carries its own `rootfs` and the transport unshares the mount namespace, so the two paths are different files. The site that stated the general rule is src/tripwire/drift/file.rs:251 at the merge base, whose predicate already excluded a namespace for exactly this reason while three other sites did not.

2. [exemption] THE FOURTH SITE IS UNREACHABLE — `exec_script_tracked` returns for pepita and container before it consults the loose predicate.
- evidence: the ordering is asserted, not assumed, at tests/falsification_a_namespace_is_not_the_controller.rs:196, a file this branch adds, and the assertion requires each predicate to sit in a guard whose body returns rather than merely appearing earlier in the text. All three lanes checked the reasoning and one established it by measurement.

3. [behaviour] THE BUG WAS OBSERVABLE — output verification reported a namespaced machine's artifacts as missing because it looked for them here.
- evidence: the gate it exercises is the predicate at src/core/executor/output_verify.rs:54, which resolves at the merge base as the loose one. The table-driven case covers the namespace row and both guard rows in one table so a fix cannot trade one for another. It fails with the loose predicate restored and passes with the strict one, and the two guards move under neither.

## REFUTED

4. [vacuity] THE RULE NAMED THREE FILES — a fourth controller-side read added anywhere else would have stayed green.
- corrected: the sweep is all of `src/` now, with one file exempt by name and a case that fails if that exemption stops being needed. Measured: the same call added to `src/core/state/reconstruct.rs` is reported by name.

5. [vacuity] IT READ ITS OWN EXPLANATION, TWICE OVER — comment stripping covered whole lines only, and the ordering check compared string positions.
- corrected: whole-line, trailing and block comments are all stripped, and each predicate must sit in a guard whose body returns. A trailing-comment fake fix is proven not to pass. This is the third instance of the shape in this repository, which is why the first half was caught within one run and recorded rather than quietly fixed.

6. [downstream] A MISSING PROBE READS AS UNCHANGED — and two lanes called that a regression this ticket introduced.
- corrected: it is older than this ticket. `src/core/planner/mod.rs` falls through to a config-hash comparison when `probes.get` returns `None`, which has been true for every non-local machine since the probe existed; every SSH target already behaves this way. The third lane said so and was right, and settling the disagreement is what located it. Filed as forjar#497. This ticket's trade — a silent gap in place of a confident wrong answer about the wrong filesystem — is deliberate and named rather than absorbed.
