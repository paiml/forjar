# Implementation receipt — PMAT-212 — drift -f walked every stack in the state dir and judged a stack it did not declare against the LOCAL filesystem, so one computer's file was reported as another's drift, for ever

verdict: DONE — `drift -f <config>` is scoped to the machines that config declares, `--all-stacks` asks the wide question on purpose, every DRIFTED row names its machine, and the four-case falsification driven through the real binary fails 3 of 4 on main and passes here. Closes #488 and #485.

## Identity

| field | value |
|---|---|
| ticket | PMAT-212 (kind: code) |
| issues | #488 (scoping), #485 (its consequence: a byte-identical file reported DRIFTED) |
| branch | PMAT-212-drift-scopes-to-its-config, merged into release-1.27.0-fleet-p0 |
| base_commit | 19b9c1f1e42ea254b3322e526d451835fb92e480 |

## The two issues are one defect

#485 was filed first and looked like a hashing bug: a `file` resource whose content is byte-identical to its declaration reported DRIFTED with two stable, different blake3 hashes, converged on every apply and was never converged. The operator proved the content side identical (479 = 479 bytes, `diff -q` clean) and stopped there, which was the right place to stop.

#488 was filed next and looked like a scoping annoyance: `drift -f machines/yoga/forjar.yaml` checked 31 stacks and reported gx10's `bashrc` first.

They are the same line of code. `check_machine_drift` resolves a stack with `config.machines.get(name)`. A stack the loaded config does not declare MISSES, falls to the lock-only arm, and that arm has no machine — so it reads the recorded path on whatever box is running the command. The two hashes in #485 differ because they are hashes of two different files on two different computers. `apply -r bashrc` converged the remote one and the next `drift` re-read the local one. For ever, exactly as reported.

## Measured, before and after, through the real binary

A two-stack fleet in a temp state dir: `one.yaml` declares this host, `two.yaml` declares a machine the first config never mentions, both applied, then the second machine's file changed — which on a real fleet is not a change at all, just a different computer's copy of the path.

| command | before | after |
|---|---|---|
| `drift -f one.yaml` | `Checking faraway (1 resources)...` / `DRIFTED: probe-two` / `Drift detected: 1 resource(s)` | `Checking noah-Lambda-Vector` only / `No drift detected.` |
| `drift -f one.yaml --all-stacks` | flag did not exist | `DRIFTED: probe-two on faraway (...)` |
| `drift -f one.yaml -m faraway` | would scan zero machines and report clean | refused by name, listing what the config declares |

## What changed

- The state-dir walk takes the machines the config declares. The stack is never opened, so there is no lock-only arm to fall to. `in_this_run` is the one predicate, and `-m` goes through it too.
- `--all-stacks` restores the aggregate. paiml/infra's nightly tripwire asks the wide question deliberately and diffs against a ledger; it keeps working, as a request rather than a surprise. **This is a default-behaviour change and infra's lane must pass the flag or point `-f` at each machine's config.**
- Every DRIFTED row names its machine. The JSON output already carried it; the text did not, which is why the operator could not attribute the rows after the fact.
- `-m` naming a machine outside the config is a refusal listing what the config declares. Without it, scoping converts that typo into a scan of zero machines reporting "No drift detected" — the same false green the unknown-machine refusal already exists to prevent, reached by a different door.
- `--abort-on-drift` asks the scoped question, because a gate that aborts an apply over another computer's copy of a path is the aborting version of the same defect.

## Falsification

`tests/falsification_drift_scopes_to_the_config_it_was_given.rs`, four cases, driven through `CARGO_BIN_EXE_forjar` because what is under test is what the operator reads. Against `origin/main`: 3 fail, 1 passes. On this branch: 4 pass. The case that passes on both is `a_declared_machine_with_no_state_yet_is_not_an_error`, which pins behaviour the fix must NOT break — scoping must not turn `drift` before a first apply into an error.

## Gaps

- The remote half of #485 was never reproduced over SSH, because SSH is out of scope for this session. What is proven is the mechanism and the local reproduction of it; the operator's fleet measurement supplies the rest.
- #487's second symptom — `drift` skipping resources the lock records as not converged — is NOT fixed here. It is the operator's option (2) and remains open; PMAT-214 took option (1), which the issue says is sufficient on its own.
- #471 and #470 are the same family (selectors that silently do not scope) and are not in this branch.

IMPL-PMAT-212-RECEIPT-END
