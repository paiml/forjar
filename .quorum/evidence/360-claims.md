# Quorum evidence — #360 / #362 (fix/drift-observables) — adjudicated claims

## CONFIRMED — 6 claims survived refutation

1. [probe] (explains-symptom) `lifecycle.ignore_drift: ["mode"]` suppressed the WHOLE observation, not one field: the observed state was one digest over the state-query stdout, so the only thing the field list could do was switch the comparison off.
   - evidence: `src/core/executor/resource_ops.rs:47` (the apply baseline), `src/tripwire/drift/mod.rs:98` (the comparison) and `src/cli/apply_variants.rs:195` (`--refresh`'s re-baseline) at the merge-base all hashed `stdout` verbatim; `src/core/parser/validation.rs:137` accepted the field list without honouring it. The generators already emit field-shaped `key=value` output, so the named tokens are dropped BEFORE hashing at all three writers (`core::observation_mask::masked_for`) — no lock schema change. Pinned by `tests/falsification_ignore_drift_names_one_field.rs` (mode ignored, content/owner/existence still watched) and `tests/falsification_ignore_drift_is_not_an_off_switch.rs`.

2. [design] The mask is applied at EVERY writer of the observed digest, and the mask a baseline was taken under is recorded, so a stale unmasked baseline is censused (`ObservationMaskChanged`), never reported as drift on the ignored field.
   - evidence: masking only the apply baseline and the comparison would let one `--refresh` write an unmasked digest and the next `drift` report false drift on exactly the ignored field — which, since #307, blocks the apply that would fix it. `record_success` records `mask_key`; `check_nonfile_drift` censuses when the recorded mask differs from the config's. The agy lane attacked every writer and the census path and refuted its own attacks (`src/core/observation_mask.rs`, `src/tripwire/drift/mod.rs`).

3. [design] The vocabulary starts narrow and `content` stays a hard error.
   - evidence: `masked_for` drops `key=value` TOKENS; a file's content hash is a bare line with no `=`, as is the `MISSING` sentinel, so a "drop the line without an `=`" rule could not tell them apart and would erode existence detection while claiming to ignore content. `ignore_drift: ["content"]` is refused by validation until the generator carries an explicit existence marker. Attacks on prefix names (`mode` vs `mode_bits`), values containing `=`, and non-`key=value` output were refuted by the lane with file:line evidence.

4. [probe] (explains-symptom) cron drift detection could not see the job: the two `grep -v`s dropped only the two COMMENT lines, so the entry survived every re-apply and a fresh copy was appended below it; the marker match was a substring match, so applying `backup` orphaned `backup-db`.
   - evidence: `src/resources/cron.rs:71` (the `grep -v` pair) and `:50` (the substring existence check) at the merge-base. One `awk` deletes the intact block — marker, cmd-marker, entry — exact-line, with the markers travelling through `ENVIRON` (not `awk -v`, which backslash-processes its value); the existence check is `grep -qFx`. Pinned by `src/resources/tests_cron_exec.rs` (a fake crontab store the generated script really writes to) and the updated `tests_cron_b.rs` expectations.

5. [design] Only an INTACT block is deleted; forjar deletes nothing it did not write.
   - evidence: the entry line is dropped only when it follows the marker AND the cmd-marker in order, so a crontab a human has already edited by hand loses its markers and nothing else. The lane confirmed the consequence — the hand-edited entry stays and a fresh block is appended (two jobs) — which is the pre-existing behaviour too; refusing by name in that case is filed as #445 rather than deleting text forjar did not write.

6. [design] The falsifiers cannot pass vacuously.
   - evidence: the cron cases execute the generated bash against a fake crontab store and read it back; the mask cases tamper the managed file and assert `drift_count >= 1` on the still-watched field while the ignored field's change is censused, not reported; `tests_cron_b.rs` edits are the new `awk` variable names, not expectations (checked by the lane).

## REFUTED — 3 claims killed

1. [design] refuted 1/1 — "Make `ResourceLock.observed` a per-field map (the issue's proposal)."
   - corrected: a lock schema change, a migration, and a change to every state-query generator, for a result the field-shaped stdout already gives; `cli::lock_core` carries two hard-coded schema checks the bump would trip. The mask over the stdout honours the field list with the observation still a digest.

2. [design] refuted 0/1 (agy lane's unique finding, countered) — "The branch drops the `ResourceType::File` exclusion from the non-file walk, so files are reported twice."
   - corrected: the branch's diff removes exactly one line from `src/tripwire/drift/mod.rs` (the unmasked digest, `:98` at the merge-base) and the exclusion at `src/tripwire/drift/mod.rs:75` is untouched; the lane read main's own comment about an earlier `|| ResourceType::File` change as this branch's. Verified by the diff, and the rebuilt branch's drift suite reports one finding per tampered file.

3. [design] refuted 1/1 (agy lane's verdict) — "String masking and awk are below the industry default; a structured parser is required."
   - corrected in part: Ansible and Puppet parse crontabs structurally and Terraform's `ignore_changes` scopes a structured attribute; forjar's observation is a shell transcript by design (bare-metal first), so the honest form is the exact-line block and the token mask, each pinned; the structured route is #445's follow-up territory, not this ticket's.
