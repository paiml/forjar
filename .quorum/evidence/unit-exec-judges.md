# PMAT-560 — adjudicated claims

Two rounds of three sandboxed agy quorum lanes; each round voided one lane on
an API 503. Six confirmations and six refutations, every one re-measured on
this host before it was acted on. The mechanism held — the manager is asked,
the live file is hashed — and round 1 found the one input that made it lie.

## CONFIRMED

1. [reads-the-manager] That the probe reads the loaded unit from systemd, not
   the unit file forjar wrote, and hashes the file at the LIVE path (both
   rounds, all counted lanes).
   - evidence: `src/resources/service_exec.rs:51` builds the probe:
     `systemctl show -p ExecStart --value` into the live-path variable, then
     `sha256sum` over that variable at `src/resources/service_exec.rs:56`.
     Mutation MB re-hashes the declared path instead and
     `tests/falsification_unit_exec_parity.rs:156` goes red, which is the
     yoga shape: the declared path held the right bytes and the unit ran
     another file.

2. [no-silent-abort] That no input under `set -euo pipefail` aborts the apply
   with no marker, and none exits 0 over the wrong program once the separator
   is right (round 1 lane 3, round 2 both lanes).
   - evidence: both substitutions end `|| echo` / `|| echo missing` inside
     brace groups, so an unloaded unit or a failed `systemctl show` becomes an
     empty path that diverges; `tests/falsification_unit_exec_parity.rs:204`
     pins the unloaded case and the apply tail at
     `src/resources/service_exec.rs:132` exits 1 on any raised flag.

3. [undeclared-unchanged] That a service declaring neither field emits
   nothing new in check, apply or state query (all counted lanes).
   - evidence: every fragment returns empty when `is_declared()` is false;
     `tests/falsification_unit_exec_parity.rs:215` asserts no `ExecStart` in
     check or query and no `exec_` line in the query output; mutation MC kills
     exactly that case.

4. [upgrade-note] That the CHANGELOG's one-time re-converge note is accurate
   (round 1 lanes 1 and 3, cited against the hasher's header).
   - evidence: `CHANGELOG.md:34` says what `planner::hashing` documents for
     any new `Resource` field — every resource replans `Update` once, every
     `state: absent` resource `Destroy` once — and the golden hash was repinned
     at `src/core/planner/tests_hash.rs:150` rather than the test deleted.

5. [apply-fails] That `apply` fails the resource rather than reporting a
   started-and-enabled unit converged over the wrong program (round 2, both).
   - evidence: `src/resources/service.rs:126` extends the apply with the
     tail; `tests/falsification_unit_exec_parity.rs:236` runs it against the
     fake host both ways; mutation MD kills it.

6. [lifecycle-move] That moving `LifecycleRules` out of `resource.rs` kept
   every use and re-export (round 2, both lanes built it).
   - evidence: `cargo check --all-targets` clean; the move kept
     `resource.rs` under the 500-line gate after the flattened field landed.

## REFUTED

1. [space-split] That the awk read the whole program path (this author, in
   the first cut).
   - corrected: round 1 lane 1 measured `split($2, a, /[ ;]/)` reading
     `/opt/my script` as `/opt/my`, and the voided lane 2 found the same. The
     worse shape was reproduced here: declared `<dir>/run.sh`, unit running
     `<dir>/run.sh evil` — the split made them equal and hashed the declared
     file, exit 0 over the wrong program. The separator is systemd's own
     ` ; ` at `src/resources/service_exec.rs:55`, pinned by
     `tests/falsification_unit_exec_parity.rs:296` and `:315`; mutation ME
     kills both.

2. [marker-seen] That the divergence marker tells the operator what runs
   (this author).
   - corrected: `cli::check` prints a failing script's STDERR under `exit 1`,
     and the marker went to stdout only, so the operator saw `exit 1` and
     nothing else. The divergent branch echoes to both, built once at
     `src/resources/service_exec.rs:67`; pinned by
     `tests/falsification_unit_exec_parity.rs:334`, killed by MF.

3. [falsify-005-006] That FALSIFY-EXEC-005 named a real mutation and
   FALSIFY-EXEC-006 tested its prediction (this author, in the contract).
   - corrected: 005 described reading an exit code the shell never reads —
     reworded as the mutation it is; 006 predicted runtime divergence and
     cited only the parse-time refusal — the runtime half is now
     `tests/falsification_unit_exec_parity.rs:358`.

4. [converged-def] That the contract's `converged` was "every declared
   obligation holds" and that the check reports "N obligations, 0 failed"
   (this author; found by the voided round-1 lane, re-read here).
   - corrected: presence is part of convergence and the check reports
     markers and an exit code, not a count; both sentences were rewritten to
     say what the code does.

5. [book-marker] That the book's "the marker names what the unit runs" held
   for the digest marker, and that validate refuses ANY non-hex digest (this
   author; voided lane 2 in round 1, lane 1 in round 2 for the CHANGELOG).
   - corrected: the digest marker names the live digest, and a `{{…}}`
     template is left to the resolver; `CHANGELOG.md:10` and the book now say
     both.

6. [separator-limit] That no program path defeats the parse (this author,
   after the first fix).
   - corrected: round 2 lane 1 — a path containing ` ; ` itself is misread.
     It reads as divergent, the failing direction, and is now the stated
     limit in the book and in the probe's doc comment rather than an
     unstated one.
