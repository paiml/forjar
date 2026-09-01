# Quorum evidence — #390-E — adjudicated claims

Canonical format: `N. [lane] claim` with an indented `- evidence:` or `- corrected:`
subline. `scripts/quorum_evidence.py` parses this and checks counts against the
receipt's tallies symmetrically.

## CONFIRMED — 5 claims survived refutation

1. [probe] (explains-symptom) With `timeout:` set, a failing command line is silently swallowed and apply reports success — the identical config without `timeout:` correctly fails.
   - evidence: Measured on the PUBLISHED 1.24.0 binary. A task with `timeout: 30`, `completion_check: "true"`, and a command beginning with `false` produced `local: 1 converged, 0 unchanged, 0 failed` and exit 0. Deleting only the `timeout:` line from the same config produced exit 1. Root cause at src/resources/task.rs:215 — `timeout {n} bash <<'FORJAR_TIMEOUT'` starts a nested shell that does not inherit the outer `set -euo pipefail` emitted at task.rs:205, and a shell exits with the status of its LAST command. A wrong result reported as success is worse than a failure.

2. [probe] (explains-symptom) The nested bash's stdin IS the heredoc, so a stdin-reading command consumes the rest of its own script.
   - evidence: Prototyped against real bash. `timeout 5 bash <<'D'` with body `cat > /tmp/eaten.txt` then `echo ... > /tmp/second.txt` produced eaten=YES second_ran=NO — the second line never ran. This is FJ-2732, the defect `src/transport/stdin_isolation.rs` was written to close, re-opened one layer BELOW the wrapper that closes it. The same body under `bash /dev/fd/3 3<<'D'` produced eaten=no second_ran=yes.

3. [probe] (explains-symptom) A fixed heredoc delimiter collides with command content and closes the heredoc early, running the remainder in the OUTER shell.
   - evidence: Measured on 1.24.0 with a command containing a bare `FORJAR_TIMEOUT` line. Output contained both `FORJAR_TIMEOUT: command not found` and `AFTER-DELIMITER` — proving the heredoc closed early and the trailing lines executed outside it. This is the C8 delimiter-collision class that src/core/shell_escape.rs documents as fixed for `file` content and which was still live for `task`. For a `sudo:` resource the consequence is worse: the remainder runs UNPRIVILEGED, silently.

4. [crux] (partially-explains) Every comparable system either avoids nested shells entirely or injects strictness explicitly — forjar was below the field default.
   - evidence: GitHub Actions `shell: bash` injects `set -eo pipefail` into every step precisely because a nested shell does not inherit it. GNU make uses `.SHELLFLAGS=-ec` per recipe. systemd `ExecStart` is a direct exec with `RuntimeMaxSec`, no shell nesting at all. Terraform `remote-exec` sends the script to one shell and kills the connection on timeout rather than nesting. Ansible async/poll runs the module as a separate process with rc captured. Four of five never create the nesting; the one that does injects strictness explicitly, which is exactly the fix applied here.

5. [pmat] (unrelated-defect) The new falsification suite is not vacuous, and the repo-wide vacuous population did not grow.
   - evidence: `analyze_vacuous_tests` examined 17797 tests across 1874 files. `tests/falsification_390e_nested_shell_strictness.rs` does not appear in the vacuous list. Repo-wide tautologies are unchanged from the 1.24.0 baseline and remain tracked separately.

## REFUTED — 2 claims killed

1. [design] refuted 1/1 — The `sudo:` wrapper also loses `set -euo pipefail` and needs it re-injected.
   - corrected: It does not. `in_declared_privilege_context` (src/core/codegen/dispatch.rs:284) wraps the WHOLE generated script, which `batch_script` already opens with `set -euo pipefail` at src/resources/task.rs:205. Only the `timeout:` wrapper injects a bare `{command}`. The sudo path needed the stdin and delimiter fixes but NOT the strictness one — asserting otherwise would have added a redundant second `set -euo pipefail` and implied a defect that is not there.

2. [design] refuted 1/1 — Existing tests asserting `sudo bash <<'FORJAR_SUDO'` prove the old shape is required and the fix is a regression.
   - corrected: Those assertions pinned the literal invocation, not the guarantee. Their intent is "sudo elevates via a heredoc", which the fix preserves — it changes only where the script is handed to the nested shell (fd 3 rather than stdin). Updating them is strengthening, not weakening: they now also assert stdin isolation. Eight such assertions across src/core/codegen/tests_sudo.rs and src/resources/tests_task.rs were updated with a comment recording why the shape changed.
