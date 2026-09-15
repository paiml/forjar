# PMAT-560 — the lanes, and what each returned

Author model: Claude Opus 5. Every lane Gemini, model measured from its own
log. gemini-3.8-flash-high returned API 503 in both rounds; lane-reduce voided
it each time and its carried verdict is not counted.

## Round 1 — at 8337cab5 — agreed=false, partial=true

| lane | model (measured) | envelope | verdict | findings |
|---|---|---|---|---|
| 1 | gemini-3.1-pro-high | SUCCESS | FAIL | 4 |
| 2 | gemini-3.8-flash-high | ERROR (503) | voided — carried FAIL, 7 findings | — |
| 3 | gemini-3.7-flash-high | SUCCESS | PASS | 6 |

Conversation ids shortened: conv-0606397e, conv-65aaf4b8, conv-289c32c7.

Lane 1 measured that the awk split systemd's `ExecStart` line on `/[ ;]/`, so
`/opt/my script` read as `/opt/my`; that FALSIFY-EXEC-005 described a mutation
the shell cannot undergo; that FALSIFY-EXEC-006's runtime half was untested;
and that the book's "hashes the file it names" was false for such a path.
Lane 3 ran the acceptance suites (10 and 75 green) and passed, having seen the
same tokenisation and not followed it to exit 0. The voided lane 2 found the
same split and four sentence-level defects no counted lane checked
(`N obligations, 0 failed`, presence missing from `converged`, the digest
marker naming a digest not a program, templates exempt from the hex rule) —
UNREVIEWED CLAIM MATERIAL, each one re-read here and each one true.

Harness: lane 1 left 270 untracked paths in its own clone (a scratch cargo
project, 8.4 GB `target/`); `agy-lane.sh` died of SIGPIPE at its KEPT branch
(`head -n 3` under pipefail) after every isolation assertion had passed, so
the KEPT line never printed. The shared checkout was byte-identical. The clone
was removed after the findings were recorded.

## Round 2 — at 921ed4b6 — agreed=false, partial=true

| lane | model (measured) | envelope | verdict | findings |
|---|---|---|---|---|
| 1 | gemini-3.1-pro-high | SUCCESS | PASS (with 2 findings) | 6 |
| 2 | gemini-3.8-flash-high | ERROR (503) | voided — carried PASS | 1 |
| 3 | gemini-3.7-flash-high | SUCCESS | PASS | 6 |

Conversation ids shortened: conv-3a491548, conv-f18cafc0, conv-11457aec.

Both counted lanes confirmed the separator fix, the stderr marker in both the
check and apply paths, the exit-0 shape turning RED under the old awk, the
contract's citations, and the `LifecycleRules` move. Lane 1's PASS carried two
findings — a path containing ` ; ` itself is misread and nowhere documented,
and the CHANGELOG's hex-64 sentence ignored the template exemption; lane 3
called both documents accurate. Both findings were acted on in `a9fd3024`.
