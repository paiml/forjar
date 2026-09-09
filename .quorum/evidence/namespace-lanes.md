# Quorum evidence — PMAT-220 — lane rulings

Three sandboxed review lanes on `d14ffaaf...832cde42`, base pinned by SHA. Verdicts 3 of 3 FAIL, `partial=false`, no lane ran cargo.

## The rule was vacuous three ways, and each lane found a different one

- **Lane 1**: it named three files, so a fourth controller-side read added anywhere else stayed green — vacuous against exactly the thing it exists to prevent.
- **Lane 3**: `code_only` stripped whole-line `//` and nothing else, leaving a trailing comment and a `/* */` block as doors.
- **Lane 2**: the ordering check compared string positions, which proves nothing about control flow; binding the names above the early returns would satisfy it while changing what the code does.

All three are fixed. The sweep is the whole `src/` tree with one file exempt by name; all three comment forms are stripped; and each predicate must sit in a guard whose body returns, because a mention is not a dispatch. Measured after: the same call added to `src/core/state/reconstruct.rs` is caught by name, and a trailing-comment fake fix does not pass.

This is the third time this repository has shipped a rule that reads its own explanation. The first two were RULE 8 of the release-workflow gate and the cargo PATH prelude, and they are why the comment-stripping half was recognised within one run here; the other two halves needed review.

## The disagreement that found something older than this ticket

Lanes 1 and 2 called the probe change a regression: excluding a namespace means `probes.get` returns `None`, so the planner skips the staleness check and a namespaced task stops rebuilding when its sources change. Lane 3 answered that the previous behaviour did not detect those changes either, because it hashed the controller.

Lane 3 is right, and settling it is what located the real defect. `src/core/planner/mod.rs` cannot tell "probed, nothing stale" from "never probed": a missing probe falls through to comparing config hashes. That has been true for **every** non-local machine since the probe existed, so every SSH target already behaves this way. This ticket adds namespaces to an existing set rather than creating the behaviour, and the trade is deliberate: a silent gap in place of a confident wrong answer about the wrong filesystem.

Filed as forjar#497, framed as what it is — the "unmeasured reads as clean" shape that the drift census already refuses and the planner does not.

## What all three confirmed

The `exec_script` exemption is sound: the dispatcher returns for pepita and for container before reaching `machine_is_local`, so the missing exclusion decides nothing there, and one lane established it by measurement rather than reading. All three also agreed the strict predicate is right for the three converted sites, that the table-driven output case distinguishes fix from bug with both guard rows passing for the right reasons, and that the diff adds nothing extra — including the deliberately absent comment at the `mod.rs` call site, which a previous review's finding made the right choice.
