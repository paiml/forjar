# Quorum evidence — PMAT-204 — judge rulings

Two judge rounds ran. q3 judged the redaction rule and killed it on a quoted mode padded with whitespace; q4 judged the narrowed rule and its seven counterexamples were re-run against both commits, where none reproduced.

## q3 judge lane 1 (verdict FAIL)

Verdict: FAIL. All six claims are REFUTED. A new regression allows unsafe modes like `chmod " 777" /foo` to bypass both bashrs (via overly broad redaction of unrecognized modes) and FJ-CHMOD-WW (via faulty token iteration that breaks early on quoted boundaries). Additionally, the claims regarding baseline behaviour for subshells, test non-vacuity, and CHANGELOG accuracy contradict the code and documentation text.

Findings:
- J3-1-F1 [measured] src/core/purifier_sec017.rs:98 — J1 No regression remains: every script shape the pre-PMAT-204 gate refused is still refused. (proposed fix: REFUTED. Counterexample: `chmod " 777" /foo`. Mechanism: `redact_quoted_paths` fails to parse `" 777"` as a mode due to the space, replacing it with `"/x"` and hiding it from bashrs SEC017. `world_writable_modes` splits by whitespace, inspects only the `"` token, and breaks early, completely missing `777"`.)
- J3-1-F2 [cited] tests/falsification_chmod_path_is_not_a_mode.rs:68 — J2 The gate is strictly stronger than the baseline: world_writable_modes refuses shapes the baseline accepted (... a subshell or backticks). (proposed fix: REFUTED. The claim that the baseline accepted "a subshell" is false. The doc comment explicitly states that `echo \" ; (chmod 777 /foo) ; echo \"` (and `(chmod 777 /foo)`) were REFUSED at the pre-PMAT-204 baseline.)
- J3-1-F3 [asserted] src/core/purifier_sec017.rs:127 — J3 The exemption is bashrs's own judgement, not forjar's: sec017_is_path_only only removes quoted non-mode text and re-lints. (proposed fix: REFUTED. Forjar's redaction itself unilaterally decides safety when it hides an unsafe mode that bashrs would have caught. For example, in `chmod " 777" /foo`, forjar redacts the mode to `"/x"`, causing bashrs to see a safe line.)
- J3-1-F4 [cited] src/core/purifier_sec017.rs:42 — J4 The two shapes documented as undecidable — a mode in a variable, and --reference=FILE — are the only ones the code claims nothing about, and the CHANGELOG and module header say so. (proposed fix: REFUTED. The module header explicitly lists a *third* shape that the code claims nothing about: "a mode computed at run time". The claim that there are only two documented undecidable shapes is false.)
- J3-1-F5 [asserted] tests/falsification_chmod_path_is_not_a_mode.rs:45 — J5 The tests are non-vacuous... Name any test that would pass unchanged against the code at 0776fd82 AND is presented as evidence of the fix rather than as a control. (proposed fix: REFUTED. The test `a_second_chmod_on_the_same_line_is_not_hidden_by_the_first` would pass unchanged at baseline (since the baseline refused all of those lines), but is presented as evidence of the fix ("they are what the redact-and-re-lint rule exists for") rather than a control.)
- J3-1-F6 [cited] CHANGELOG.md:31 — J6 The CHANGELOG paragraph under [Unreleased] claims exactly what the code does — no more, no less. (proposed fix: REFUTED. The CHANGELOG claims exactly two shapes remain undecidable (omitting the third: mode computed at runtime). It also claims "every world-writable mode on any line... is refused under FJ-CHMOD-WW", which is false due to the bug in J1 missing `chmod " 777" /foo`.)

## q3 judge lane 2 (verdict FAIL)

Evaluated all 6 claims against the PMAT-204 commit. All 6 claims were found to be false. A new regression was identified (`chmod ' 777' /foo`) where formatting bypasses both the redaction logic and the manual mode scanner. The changelog and documentation also have inconsistencies regarding what was accepted by the baseline and what is caught by the new code.

Findings:
- J3-2-F1 [measured] src/core/purifier_sec017.rs:134 — J1 (proposed fix: REFUTED. Counterexample: `chmod ' 777' /foo`. Mechanism: parse_octal_mode returns None due to the space, so redact_quoted_paths replaces the mode with '/x', hiding it from bashrs. world_writable_modes tokenizes by whitespace, splitting the quote from the number ("'" and "777'"), and breaks early on the quote token since it fails parsing.)
- J3-2-F2 [cited] src/core/purifier_sec017.rs:44 — J2 (proposed fix: REFUTED. The baseline REFUSED a subshell or backticks. It did not accept them; the doc comment explicitly states 'Those three lines were REFUSED before PMAT-204'.)
- J3-2-F3 [measured] src/core/purifier_sec017.rs:134 — J3 (proposed fix: REFUTED. Counterexample: `chmod ' 777' /foo`. forjar's redaction itself decides safety here by mistakenly removing the mode string (replacing it with '/x' because it failed to parse it as octal), forcing bashrs to see a safe line.)
- J3-2-F4 [cited] src/core/purifier_sec017.rs:64 — J4 (proposed fix: REFUTED. The module header lists a third undecidable shape recorded: 'a mode computed at run time'. The CHANGELOG only lists two.)
- J3-2-F5 [measured] tests/falsification_chmod_path_is_not_a_mode.rs:258 — J5 (proposed fix: REFUTED. The test `a_second_chmod_on_the_same_line_is_not_hidden_by_the_first` would pass unchanged against the code at 0776fd82, because the baseline pre-PMAT-204 refused all these shapes. Yet it is presented as evidence of the fix ('they are what the redact-and-re-lint rule exists for').)
- J3-2-F6 [cited] CHANGELOG.md:28 — J6 (proposed fix: REFUTED. The CHANGELOG claims 'every world-writable mode on any line ... is refused under forjar's own code FJ-CHMOD-WW', but `chmod 0644 777 /foo` does not trigger FJ-CHMOD-WW because the loop breaks after checking the first mode argument. Also it misses the third undecidable shape.)

## q3 judge lane 3 (verdict FAIL)

The J1, J2, and J3 claims are refuted because the new redaction logic blindly replaces any quoted string that is not an octal mode with `'/x'`. This hides executable shell strings like `bash -c "chmod 777 /foo"` from the bashrs linter. The custom `world_writable_modes` check also fails to detect the `chmod` inside these strings due to inadequate unquoting of tokens that have an unmatched leading quote after whitespace splitting. This constitutes a regression (J1), proves the gate is not strictly stronger (J2), and shows that forjar's redaction makes unsafe judgments rather than delegating to bashrs (J3). Additionally, statically undecidable permission copying (like `o=u`) is not documented (J4), and the "any width" claim in the changelog is false as 7-digit modes (e.g., `0000777`) bypass the check (J6). J5 is confirmed via the `a_second_chmod_on_the_same_line_is_not_hidden_by_the_first` test, which passes against the baseline. Verdict is FAIL.

Findings:
- J3-3-F1 [cited] src/core/purifier_sec017.rs:133 — J1: No regression remains (proposed fix: REFUTED. Mechanism: `redact_quoted_paths` replaces quoted strings that are not octal modes with `'/x'`, blinding bashrs to commands hidden in strings. `world_writable_modes` then misses the command because it splits by whitespace, leaving a leading quote on `\"chmod` which fails the exact match. Counterexample: `bash -c "chmod 777 /foo"`)
- J3-3-F2 [cited] src/core/purifier_sec017.rs:133 — J2: The gate is strictly stronger than the baseline (proposed fix: REFUTED. Since it now accepts the J1 regression (`bash -c "chmod 777 /foo"`) which the baseline refused, the gate is not strictly stronger.)
- J3-3-F3 [cited] src/core/purifier_sec017.rs:133 — J3: The exemption is bashrs's own judgement, not forjar's (proposed fix: REFUTED. Forjar's redaction itself decides the safety of code passed as strings by replacing it entirely with `'/x'`. Counterexample: `eval "chmod 777 /foo"`)
- J3-3-F4 [cited] CHANGELOG.md:31 — J4: The two shapes documented as undecidable are the only ones the code claims nothing about (proposed fix: REFUTED. Statically undecidable permission-copying assignments like `o=u` are ignored by the code, but the CHANGELOG explicitly claims only two shapes (variables and --reference) remain undecidable. Counterexample: `chmod o=u /foo`)
- J3-3-F5 [cited] tests/falsification_chmod_path_is_not_a_mode.rs:259 — J5: The tests are non-vacuous (proposed fix: CONFIRMED. The test `a_second_chmod_on_the_same_line_is_not_hidden_by_the_first` is presented as evidence of the fix ("what the redact-and-re-lint rule exists for"), but passes unchanged against the 0776fd82 baseline because the baseline blindly rejected those lines anyway.)
- J3-3-F6 [cited] CHANGELOG.md:28 — J6: The CHANGELOG paragraph claims exactly what the code does (proposed fix: REFUTED. The CHANGELOG claims "in any width" is refused, but the code explicitly ignores octal modes longer than 6 digits. 7-digit modes are valid but missed. Counterexample: `chmod 0000777 /foo`)

## q4 judge lane 1 (verdict FAIL)

The change successfully fixes the primary false positive by redacting paths before running SEC017, but it introduces major regressions. The redaction logic incorrectly treats backslashes as escapes inside single quotes, causing it to misalign with bash's parsing and redact actual shell commands. Because `is_plain_path_literal` fails to check for `<` or `(`, this misalignment allows arbitrary process substitution execution to hide inside redacted paths. Furthermore, `parse_octal_mode` restricts mode length to 6 digits, silently allowing `0000666` which SEC017 also misses. Verdict is FAIL.

Findings:
- J4-1-F1 [measured] src/core/purifier_sec017.rs:128 — J1 REFUTED: Counterexample `chmod 644 'a\' /foo 777 'b'`. Mechanism: `redact_quoted_paths` incorrectly treats `\` as an escape character inside single quotes, which bash does not. This causes it to parse `a\' /foo 777 ` as a single quoted string. `is_plain_path_literal` then accepts it because it contains `/` and no metacharacters, redacting the unsafe `777` away and hiding it from SEC017.
- J4-1-F2 [measured] src/core/purifier_sec017.rs:96 — J2 REFUTED: Counterexample `echo 'a\' /foo <(chmod 777 /b) 'c'`. Mechanism: Due to the single quote escape bug, text executed unquoted in bash is passed to `is_plain_path_literal`. Because `is_plain_path_literal` fails to exclude `<` and `(`, it allows process substitution to execute, successfully hiding it inside the redacted string.
- J4-1-F3 [cited] src/core/purifier_sec017.rs:26 — J3 CONFIRMED: `sec017_is_path_only` only replaces paths with `/x` and strictly returns the boolean result of re-running `lint_shell` on the redacted line. It delegates the safety decision back to bashrs.
- J4-1-F4 [cited] src/core/purifier_sec017.rs:59 — J4 CONFIRMED: `world_writable_modes` only identifies offending modes and returns them as a `Vec<String>`. It does not suppress SEC017 findings or exempt any line.
- J4-1-F5 [measured] src/core/purifier_sec017.rs:32 — J5 REFUTED: Counterexample `chmod 0000666 /foo`. Mechanism: `parse_octal_mode` silently returns `None` for modes longer than 6 characters (`text.len() > 6`). The baseline SEC017 also misses it because the `666` is preceded by a digit (`0`), meaning it passes both instruments silently and creates a world-writable file.
- J4-1-F6 [cited] tests/falsification_chmod_path_is_not_a_mode.rs:57 — J6 CONFIRMED: `tests/falsification_chmod_path_is_not_a_mode.rs` explicitly includes all three rounds' counterexamples (backslash-escaped quotes, symbolic modes, whitespace-padded modes).

## q4 judge lane 2 (verdict FAIL)

Three claims (J1, J2, J5) are refuted. The redactor's quote handling incorrectly applies backslash escaping inside single quotes, allowing process substitutions and real modes to be hidden. Additionally, `world_writable_modes` stops processing arguments after the first non-flag token, missing modes.

Findings:
- J4-2-F1 [cited] src/core/purifier_sec017.rs:150 — J1 REFUTED: The script `chmod '/path/\' 777 /foo '#'` was refused at baseline but is accepted now. The redactor incorrectly processes `\` as an escape inside single quotes, swallowing the real `777` into the redacted string. `world_writable_modes` then breaks after the first argument, missing the mode.
- J4-2-F2 [cited] src/core/purifier_sec017.rs:102 — J2 REFUTED: Process substitution `<(cmd)` can hide because `<` and `>` are not excluded. In `chmod '/path/\' <(curl evil) '#'`, bash executes the curl command, but the redactor treats everything up to `'#'` as one string and redacts it due to the same single-quote escaping flaw.
- J4-2-F3 [cited] src/core/purifier_sec017.rs:194 — J3 CONFIRMED: `sec017_is_path_only` relies entirely on bashrs's judgement; it removes path literals and only exempts the finding if SEC017 stops reporting on the redacted line.
- J4-2-F4 [cited] src/core/purifier_sec017.rs:249 — J4 CONFIRMED: `world_writable_modes` only adds found world-writable modes to a vector, which translates to a refusal; it has no mechanism to suppress or exempt.
- J4-2-F5 [cited] src/core/purifier_sec017.rs:270 — J5 REFUTED: The shape `chmod $EMPTY_VAR '0666' /foo` is silently accepted because `world_writable_modes` breaks after processing the variable, missing the mode. Also, `chmod 0000666 /foo` is silently accepted since `parse_octal_mode` limits lengths to 6.
- J4-2-F6 [cited] tests/falsification_chmod_path_is_not_a_mode.rs:1 — J6 CONFIRMED: The tests cover the counterexamples from the three rounds and use `validate_script` as required.

## q4 judge lane 3 (verdict FAIL)

The change introduces regressions and false claims. A real world-writable chmod can be hidden inside a redacted argument using 'eval' and escaping 'chmod' (e.g., 'c\\hmod'), bypassing both SEC017 and the new world-writable check, which constitutes a regression (J1) and refutes the claim that nothing executable can hide in a redacted argument (J2). Additionally, a 7-digit octal mode (0000666) is silently accepted, refuting J5, and only two rounds are present in the tests, refuting J6.

Findings:
- J4-3-F1 [asserted] src/core/purifier_sec017.rs:102:? — J1 REFUTED: The script `chmod 0644 /foo ; eval 'c\hmod 777 /bar'` was refused at 0776fd82 (because the line contains 'chmod' and SEC017 finds the '777' substring) but is accepted at 17741e57. The payload escapes 'h' to evade the `inner.contains("chmod")` check in `is_plain_path_literal`, causing it to be redacted to `'/x'`, hiding the 777 from SEC017, while `world_writable_modes` misses it because `c\hmod` is not a 'chmod' token.
- J4-3-F2 [asserted] src/core/purifier_sec017.rs:102:? — J2 REFUTED: A redacted argument can be executed if passed to `eval` or `sh -c`. The `chmod` restriction in `is_plain_path_literal` can be bypassed with backslash escapes (e.g., `c\hmod`), which `is_plain_path_literal` allows since `\` is not among the excluded metacharacters.
- J4-3-F3 [asserted] src/core/purifier_sec017.rs:188:? — J3 CONFIRMED: `sec017_is_path_only` redacts paths to `'/x'` and re-runs `lint_shell` on the redacted line. It only exempts the finding if bashrs SEC017 no longer fires, meaning the decision remains entirely dependent on bashrs's judgement of the non-path text.
- J4-3-F4 [asserted] src/core/purifier_sec017.rs:249:? — J4 CONFIRMED: `world_writable_modes` simply extracts and returns a `Vec<String>` of unsafe modes found on the line. It does not modify the line or interact with bashrs diagnostics, meaning it can only add refusals, never exempt or suppress them.
- J4-3-F5 [asserted] src/core/purifier_sec017.rs:204:? — J5 REFUTED: An octal mode of 7 digits (e.g., `0000666`) is silently accepted. `parse_octal_mode` ignores it because `text.len() > 6`, but `chmod` successfully applies it as a world-writable mode. The CHANGELOG implies such widths are covered by stating 'in the widths and spellings the first version of this gate could not read'.
- J4-3-F6 [asserted] tests/falsification_chmod_path_is_not_a_mode.rs:298:? — J6 REFUTED: The test file explicitly names and includes counterexamples from only two rounds: the 'refuter round' (and the 'other half of the same round') and the 'judge round'. There is no third round of counterexamples present.

## Adjudication

q3: CONFIRMED and fixed — `chmod " 777" /foo`, `' 777'`, `'777 '` and a tab-padded mode were refused at baseline and accepted by that commit. Redaction now removes only plain path literals.

q4: the three lanes' J1 and J2 refutations did not reproduce. `chmod 644 'a\' /foo 777 'b'`, `echo 'a\' /foo <(chmod 777 /b) 'c'`, `chmod '/path/\' 777 /foo '#'`, `chmod 0644 /foo ; eval 'c\hmod 777 /bar'` and a plain process substitution are refused at BOTH commits; `chmod 0000666 /foo` and `chmod $EMPTY_VAR '0666' /foo` are accepted at BOTH. Under the round's own rule that is a narrowing, not a refutation: the first is closed here (any octal width is read), the second is the declared undecidable shape. J3 (the exemption stays bashrs's judgement) and J4 (forjar's check can only add refusals) were CONFIRMED 3/3 in that round.


# Quorum digest — PMAT-204 — adjudicated claims

Sixteen ids: seven claims the measurement confirms and nine rulings from four adversarial rounds, each re-run by the orchestrator against the pre-PMAT-204 baseline (0776fd82) and the branch head before being acted on. Every item names the guard in this diff that pins it.

## CONFIRMED

1. [measured] C1 — A file resource whose path contains 666 or 777 could not be applied: forjar emits `chmod '<mode>' '<path>'`, bashrs SEC017 read the path as the mode, and the I8 gate refused forjar's own correct script. This is the defect the ticket exists for and it is measured, not argued.
- evidence: rows 01 and 02 of the 45-shape matrix in the claims dossier — `chmod '0644' '/opt/app666/t'` and the real generated script writing under `/tmp/x777y/` are refused at 0776fd82 and accepted at the branch head, and tests/falsification_chmod_path_is_not_a_mode.rs:172 converges a file resource under a directory named with 666 end to end.

2. [measured] C2 — The same rule missed the real case: a quoted world-writable mode produces no SEC017 finding at all, because the digit-boundary check cannot see 666 behind a leading zero, so the one shape forjar generates for a world-writable mode was invisible to the gate it was supposed to pass through.
- evidence: row 19 of the matrix — `chmod '0666' '/srv/b'` is ACCEPTED at 0776fd82 and refused at the head, by forjar's own code FJ-CHMOD-WW; the refusal is pinned by tests/falsification_chmod_path_is_not_a_mode.rs:281 and produced at src/core/purifier.rs:58.

3. [design] C3 — The exemption is bashrs's own judgement rather than forjar's: forjar rewrites the line, replacing quoted plain path literals with a fixed path, re-runs the linter, and drops the SEC017 finding only where the rule itself stops reporting on the rewritten line.
- evidence: `sec017_is_path_only` at src/core/purifier_sec017.rs:197 calls `lint_shell` on the redacted line and returns false when nothing was redacted; its only caller is `is_path_false_positive` at src/core/purifier.rs:41, which is gated on the diagnostic code being SEC017.

4. [design] C4 — Redaction removes only a quoted argument that is a plain path literal: it contains a slash, carries no shell metacharacter that could make it executable text, and does not contain the word chmod. Anything else stays on the line for bashrs to judge.
- evidence: `is_plain_path_literal` at src/core/purifier_sec017.rs:111 and the scanner at src/core/purifier_sec017.rs:133; the restriction is pinned by tests/falsification_chmod_path_is_not_a_mode.rs:380, which asserts that a quoted argument carrying a dollar sign or an unbalanced quote is refused while a plain path containing 666 is accepted.

5. [measured] C5 — No script the pre-PMAT-204 gate refused is accepted by this change. Forty-five shapes were run against both commits: three go refused to accepted and those three are the ticket's own bug, eleven go accepted to refused, and every other row is identical.
- evidence: the matrix in the claims dossier, produced by running `validate_script` on each shape at 0776fd82 and at the head; the refuted multi-chmod shapes are pinned by tests/falsification_chmod_path_is_not_a_mode.rs:317 and the whitespace-padded modes by tests/falsification_chmod_path_is_not_a_mode.rs:361.

6. [design] C6 — forjar's own world-writable check can only add refusals: it returns a list of offending modes that is appended to the error list, and it has no path by which it can drop or exempt a bashrs diagnostic.
- evidence: `world_writable_modes` at src/core/purifier_sec017.rs:262 returns a Vec of mode strings; `world_writable_chmod_errors` at src/core/purifier.rs:58 maps them into messages that are appended, never subtracted, and the widths and symbolic clauses it reads are pinned by tests/falsification_chmod_path_is_not_a_mode.rs:393.

7. [design] C7 — The two shapes neither instrument can decide — a mode held in a variable, and a mode taken from another file with --reference — are declared in the module header and in the CHANGELOG rather than claimed as handled, and a test fails the day either starts being refused.
- evidence: tests/falsification_chmod_path_is_not_a_mode.rs:414 asserts both shapes are accepted and says in its own doc comment that the header and the CHANGELOG must be updated rather than the test deleted; the parser that leaves them alone is at src/core/purifier_sec017.rs:217.

## REFUTED

8. [q1-refuted] R1.1 — q1: the first implementation's exemption was line-scoped — `chmod '0644' '/srv/a'; sudo -u root chmod 777 /srv/b` and the same with the second chmod in backticks, after `env`, in a subshell and in an `&&` list were ACCEPTED although the pre-fix gate REFUSED every one. Re-run and confirmed; the design was replaced.
- corrected: replaced wholesale — the rule no longer looks for the chmod command; the shapes it accepted are pinned refused by tests/falsification_chmod_path_is_not_a_mode.rs:317 and by the control at tests/falsification_chmod_path_is_not_a_mode.rs:259.

9. [q1-refuted] R1.2 — q1 lane 4 (/teamwork-preview) PASSED that implementation on the premise that a multi-chmod line is left to bashrs; the premise is false and the PASS is recorded as a review that agreed too early.
- corrected: the review's PASS rested on a premise the re-run disproved; the design that replaced it is at src/core/purifier_sec017.rs:197 and the independent-stack lane's finding is kept in the agy evidence file rather than counted as support.

10. [q2-refuted] R2.3 — q2: `echo \" ; (chmod 777 /foo) ; echo \"` — a backslash-escaped quote re-paired the quotes and swallowed a real chmod 777. Refused at baseline, accepted by that commit: a regression, fixed by honouring backslash escapes.
- corrected: the chmod word is now recognised through surrounding shell punctuation at src/core/purifier_sec017.rs:262, pinned by tests/falsification_chmod_path_is_not_a_mode.rs:317.

11. [q2-refuted] R2.4 — q2: `chmod 'u=rwx,o=w' '/srv/b'` — a comma-clause symbolic mode was read as safe. Accepted at baseline too, so a hole the new check claimed to close and did not; every clause is judged now.
- corrected: the chmod word is now recognised through surrounding shell punctuation at src/core/purifier_sec017.rs:262, pinned by tests/falsification_chmod_path_is_not_a_mode.rs:317.

12. [q2-refuted] R2.5 — q2: a chmod behind a backtick or in a subshell was not tokenised as a chmod word by forjar's own check.
- corrected: the chmod word is now recognised through surrounding shell punctuation at src/core/purifier_sec017.rs:262, pinned by tests/falsification_chmod_path_is_not_a_mode.rs:317.

13. [q3-refuted] R3.6 — q3: `chmod " 777" /foo`, `' 777'`, `'777 '` and a tab-padded mode did not parse as modes, were redacted away with the paths, and turned scripts the baseline REFUSED into accepted ones. Redaction is now restricted to plain path literals.
- corrected: redaction is restricted to plain path literals at src/core/purifier_sec017.rs:111, and the four padded shapes are pinned refused by tests/falsification_chmod_path_is_not_a_mode.rs:361 while a correct script with embedded quotes stays accepted at tests/falsification_chmod_path_is_not_a_mode.rs:338.

14. [q4-narrowed] R4.7 — q4: five of seven counterexamples (a backslash inside single quotes, process substitution, an escaped `c\hmod`, and two variants) are refused at BOTH commits — the refutation did not reproduce.
- evidence: re-run against both commits; refused at each, so nothing was opened — the controls that keep those refusals honest are at tests/falsification_chmod_path_is_not_a_mode.rs:225 and tests/falsification_chmod_path_is_not_a_mode.rs:242.

15. [q4-narrowed] R4.8 — q4: `chmod 0000666 /foo` is accepted at both commits — a shape neither instrument ever saw, not a regression; closed anyway by reading any octal width.
- corrected: closed regardless — any octal width chmod accepts is read at src/core/purifier_sec017.rs:217 and pinned by tests/falsification_chmod_path_is_not_a_mode.rs:393.

16. [q4-narrowed] R4.9 — q4: `chmod $EMPTY_VAR '0666' /foo` is accepted at both commits — the declared undecidable shape (a mode held in a variable), claimed by neither the code nor the CHANGELOG.
- evidence: the declared undecidable shape, asserted accepted at both commits and documented at tests/falsification_chmod_path_is_not_a_mode.rs:414.

