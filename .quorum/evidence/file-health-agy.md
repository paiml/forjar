**DO-NOT-SHIP**

Your instinct to be adversarial was correct. The refactor of `task.rs` is flawless, but the CI ratchet is dangerously broken and will both break builds for valid code (false positives) and silently allow technical debt through (false negatives).

### Part 1: The `src/resources/task` Pure Move (SHIP)
I independently verified the "pure move" claim by stripping out imports, module declarations, and visibility modifiers, then diffing the remaining function bodies against `origin/main:src/resources/task.rs`. 
- **Behavioral Changes:** Zero. The generated shell script bytes and logic are identical.
- **Visibility:** Only three functions were widened (`service_rid`, `timeout_wrapped`, and `pipeline_script`). They were widened from private to exactly `pub(super)`. Because they are now invoked across the new module boundaries (e.g., `apply.rs` and `check.rs` calling `helpers.rs`), this is the strict minimum visibility required. The public API surface of the `resources::task` module remains identical.

### Part 2: `.github/workflows/lint.yml` (DO-NOT-SHIP)
The `file-health` CI job contains multiple shell and Git edge-case failures. 

#### FALSE POSITIVES (Fires wrongly, blocking valid code)
1. **The `pipefail` abort on new files (Confirmed)** - `.github/workflows/lint.yml:39`
   ```bash
   before=$(git cat-file -p "$BASE:$f" 2>/dev/null | wc -l)
   ```
   If `$f` is a newly added file, it does not exist in `$BASE`. `git cat-file` exits with `128`. Because `set -euo pipefail` is active (line 30), the pipeline fails immediately. The entire GitHub Action aborts with `128` on the very first new `.rs` file, regardless of whether it's 10 lines or 600 lines. The script never reaches the `before=0` comment.
   *Fix:* `before=$( (git cat-file -p "$BASE:$f" 2>/dev/null || true) | wc -l )`

2. **PR Base Drift (Blaming PRs for `main`'s growth)** - `.github/workflows/lint.yml:31`
   ```bash
   BASE="${{ github.event.pull_request.base.sha || 'HEAD~1' }}"
   ```
   For a `pull_request` event, `actions/checkout` fetches and checks out a synthetic merge commit (`refs/pull/PR/merge`) built against the *current* tip of `main`. However, `github.event.pull_request.base.sha` is the tip of `main` *at the moment the webhook fired*. 
   If `main` advances in the background before checkout, `HEAD` contains changes from the new `main` that `$BASE` lacks. `git diff "$BASE" HEAD` will falsely attribute `main`'s growth to the PR, failing the PR for someone else's commits.
   *Fix:* Compare against the merge commit's actual first parent: `BASE=$(git rev-parse HEAD^1)`

#### FALSE NEGATIVES (Fails to fire, smuggling in debt)
3. **Missing Rename Detection (`R`)** - `.github/workflows/lint.yml:46`
   ```bash
   done < <(git diff --name-only --diff-filter=ACM "$BASE" HEAD -- '*.rs')
   ```
   `ACM` excludes `R` (Renamed). If an author renames a 500+ line file and adds 100 more lines of debt to it in the same PR, `git diff` detects it as a rename (e.g., `R100`). It bypasses this loop entirely. 
   *(Note: If you just add `R`, `before` will evaluate `$BASE:$f` using the NEW path, fail, return `0`, and falsely block any rename of a large file. You must parse `git diff --name-status` to get the old path for the `before` count).*

4. **Multi-commit Push Bypass** - `.github/workflows/lint.yml:31`
   For a direct push to `main` (no PR context), `base.sha` is null, falling back to `'HEAD~1'`. If a developer pushes 5 commits at once, `HEAD~1` is only the parent of the *last* commit. If commit 1 smuggles in a 1000-line file and commit 5 just fixes a typo, the diff between `HEAD~1` and `HEAD` sees zero growth. The debt is merged.
   *Fix:* Use `${{ github.event.before }}` for push events.

5. **Quoted paths with spaces bypass `[ -f ]`** - `.github/workflows/lint.yml:35, 46`
   If a file is named `src/foo bar.rs`, `git diff --name-only` outputs it wrapped in double quotes: `"src/foo bar.rs"`. `[ -f "$f" ]` will literally look for a file named `"src/foo bar.rs"` (with the quotes in the filename), fail to find it, and trigger `|| continue`. The file is silently ignored.
   *Fix:* Use `git diff -z` and `while IFS= read -r -d '' f; do`.

[exited with code 0]
