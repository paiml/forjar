# PMAT-535 — tools, and what they measured

| tool | what it said |
|---|---|
| `cargo test --test falsification_a_branch_names_its_own_ticket` | 8 passed |
| `cargo test --test falsification_quorum_gate_reads_the_pushed_ref` | 5 passed |
| `cargo test --test falsification_quorum_gate_has_a_triage_shape` | 8 passed |
| `cargo test --test falsification_quorum_anchors_release_shaped` | 4 passed |
| `cargo test --test falsification_dogfood_scripts_declare_mutations` | 4 passed |
| `cargo fmt --all -- --check` | exit 0 |
| `bashrs lint scripts/quorum-gate.sh` | 0 errors, 19 warnings, 51 infos |
| `bashrs lint` on `origin/main`'s copy | 0 errors, 11 warnings, 41 infos |
| `bash scripts/quorum-gate.sh` against 7 branch shapes | log §4, §5, §6 |
| `bash -n scripts/quorum-gate.sh` | exit 0, on the fixed script and on the arm-deleted variant |
| `gh issue create` | forjar#540 |

**bashrs exits 1 on BOTH sides**, because it returns non-zero whenever it has
anything to say. The number that carries information is ERRORS, and it is 0 on
both. Of the eighteen findings the arm adds, sixteen are quote-context artifacts
— six SC2086 on variables that are already double-quoted, BRS0023/SC2162/REL003
twice on the English word "read" in prose, SC2230/SC2023 on the English word
"which", SC2016/BRS0006 on `'$branch'` quoted for a reader — and the remaining
two are the standard SC2089 note on a single-quoted `sed` script and `printf`
format, which is what those lines intend. §9 of the log prints each one with the
exact text at the column bashrs named, so the claim is checkable rather than
asserted.

**The SC2101 the first revision argued away is gone**, because the `case`
pattern it flagged is gone: `[ -n "$claimed" ]` replaced `*[![:space:]]*`.

**Tool defect worth naming:** none new. The `git clone --shared` sandbox
problem in the agy note is a procedure defect, not a tool defect.
