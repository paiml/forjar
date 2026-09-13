# PMAT-549 — the CRUX: is UNMEASURED how others say "could not measure"?

In round one, one survey lane (gemini-3.1-pro-high) was asked three things:
how three systems separate an unreachable target from a failed check, what
each does when both happen at once, and whether forjar's UNMEASURED state and
its exit 4 match them.

NAGIOS.
- A plugin exits 0 OK, 1 WARNING, 2 CRITICAL or 3 UNKNOWN.
- `check_by_ssh` exits UNKNOWN when it cannot reach the target.
- When states aggregate, CRITICAL outranks UNKNOWN.
- A caller that alerts only on CRITICAL treats an unreachable host as fine.

PROMETHEUS.
- Reachability is its own series, `up`, which is 0 after a failed scrape.
- Threshold queries over a target that was never scraped return nothing and
  fire nothing.
- Without an alert on `up == 0`, or `absent()`, a dead target looks clean.

TERRAFORM.
- `plan -detailed-exitcode` exits 1 for an error, such as a provider that
  cannot refresh, and 2 for a diff.
- The error halts the run, so it wins over the diff, and a caller never sees a
  false clean.

THE SURVEY'S VERDICT. The judges ruled it CONFIRMED, 3 of 3: it describes
forjar and the three systems accurately.

1. Separation MATCHES. UNMEASURED is its own state, as UNKNOWN and `up == 0`
   are, and it is folded into neither drift nor clean.
2. Precedence and exit status do NOT fully match.
   - Drift wins over unmeasured, as CRITICAL wins over UNKNOWN in Nagios, and
     unlike Terraform, where the error wins.
   - A tripwire run with drift exits 1, not the drift class 10. The error
     string is "N drift finding(s)", and the classifier looks for "drift
     detected".
   - That string is on the base commit and predates this change. The branch
     keeps it, and it is filed as ISSUE_EXIT10, because changing it changes the
     exit code every `--tripwire` caller sees for drift.
3. Caller visibility does NOT match.
   - Without `--tripwire`, `forjar drift` exits 0 whatever it found. That is
     true of drift on the base commit too.
   - The MCP verb's `drifted` is false when only unmeasured resources remain.
     That is the E05 contract, which tells an agent to read `drifted` together
     with `unchecked`. The unmeasured resources are named in both `unmeasured`
     and `unchecked`.

WHAT THE BRANCH DOES WITH IT. It takes the separation and the precedence, and
keeps exit 4 (connection) for a tripwire run where the only problem is
unmeasured resources. Points 2 and 3 stay as disclosed, pre-existing behaviour.
An exit-code change for drift is not folded into a fix about unanswered queries.
