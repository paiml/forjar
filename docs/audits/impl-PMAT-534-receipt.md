# Implementation receipt — PMAT-534 — the latest release is the latest release

verdict: PASS — gate R refuses a full release that `repos/:owner/:repo/releases/latest` does not resolve to, reports a prerelease with the command that promotes it, and treats an unreadable pointer as UNMEASURED. The promotion step, which existed only as one line of a workflow's output for four releases, is now named in three places a reader reaches. Four cases, four killers.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=self  w=100.00  basis=absent  (one gate arm, four cases)
  ph1.delegate  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]
  ph2  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --test falsification_release_check_latest_points_at_the_release"  rerun_exit=0 (4 passed)  log=docs/audits/logs/PMAT-534-latest-points-at-the-release.log §4
  cmd="cargo test --test falsification_release_check_says_what_is_pending --test falsification_dogfood_release_check_pr_window"  rerun_exit=0 (4 + 9, unchanged)  log=§4
  cmd="the same 4 against origin/main's release-check.sh"  rerun_exit=101 (3 failed, 1 guard green)  log=§4
  cmd="four targeted mutations"  rerun_exit=101 each; the kill matrix is §6  log=§5, §6
  cmd="bash scripts/dogfood/release-check.sh"  rerun_exit=0 against the real repository, with the arm  log=§3
  cmd="bashrs lint scripts/dogfood/release-check.sh"  rerun_exit=1, 0 errors
  cmd="cargo clippy --all-targets -- -D warnings" / "cargo fmt --all -- --check"  rerun_exit=0 / 0
  cmd="python3 yaml.safe_load on .github/workflows/release.yml"  parses

## What was measured

On 2026-09-12, before it was corrected by hand:

```
repos/paiml/forjar/releases/latest  ->  v1.25.2
  v1.28.0  prerelease=false  draft=false   created 2026-09-10
  v1.27.0  prerelease=false  draft=false   created 2026-09-08
  v1.26.0  prerelease=false  draft=false   created 2026-09-08
  v1.25.2  prerelease=false  draft=false   created 2026-09-05
```

Three full releases, days newer, and the pointer never moved. Every
`curl -L …/releases/latest/download/…`, every badge and every script that
follows that URL got a four-version-old binary — for days — while gate R said
the release was fine.

## Why the flag alone is not enough

A release is **born a draft prerelease** (`release.yml`, PMAT-166) so no
consumer sees a half-uploaded asset set, and the publish job un-drafts it while
deliberately leaving it a prerelease:

```
gh release edit "$RELEASE_TAG" --draft=false --prerelease --repo …
```

**GitHub fixes "latest" when the prerelease flag is WRITTEN and never recomputes
it.** So `gh release edit <tag> --prerelease=false` clears the flag and leaves
the pointer exactly where it was. The promotion has to say `--latest`, and the
step that was supposed to perform it existed only in one `echo` at the end of a
workflow job — written down nowhere else, checked by nothing.

## The arm, and what it deliberately does not do

| state | verdict |
|---|---|
| full release, `/releases/latest` resolves to it | **pass** |
| full release, resolves to something else | **FAIL**, naming both and the command |
| prerelease | **pass**, reported as pending with the command that ends it |
| `gh api` cannot answer | **FAIL**, UNMEASURED |

**A prerelease is a deliberate state.** forjar publishes a rolling `nightly`
prerelease, and a release between its tag and `make dogfood-published` is
legitimately one; GitHub will never make either latest. Forcing it would refuse
the correct state, so that case is reported with the remedy rather than refused.

**An unreadable pointer is UNMEASURED.** "The pointer could not be read, so
assume it points here" is precisely the silence the four stale releases lived
in.

## The step is now named where a reader is

- `release.yml` prints the exact command, and says why `--latest` is not
  decoration.
- `CLAUDE.md`'s gate R row says what is checked.
- The `forjar-dogfood` skill carries the measurement and the command.

## Falsification

Four cases, **four killers** (log §4–§6):

| case | killed by |
|---|---|
| `a_full_release_that_is_not_latest_is_named_and_red` | removal, M1 `elif false` |
| `a_pointer_that_cannot_be_read_is_unmeasured_and_red` | removal, M2 assume agreement |
| `a_prerelease_is_reported_and_not_forced_to_latest` | removal, M3 fail on prerelease |
| `a_full_release_that_is_latest_passes` | M4 `elif true` |

The one case that survives removal is the over-refusal guard — it asserts the
arm does not fire — and M4 is the mutation that makes it fire wrongly. M4 also
turns three of the four pre-existing `says_what_is_pending` cases red, which is
the other half of the claim: the arm sits in the path every published release
takes.

The fixture's `gh` stub answers `api` **before** `release`, because the two are
told apart by `$1` alone and a stub that answered the PR list for `api` would
hand the gate a JSON array where it wants a tag name.

## Gaps, named

- **Nothing performs the promotion.** This refuses a release that was not
  promoted; it does not promote one. Automating it was rejected on purpose:
  PMAT-166 made promotion the operator's step *after* `make dogfood-published
  VERSION=` passes, and a workflow that promoted on tag would undo that gate.
- **`/releases/latest` is read once, at gate R time.** A release demoted
  afterwards is not noticed until the next run.
- **The arm cannot see a release that should NOT be latest.** A hotfix on an old
  minor, deliberately published after a newer release, would be refused for
  pointing elsewhere — correctly by this rule, but this repository has never cut
  one and the rule has not been tested against that shape.
- **`nightly` is only covered by the prerelease branch.** Nothing asserts that
  the rolling nightly release stays a prerelease; if it were ever promoted, it
  would become latest and this arm would then demand every release match it.

IMPL-PMAT-534-RECEIPT-END
