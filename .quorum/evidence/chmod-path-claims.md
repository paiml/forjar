# Quorum evidence — PMAT-204 — the claims as put to the refuters and judges

The claim under test is one sentence: forjar's I8 gate refused a correct script because bashrs SEC017 read a PATH as a chmod mode, and the fix removes that mistake without removing any refusal. Everything below is measured by running `forjar`'s own `validate_script` on the shape, first at the pre-PMAT-204 baseline (0776fd82, the release's main) and then at the branch head. A shape's row says what each commit did with it; a claim is confirmed only where the two columns say what the ticket says they should.

## The 45-shape matrix (baseline vs fix)

| shape | at 0776fd82 | at the fix |
|---|---|---|
| `01 path 666 safe mode` | refused | ACCEPTED |
| `02 real generated script` | refused | ACCEPTED |
| `03 plain safe` | ACCEPTED | ACCEPTED |
| `04 quoted path with a quote` | ACCEPTED | ACCEPTED |
| `05 escaped-quote path, safe` | refused | ACCEPTED |
| `06 safe symbolic u+x` | ACCEPTED | ACCEPTED |
| `07 safe symbolic g+w` | ACCEPTED | ACCEPTED |
| `08 variable mode` | ACCEPTED | ACCEPTED |
| `09 --reference` | ACCEPTED | ACCEPTED |
| `10 bare chmod 666` | refused | refused |
| `11 bare chmod -R 777` | refused | refused |
| `12 echo chmod 777` | refused | refused |
| `13 sudo second chmod` | refused | refused |
| `14 backtick second chmod` | refused | refused |
| `15 env second chmod` | refused | refused |
| `16 subshell second chmod` | refused | refused |
| `17 and-list second chmod` | refused | refused |
| `18 quoted 0666 second` | ACCEPTED | refused |
| `19 quoted 0666 alone` | ACCEPTED | refused |
| `20 five-digit octal` | ACCEPTED | refused |
| `21 0662` | ACCEPTED | refused |
| `22 symbolic a+w` | ACCEPTED | refused |
| `23 symbolic o+w` | ACCEPTED | refused |
| `24 comma symbolic u=rwx,o=w` | ACCEPTED | refused |
| `25 comma symbolic bare` | ACCEPTED | refused |
| `26 comma symbolic a+w` | ACCEPTED | refused |
| `27 find -exec` | ACCEPTED | refused |
| `28 subshell alone` | refused | refused |
| `29 backtick alone` | ACCEPTED | refused |
| `30 escaped quotes hide chmod` | refused | refused |
| `31 escaped quotes + subshell` | refused | refused |
| `32 unbalanced quote` | refused | refused |
| `33 leading space dq` | refused | refused |
| `34 leading space sq` | refused | refused |
| `35 trailing space` | refused | refused |
| `36 tab mode` | refused | refused |
| `37 two modes` | refused | refused |
| `38 bash -c nested` | ACCEPTED | ACCEPTED |
| `39 eval nested` | refused | refused |
| `40 o=u copy` | ACCEPTED | ACCEPTED |
| `41 seven-digit octal` | ACCEPTED | ACCEPTED |
| `42 dollar-paren second` | refused | refused |
| `43 pipe then chmod` | refused | refused |
| `44 heredoc with chmod` | refused | refused |
| `45 quoted path with dollar` | refused | refused |

Three rows go refused -> accepted: those are the bug (a safe mode on a path containing 666 or 777, and the real generated script). Eleven go accepted -> refused: every world-writable mode bashrs could not see. No row goes the other way.

## The claims

- C1 SEC017 reads the literal text of a chmod line and never asks which word is the mode, so a PATH containing 666 or 777 was read as a world-writable mode and `forjar apply` refused its own script. Measured: rows 01 and 02 of the matrix.
- C2 The same rule misses the real case: `chmod '0666' '/tmp/plain/t'` produces no finding at all, because the boundary check cannot see 666 behind a leading 0. Measured: row 19 at baseline.
- C3 The exemption is bashrs's own judgement, not forjar's: the line is re-linted with its quoted plain PATH LITERALS replaced, and the finding is dropped only if the rule stops reporting.
- C4 forjar's own check refuses every world-writable mode it can read on any line, in any octal width or symbolic clause.
- C5 No script the baseline refused is accepted by the fix. Measured: the matrix, plus the seven counterexamples of the fourth round.
- C6 The undecidable shapes — a mode held in a variable, and `--reference=FILE` — are declared, not claimed.
- C7 The falsification suite goes through `validate_script` and carries every round's counterexamples.

## Rounds

Four adversarial rounds ran, each in per-lane standalone clones with no build:

- q1, width 5 (three blind claim lanes, one `/teamwork-preview` independent-stack lane, one CRUX lane) on the first implementation: 3 FAIL. Its rule found the chmod command and read its mode; a second chmod behind `sudo`, backticks, `env`, a subshell or an `&&` list was not seen and the line was exempted whole.
- q2, width 3 (refuters) on the redaction rule: 3 FAIL. Two kills survived re-running — a backslash-escaped quote re-paired the quotes and swallowed a real `chmod 777` (a regression), and a comma-clause symbolic mode was read as safe (a pre-existing hole).
- q3, width 3 (judges): 3 FAIL. A quoted mode padded with whitespace was redacted away; re-running against the baseline showed four such shapes had been refused there. Redaction was narrowed to plain path literals.
- q4, width 3 (judges) on the narrowed rule: 3 FAIL, and not one of the seven counterexamples reproduced. Five are refused at both commits; two (`chmod 0000666`, a mode in an empty variable) are accepted at both, so they are shapes neither instrument ever saw. The width one was closed anyway.

Every lane verdict in this file is a claim. Each was re-run by the orchestrator against both commits before it was acted on or set aside, and the matrix above is that re-run.

