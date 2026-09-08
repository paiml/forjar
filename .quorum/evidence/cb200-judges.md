# Quorum digest — PMAT-206 — adjudicated claims

Thirteen ids: seven claims confirmed by the second round or by measurement, and six rulings from the two rounds that were acted on or set aside. Every item names the guard in this diff that pins it.

## CONFIRMED

1. [measured] C1 — `observe::classify` is a static table of 32 (field, observability) pairs and an unknown field still falls through to `classify_e01::classify`; the three q2 lanes diffed the old arms against the table and confirmed every alt and reason string unchanged.
- evidence: src/core/observe/mod.rs:70 holds the table and src/core/observe/mod.rs:55 the lookup with `or_else(|| classify_e01::classify(field))`; the orchestrator's own diff of the 32 pairs against the match found only trailing-comma differences.

2. [measured] C2 — The purifier decompositions keep every pinned verdict: the two falsification suites (9 and 15 tests) pass at HEAD, and one lane's narrowing — that an escaped-quote line now passes where it once failed — is the PMAT-204 fix already on main, not this diff.
- evidence: src/core/purifier_sec017.rs:191, src/core/purifier_sec017.rs:164, src/core/purifier_sec017.rs:293 and src/core/purifier_sec017.rs:391 are the decomposed sites; the suites that pin them were re-run after each edit.

3. [measured] C3 — The example asserts all fourteen criteria it asserted before, with the same labels, through one `criterion` helper; three lanes read every assertion and the example runs to its final line.
- evidence: the helper prints the verdict then asserts with the label; `cargo run --example cron_secret_encryption_falsification` ends with the survival line; the decomposed sites are pinned by src/core/purifier_sec017.rs:164 in the same spirit.

4. [design] C4 — The cache removal cannot leave comply's cache root: the glob is anchored to `$cache_root/<basename>-*`, a case pins the match under the root, `-d` requires a directory, and `rm -rf "${dir:?}"` refuses an empty variable; three lanes tried a hostile basename and environment and confirmed.
- evidence: the guarded site in scripts/cb200-ratchet.sh follows the same `:?` discipline as scripts/publish-from-tag.sh; the Rust surface it protects is measured at src/core/observe/mod.rs:55.

5. [design] C5 — The ratchet never swallows a measurement: an empty comply result is UNMEASURED and exit 1, a stale cache is announced as a NOTE and removed rather than silently refreshed, and the probe run with its `|| true` from the first rewrite is gone.
- evidence: observed twice on this branch — cache fresh: exit 0 at 651; source newer than the cache: NOTE printed, cache removed, exit 0 at 651; the number it measures is the grade of sites such as src/core/purifier_sec017.rs:191.

6. [measured] C6 — With a fresh comply index the branch measures exactly the recorded ceiling, 651, and the ceiling in scripts/ratchets/cb200-baseline.json is unchanged; three lanes confirmed the number and the file.
- evidence: the measurement table in the claims dossier (654 → 653 → 652 → 651 as the cache was refreshed); the sites that moved it are src/core/observe/mod.rs:70 and src/core/purifier_sec017.rs:293.

7. [scope] C7 — Nothing in the diff is outside the ticket's two acceptance criteria once the offender dump was removed; the lanes confirmed the remaining files each serve one of the two criteria.
- evidence: four files change — the observe table at src/core/observe/mod.rs:70, the purifier decompositions at src/core/purifier_sec017.rs:191, the example, the ratchet and its baseline; the roadmap row is the ticket itself.

## REFUTED

8. [q1] R1 — the first rewrite parsed the cache path out of comply's JSON with an unanchored regex and handed it to `rm -rf`; four lanes found that the JSON carries source snippets and file paths from violations, so the regex could match a path outside the cache.
- corrected: the directory is found by name under the cache root and never read from JSON; the guard discipline mirrors the sites this file protects, such as src/core/purifier_sec017.rs:391.

9. [q1] R2 — the first rewrite ran comply twice and put `|| true` on the probe run, so a failed probe silently fell through to a stale measurement.
- corrected: the probe run is gone; one comply run, its absence UNMEASURED; the measured grades at src/core/purifier_sec017.rs:367 and siblings are what it reports.

10. [q1] R3 — the offender dump to target/cb200-offenders.txt was scope the ticket did not ask for.
- corrected: removed; with the cache fixed the number is actionable on its own, as the movement of src/core/observe/mod.rs:70 out of the top ten showed.

11. [q1] R4 — the baseline's new `why` entry said a 34-arm match was replaced; the match had 32 field arms and a fallthrough (one lane measured it).
- corrected: the entry now says 32 field arms and a fallthrough, which is what src/core/observe/mod.rs:55 replaced.

12. [q2] R5 — the claim 'no `|| true` on `pmat comply check`' is false as written: the measuring run has carried `|| true` since before this branch, because comply exits 1 whenever CB-200 reports — which it is expected to — and the JSON must still be captured; the very next line refuses an empty result as UNMEASURED.
- corrected: the claim is reworded to what the script does (C5); the script is unchanged there and the refusal path is the one that judges sites like src/core/purifier_sec017.rs:309.

13. [q2] R6 — the baseline's `why` entry still said the cache entry was read from comply's own message — the withdrawn first rewrite — while the shipped script finds it by name (one lane).
- corrected: the entry now describes the shipped rule and names the quorum finding that changed it; the reductions it records are the ones at src/core/observe/mod.rs:70 and src/core/purifier_sec017.rs:293.

