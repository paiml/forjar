//! Mutation table for the quorum-gate canary branch falsifiers (split from the root file to stay under the file-health limit).
#![allow(dead_code)]
use super::*;

/// (name, text in the gate, its mutation, the probe it must flip). A probe runs
/// the (mutated) gate and says what the answer was designed to be.
type Probe = fn(&Path) -> (Run, Option<&'static str>);
pub const MUTANTS: [(&str, &str, &str, Probe); 9] = [
    (
        "read a silent python3 as no canary",
        "if [ \"$has_canary_rc\" -ne 0 ] || { [ \"$has_canary\" != \"yes\" ] && [ \"$has_canary\" != \"no\" ]; }; then",
        "if false; then",
        |g| {
            let (_bin, path) = python_shim(0);
            let (r, _) = case_run(&CASES[4], g, Some(&path));
            (r, Some("the canary detection could not run"))
        },
    ),
    (
        "skip the canary check when refuted >= 1",
        "if \"canary\" in q:",
        "if refuted == 0 and \"canary\" in q:",
        |g| case_run(&CASES[0], g, None),
    ),
    (
        "let a canary block pass the waiver check",
        "0) die \"quorum.canary is present next to a waiver or override key: a waived round is not a canary round\" ;;",
        "0) : ;;",
        |g| case_run(&CASES[4], g, None),
    ),
    (
        "do not compare reviewed_base with the merge base",
        "if rb != merge_base:",
        "if False:",
        |g| case_run(&CASES[10], g, None),
    ),
    (
        "refuse a regenerated .pmat/ after reviewed_head",
        "startswith((\".quorum/\", \".pmat/\"))",
        "startswith((\".quorum/\",))",
        |g| case_run(&CASES[11], g, None),
    ),
    (
        "ignore a code file added after reviewed_head",
        "if later.returncode != 0 or any(not p.startswith(",
        "if later.returncode != 0 or any(False and not p.startswith(",
        |g| case_run(&CASES[14], g, None),
    ),
    (
        "treat jq exit 2 as no hit",
        "        1) : ;;\n",
        "        1 | 2) : ;;\n",
        |g| jq_fails(2, g),
    ),
    (
        "treat a failing jq as no hit whatever its code",
        "        *) die \"the canary/waiver check could not run (jq exit $canary_waiver_rc): UNMEASURED\" ;;",
        "        *) : ;;",
        |g| jq_fails(5, g),
    ),
    (
        "do not detect the canary block before the jq check",
        "print(\"yes\" if isinstance(q, dict) and \"canary\" in q else \"no\")",
        "print(\"no\")",
        |g| case_run(&CASES[4], g, None),
    ),
];

/// The jq-free path: the mutant that runs jq for every push breaks it.
pub const NO_JQ_MUTANT: (&str, &str, &str) = (
    "run jq for pushes whose receipt has no canary block",
    "if [ \"$has_canary\" = \"yes\" ]; then",
    "if true; then",
);
