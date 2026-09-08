# Quorum evidence — PMAT-206 — refuter rulings

Round q1 (three blind claim lanes on the first ratchet rewrite, all FAIL) and round q2 (three refuter-judges on the corrected commit, all FAIL on one claim's wording). Every finding is quoted as returned; what the orchestrator did with each is in the judges digest.

## q1 lane 1 (FAIL)

The diff fails the acceptance criteria. It introduces a path traversal vulnerability in the cache deletion logic where the unanchored regex can match `..` and delete a parent directory. It also weakens the gate by adding `|| true` to a measuring command (`probe`), decomposes a function (`world_writable_modes_in_segment`) without documenting it in the baseline JSON or requesting it, and adds an unrequested Python reporting feature to the ratchet script.

## q1 lane 2 (FAIL)

The `classify` table mapping matches byte-for-byte with fallthrough intact, and the `criterion` helper perfectly preserves assertions. However, the ratchet script introduces multiple defects. The index deletion logic adds `|| true` to a measuring command, silently skipping deletion if the parse fails and thereby weakening the gate by grading against a stale cache. It also adds an unrequested Python script for offender dumping, while the baseline JSON wrongly claims a 34-arm match instead of 33.

## q1 lane 3 (FAIL)

FAIL. The `classify` table mapping and example assertions are perfectly preserved, but the new measurement fix introduces critical defects. The `probe` command is a measuring command that uses `|| true` and silently fails on crashes; the `grep` regex safely matches `*` but will match `..` (deleting a parent directory) or any project source file path containing `/comply/index/` (deleting user code). The diff also adds unauthorized Python JSON parsing to track offenders.

## q2 lane 1 (FAIL)

Reviewed the PMAT-206 clone-1 diff according to the constraints. Claims R1, R2, R3, R4, R6, and R7 are CONFIRMED as they preserve behaviour, correctly anchor paths, and meet the ticket's criteria. Claim R5 is REFUTED because the script factually contains `|| true` on the `pmat comply check` command, rendering the specific textual claim false. Because R5 is refuted, the overall verdict is FAIL.

## q2 lane 2 (FAIL)

The diff preserves behaviour in the refactors (R1-R3) and implements safe cache removal (R4), but R5 and R6 are refuted. R5 is refuted because `pmat comply check` explicitly uses `|| true` on line 73. R6 is refuted because the new `why` entry falsely claims the script reads the cache entry from comply's message, whereas the script explicitly avoids doing so. Therefore, the verdict is FAIL.

## q2 lane 3 (FAIL)

R1 is CONFIRMED: The table exactly matches the 32 old arms and the fallthrough.
R2 is CONFIRMED: All assertions are preserved and routed through the new `criterion` helper.
R3 is CONFIRMED-AS-NARROWED: Verdict differs for `echo \"x\" ; chmod '0644' '/opt/app666/t'` (the fix in quote escaping allows it to pass where it previously failed).
R4 is CONFIRMED: Parameter expansion `:-` prevents an empty `$PMAT_COMPLY_CACHE` from resolving to `/`, and the glob/case safely pin the path.
R5 is REFUTED: Line 73 in `scripts/cb200-ratchet.sh` contains `|| true` (`raw=$(pmat comply check --format json 2>/dev/null || true)`), which violates the claim.
R6 is CONFIRMED: Ceiling is exactly 651 and `why` matches the diff.
R7 is CONFIRMED: The diff only touches files relevant to these criteria.

Verdict is FAIL due to R5 being refuted.

