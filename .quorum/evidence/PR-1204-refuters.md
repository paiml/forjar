=== REFUTER 1 ===
[
  {
    "claim": "1.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59-61 `let quoted = sh_squote(&launch_io::token(&spec_of(&gated(std::path::Path::new(\"/t\"))).unwrap()));` still calls `launch_io::token`, which was removed in src/resources/cloud_instance/launch_io.rs:97-102 (`pub fn apply_script`), while src/resources/cloud_instance/mod.rs:63-64 unconditionally compiles `#[cfg(test)] mod tests_storage;`; grep confirms no other callers of removed `token`, `tail`, or `is_todays` remain."
  },
  {
    "claim": "1.2",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `awk -v c={q} 'BEGIN {{ gsub(/_/, \\\"-\\\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \\\"-\\\", n); if (n == c && $2 == \\\"=\\\" && $3 ~ /^\\\"[0-9A-Za-z._+-]+\\\"$/) ...` maps `_` to `-` in both `c` and `n = $1` before comparing `n == c` (verified in awk for both `foo_bar` vs `foo-bar` and vice versa)."
  },
  {
    "claim": "1.3",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:67-70 `(-[A-Za-z][0-9A-Za-z.-]*)?([^.0-9]|[.]([^0-9]|$)|$)` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` capture `-<letter>...` into `FJ_LIVEPRE` while leaving `FJ_LIVE` unchanged; src/resources/github_release.rs:179-189 uses `if [ -n \"$FJ_LIVE\" ]; then` in `downgrade_guard`."
  },
  {
    "claim": "1.4",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/semver.rs:31-50 `pub fn parse_semver(text: &str) -> Option<Semver> { parse_semver_span(text).map(|(v, _)| v) }` and `return Some((v, end));`; src/tripwire/drift/version_pin.rs:49 `pub use super::semver::{parse_semver, parse_semver_span, Semver};`; src/resources/github_release_check.rs:95,107-118 slices `&tag[end..]` and diverges on `[ \"$FJ_LIVEPRE\" != '{pre}' ]`."
  },
  {
    "claim": "1.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102 `pub fn apply_script(resource: &Resource) -> String { let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH); super::apply_script_on(resource, day_of(now.map_or(0, |d| d.as_secs()))) }`, line 181 `pub(super) fn tail_on(s: &CloudInstance, day: u64) -> String`, and src/resources/cloud_instance/mod.rs:41,404,489 confirm `token()` is removed and `apply_script` reads the clock once and delegates to `apply_script_on`."
  },
  {
    "claim": "1.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:51-66 `fn nothing_declared_is_byte_identical_to_today()` calls `apply_script_on(..., DAY)` at line 52 and removed `launch_io::token(...)` at line 59, causing `E0425` that blocks both it (`TODAY_TAIL`) and src/resources/cloud_instance/userdata.rs:186 `fn unset_script_is_byte_identical_to_today()`, and mixing fixed `DAY` (`20_400`) with wall-clock `token` would fail lines 62-66."
  },
  {
    "claim": "1.7",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:52,59-61 `let s = apply_script_on(&gated(std::path::Path::new(\"/t\")), DAY);` and `let quoted = sh_squote(&launch_io::token(&spec_of(&gated(std::path::Path::new(\"/t\"))).unwrap()));` confirm `nothing_declared_is_byte_identical_to_today` was not updated to use `tok(...)`."
  },
  {
    "claim": "1.8",
    "verdict": "SURVIVED",
    "evidence": "ANGLE.diff:225-247 and src/resources/cloud_instance/tests_chain.rs:289-312 `assert_eq!(rc, 0, \"{out}\");` and `assert_eq!(t[2 * round - 2..], [t1.clone(), t2.clone()], \"{log}\");` confirm both `if run_tokens(&log)[0] ... t1` guards were removed and the assertions run unconditionally."
  },
  {
    "claim": "1.9",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8130-8147 (`- id: PMAT-1189`), 8166-8183 (`- id: PMAT-1190`), and 8202-8219 (`- id: PMAT-1198`) contain the 54 inserted lines, and `tree/.git` does not exist so per-commit placement cannot be verified."
  },
  {
    "claim": "1.10",
    "verdict": "SURVIVED",
    "evidence": "src/resources/tests_package_e.rs:134-149 (2 `fj1189` tests) and src/resources/tests_github_release_verified.rs:36-106 (8 tests including `dash_before_the_version_is_not_a_prerelease`) exist as described, but src/resources/cloud_instance/tests_storage.rs:59 (`launch_io::token`) breaks `--lib` test compilation."
  },
  {
    "claim": "1.11",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59 (`launch_io::token`) breaks `--lib` test compilation, and static `#[test]` counts match 139 for `cloud_instance` (133 in `src/resources/cloud_instance/` plus 6 in src/core/parser/tests_cloud_instance_validate.rs:30-164 via src/core/parser/mod.rs:30-31 `#[cfg(test)] mod tests_cloud_instance_validate;`), 41 for `github_release`, and 133 for `tripwire::drift`."
  },
  {
    "claim": "1.12",
    "verdict": "SURVIVED",
    "evidence": "Cargo.toml:24 `edition = \"2021\"`, src/resources/cloud_instance/mod.rs:63-64 `#[cfg(test)] mod tests_storage;`; `rustfmt --edition 2021 --check` exited 0 on all 13 changed `.rs` files, and `cargo clippy --lib` without `--tests` skips `#[cfg(test)]` modules."
  },
  {
    "claim": "1.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 (`gsub(/_/, \\\"-\\\", n)`) vs src/resources/tests_package_e.rs:134-149 (only `fj1189_hyphen_declared_matches_underscore_in_registry` fails if dropped) and src/resources/github_release_check.rs:106-118 vs src/resources/tests_github_release_verified.rs:99-106 (only `dash_before_the_version_is_not_a_prerelease` fails if reading from first `-`), while execution is blocked by src/resources/cloud_instance/tests_storage.rs:59."
  },
  {
    "claim": "1.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/disk_budget/tests/execution.rs:189 `fn a_cargo_target_whose_binary_is_running_survives()` exists and is untouched in ANGLE.diff; `src/resources/` has 961 `#[test]` functions, and earlier-head runs cannot be directly verified from the merge tree."
  },
  {
    "claim": "1.15",
    "verdict": "REFUTED",
    "evidence": "src/resources/github_release_check.rs:61-70,81-83 modifies `live_version` (adding `FJ_LIVEPRE` and `FJ_FULL`) wrapped by `live_version_unguarded`, which is called by src/resources/github_release.rs:79 (`apply_script` post-install), line 189 (`downgrade_guard` in `apply_script`), and line 209 (`state_query_script`), and src/tripwire/drift/semver.rs:31-50 is also modified; thus the PR does not change only the check script in `github_release_check.rs` plus the `parse_semver_span` re-export."
  },
  {
    "claim": "1.16",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match ANGLE.diff (14 files, 182 `+`, 61 `-`, 243 total, 189 outside roadmap) and PR_BODY.md:29 exactly."
  },
  {
    "claim": "2.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:52,59-62 calls `apply_script_on(..., DAY)` (`DAY = 20_400` at tests_launch.rs:12) and removed `launch_io::token` (deleted at src/resources/cloud_instance/launch_io.rs:97-102), causing `E0425` and a `DAY` vs today token mismatch on line 62; no other callers of removed `token` or renamed `tail` remain in `src/`."
  },
  {
    "claim": "2.2",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:11,19 claims `TODAY_TAIL` and `cloud_instance: 133 passed` (and the other `--lib` filters), while src/resources/cloud_instance/tests_storage.rs:51-66 still calls removed `launch_io::token` at line 59, preventing `--lib` test compilation."
  },
  {
    "claim": "2.3",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `awk -v c={q} 'BEGIN {{ gsub(/_/, \\\"-\\\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \\\"-\\\", n); if (n == c && ...)` maps `_` to `-` in both declared and search crate names (verified directly with awk)."
  },
  {
    "claim": "2.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:61-70,92-96,106-118 (`FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"`, `pin_prerelease`), src/tripwire/drift/semver.rs:31-50 (`parse_semver_span`), and src/tripwire/drift/version_pin.rs:49 confirm `FJ_LIVEPRE` capture, unchanged `FJ_LIVE`/`parse_semver`, ignoring `-` before version, and divergence on mismatch."
  },
  {
    "claim": "2.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 confirm `token()` is removed, `apply_script_on` and `tail_on` accept UTC `day`, and `apply_script` reads `SystemTime::now()` once and delegates to `apply_script_on`."
  },
  {
    "claim": "2.6",
    "verdict": "SURVIVED",
    "evidence": "ANGLE.diff:225-247 and src/resources/cloud_instance/tests_chain.rs:290-294,307-311 confirm both `if run_tokens(&log)[0] ... t1` midnight guards were removed and those assertions execute unconditionally."
  },
  {
    "claim": "2.7",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8130 (`- id: PMAT-1189`), 8166 (`- id: PMAT-1190`), and 8202 (`- id: PMAT-1198`) are each present exactly once with no duplicates."
  },
  {
    "claim": "2.8",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:13,20-22 (`one in each ticket's commit`, `cargo clippy -j 4 --locked --lib -- -D warnings`, `Mutants on the added rows`, `Earlier-head results`) cannot be verified from the exported read-only merge tree without `.git` history or cargo execution."
  },
  {
    "claim": "2.9",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:20 `cargo fmt --all -- --check`: clean; both `rustfmt --edition 2021 --check` on all 13 changed `.rs` files and `cargo fmt --manifest-path tree/Cargo.toml --all -- --check` exited 0 with no diffs."
  },
  {
    "claim": "2.10",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/version_pin.rs:49,67,151,246 (`parse_semver(tag)`, `(Live::Version(l), Some(e)) if *l != e`, `parse_semver(text)`) confirm `version_pin` still compares only `N.N.N` (`Semver`)."
  },
  {
    "claim": "2.11",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 (`182 insertions, 61 deletions = 243 changed lines in 14 files`, `54` roadmap, `189` outside)."
  },
  {
    "claim": "2.12",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:57,63-66 `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` leaves a third obsolete midnight guard despite `launch_with` calling `fixture` -> `apply_script_on(&r, DAY)` at src/resources/cloud_instance/tests_launch.rs:104, silently skipping the check if `t2[0] != t[0]`."
  },
  {
    "claim": "2.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102 (`super::apply_script_on(resource, day_of(now.map_or(0, |d| d.as_secs())))`); grep across `tree/src/` and `tree/tests/` confirms no remaining `apply_script` caller checks the computed token value or UTC day (only `--client-token fj-` at src/resources/cloud_instance/tests_storage.rs:119)."
  },
  {
    "claim": "2.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` greedily consumes trailing `.`/`-` (verified in bash: `tool 1.2.3-rc1.` sets `FJ_LIVEPRE=-rc1.` and `tool 1.2.3-rc.1. Built 2026` sets `FJ_LIVEPRE=-rc.1.`, whereas `tool 1.2.3.` strips the trailing `.` and sets `FJ_LIVE=1.2.3`, `FJ_LIVEPRE=`), diverging against `v1.2.3-rc1`."
  },
  {
    "claim": "2.15",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` and line 113 `Some(s) if s.starts_with(|c: char| c.is_ascii_alphabetic())` drop digit-leading SemVer 2.0.0 pre-release suffixes (`1.2.3-0.3.7`, `1.2.3-1`, `1.2.3-20261118` all yield empty `FJ_LIVEPRE` and `pre`, verified in bash)."
  },
  {
    "claim": "2.16",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:95 diverges on `[ \"$FJ_LIVEPRE\" != '{pre}' ]` (triggering apply for pin `v1.2.3-rc1` vs live `1.2.3`), while src/resources/github_release.rs:179-188 `downgrade_guard` compares only `$FJ_MAJ/$FJ_MIN/$FJ_PAT` (`1.2.3` vs `1.2.3`) and never checks `$FJ_LIVEPRE`, allowing silent downgrade of final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "2.17",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:23-28 allows ASCII uppercase in `is_safe_cargo_token` (`c.is_ascii_alphanumeric()`), while line 274 `BEGIN {{ gsub(/_/, \\\"-\\\", c) }} ... n = $1; gsub(/_/, \\\"-\\\", n); if (n == c ...)` omits `tolower` (verified in awk: `Foo_Bar` vs `foo-bar` and `inflector` vs `Inflector` return empty, hitting lines 277-281 `_FJ_NC=1`)."
  },
  {
    "claim": "3.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `awk -v c={q} 'BEGIN {{ gsub(/_/, \\\"-\\\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \\\"-\\\", n); if (n == c && $2 == \\\"=\\\" && $3 ~ /^\\\"[0-9A-Za-z._+-]+\\\"$/) ...` maps `_` to `-` in both `c` and `n` before comparing (verified in awk)."
  },
  {
    "claim": "3.2",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:61-70 `FJ_PRESENT=0; FJ_LIVE=''; FJ_LIVEPRE=''; FJ_VERLINE=''` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` capture `-<letter>[0-9A-Za-z.-]*` in `FJ_LIVEPRE` while keeping `FJ_LIVE` identical."
  },
  {
    "claim": "3.3",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/semver.rs:31-49 (`parse_semver_span(text).map(|(v, _)| v)`), src/tripwire/drift/version_pin.rs:49 (`pub use super::semver::{parse_semver, parse_semver_span, Semver};`), and src/resources/github_release_check.rs:92-96,106-118 (`let Some((_, end)) = parse_semver_span(tag)`, `if [ \"$FJ_NORM\" != '{want}' ] || [ \"$FJ_LIVEPRE\" != '{pre}' ]; then`) confirm all points."
  },
  {
    "claim": "3.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 confirm `launch_io::token` is removed, `apply_script_on` and `tail_on` take UTC `day`, and `apply_script` reads `SystemTime::now()` once and calls `apply_script_on`."
  },
  {
    "claim": "3.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:51-66 (`fn nothing_declared_is_byte_identical_to_today()`, line 59 `launch_io::token(`) and src/resources/cloud_instance/mod.rs:64 (`#[cfg(test)] mod tests_storage;`) confirm the `TODAY_TAIL` test cannot compile at the merge commit."
  },
  {
    "claim": "3.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:52,59-61 `let s = apply_script_on(&gated(std::path::Path::new(\"/t\")), DAY);` vs `let quoted = sh_squote(&launch_io::token(...));` confirms `tests_storage.rs` generates `s` for fixed `DAY` while line 59 still calls removed `launch_io::token`."
  },
  {
    "claim": "3.7",
    "verdict": "SURVIVED",
    "evidence": "ANGLE.diff:225-247 and src/resources/cloud_instance/tests_chain.rs:288-312 confirm both `if run_tokens(&log)[0] ... t1` guards in `adoption_uses_the_current_chain_token_and_a_rerun_walks_the_same_chain` were removed and assertions run unconditionally."
  },
  {
    "claim": "3.8",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8130-8219 contains `PMAT-1189`, `PMAT-1190`, and `PMAT-1198`, while `tree/.git` is absent so per-commit placement cannot be verified."
  },
  {
    "claim": "3.9",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59-61 (`launch_io::token(`) vs src/resources/cloud_instance/launch_io.rs:97-102 confirms the `## Tests` pass claim is false due to `E0425` compilation failure (note: `src/resources/cloud_instance/` has 133 tests, though `--lib cloud_instance` matches 139 including 6 in src/core/parser/tests_cloud_instance_validate.rs)."
  },
  {
    "claim": "3.10",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:20-22 lists separate-host clippy, fmt, mutation, and earlier-head results; `clippy --lib` without `--tests` skips `#[cfg(test)]` modules like `tests_storage.rs`, and the historical/mutation runs cannot be verified from the read-only merge tree."
  },
  {
    "claim": "3.11",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/version_pin.rs:63-70,146-154 compares only `N.N.N`, while modifying `live_version` (src/resources/github_release_check.rs:61-70,81-83) also changes the shell emitted via `live_version_unguarded` in src/resources/github_release.rs:79,189 (`apply_script`) and line 209 (`state_query_script`)."
  },
  {
    "claim": "3.12",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 (`182 + 61 = 243`, `54` roadmap, `189` non-roadmap) exactly."
  },
  {
    "claim": "3.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:52,59-62 still calls deleted `launch_io::token` (removed at src/resources/cloud_instance/launch_io.rs:97-102) in `#[cfg(test)] mod tests_storage` (mod.rs:64), breaking compilation (`E0425`) and mismatching `DAY` (`20_400`) vs today; no other caller of removed `token`, `tail`, or `is_todays` remains."
  },
  {
    "claim": "3.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:92-96 diverges for pin `v1.2.3-rc1` vs live `1.2.3`, while src/resources/github_release.rs:158,179-188 (`/// forjar#613: refuse to install a pin OLDER than the live binary.` / `$FJ_MAJ -gt {maj}` ... `$FJ_PAT -gt {pat}`) ignores `FJ_LIVEPRE` and allows downgrading final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "3.15",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:60-66 `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` remains even though src/resources/cloud_instance/tests_launch.rs:104 `let script = apply_script_on(&r, DAY);` fixes `DAY`, silently skipping the assertion if `t2[0]` changes."
  },
  {
    "claim": "3.16",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?([^.0-9]|[.]([^0-9]|$)|$)` consumes trailing `.` into `FJ_LIVEPRE` (`tool 1.2.3-rc1.` -> `FJ_LIVEPRE='-rc1.'`, verified in bash), falsely diverging against `tag: v1.2.3-rc1`, whereas `tool 1.2.3.` strips the trailing `.` and converges against `v1.2.3`."
  },
  {
    "claim": "3.17",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:4-5 (`a pre-release suffix (-rc1) on either side must equal the other side's`), lines 68-69 (`(-[A-Za-z][0-9A-Za-z.-]*)?`), and line 113 (`s.starts_with(|c: char| c.is_ascii_alphabetic())`) drop digit-leading SemVer 2.0.0 pre-releases like `v1.2.3-0.3.7` or `v1.2.3-1`, converging them against `1.2.3`."
  },
  {
    "claim": "3.18",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_launch.rs:11-12 (`/// The UTC day every script under test is generated for.`) and src/resources/cloud_instance/tests_chain.rs:4,29 (`the token's UTC day is injected (DAY).` vs `let s = apply_script(&gated(...));`, plus wall-clock `apply_script` calls across `tests_storage.rs`, `profile.rs`, `userdata.rs`, etc.) contradict those docs, and src/resources/cloud_instance/launch_io.rs:97-99 links publicly re-exported `pub fn apply_script` (mod.rs:41) to `pub(crate) fn apply_script_on` (mod.rs:404) with no lint suppression."
  }
]
VERDICT: FAIL
text_only: yes
refuter: 1
reviewer_model: gemini-4-neon-high

=== REFUTER 2 ===
[
  {
    "claim": "1.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59-61 `let quoted = sh_squote(&launch_io::token(` still calls `launch_io::token`, which was removed in src/resources/cloud_instance/launch_io.rs:97-102 while src/resources/cloud_instance/mod.rs:63-64 unconditionally includes `#[cfg(test)] mod tests_storage;`; no other callers of `token`, `tail`, or `is_todays` remain in the tree."
  },
  {
    "claim": "1.2",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `awk -v c={q} 'BEGIN {{ gsub(/_/, \"-\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \"-\", n); if (n == c && $2 == \"=\" && $3 ~ /^\"[0-9A-Za-z._+-]+\"$/)` normalizes `_` to `-` on both declared `c` and registry `n`."
  },
  {
    "claim": "1.3",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:68-70 `(-[A-Za-z][0-9A-Za-z.-]*)?` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` capture `-<letter>...` into `FJ_LIVEPRE` while preserving the extracted `N.N.N` in `FJ_LIVE` read by src/resources/github_release.rs:179-189."
  },
  {
    "claim": "1.4",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/semver.rs:31-49 `pub fn parse_semver(text: &str) -> Option<Semver> { parse_semver_span(text).map(|(v, _)| v) }`, src/tripwire/drift/version_pin.rs:49 `pub use super::semver::{parse_semver, parse_semver_span, Semver};`, and src/resources/github_release_check.rs:95-96,106-118 `let Some((_, end)) = parse_semver_span(tag)` / `if [ \"$FJ_NORM\" != '{want}' ] || [ \"$FJ_LIVEPRE\" != '{pre}' ]; then` confirm unchanged `parse_semver`, suffix extraction only after `N.N.N`, and divergence on mismatch."
  },
  {
    "claim": "1.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 `pub fn apply_script(resource: &Resource) -> String` / `pub(super) fn tail_on(s: &CloudInstance, day: u64) -> String` and src/resources/cloud_instance/mod.rs:41,404,489 `pub use launch_io::apply_script;` / `pub(crate) fn apply_script_on(resource: &Resource, day: u64) -> String` confirm `token()` is removed and `apply_script` reads `SystemTime::now()` once and delegates to `apply_script_on`."
  },
  {
    "claim": "1.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:51-66 `let s = apply_script_on(&gated(std::path::Path::new(\"/t\")), DAY);` and `let quoted = sh_squote(&launch_io::token(` fails to compile (`E0425`) and mixes fixed `DAY` (`20_400`) with wall-clock `launch_io::token`, blocking both golden tests including src/resources/cloud_instance/userdata.rs:186-194."
  },
  {
    "claim": "1.7",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:52,59-61 generates `s` with `apply_script_on(..., DAY)` while line 59 still calls removed `launch_io::token(...)` instead of `tok(...)`."
  },
  {
    "claim": "1.8",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:290-312 runs `assert_eq!(rc, 0, \"{out}\");` and `assert_eq!(t[2 * round - 2..], [t1.clone(), t2.clone()], \"{log}\");` unconditionally with both `if run_tokens(&log)[0] ... t1` guards removed (ANGLE.diff:225-249)."
  },
  {
    "claim": "1.9",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8130-8219 contains `- id: PMAT-1189`, `- id: PMAT-1190`, and `- id: PMAT-1198` (54 inserted lines), and `tree/.git` does not exist, so per-commit placement cannot be verified."
  },
  {
    "claim": "1.10",
    "verdict": "SURVIVED",
    "evidence": "src/resources/tests_package_e.rs:134-149 (2 `fj1189` tests) and src/resources/tests_github_release_verified.rs:36-106 (8 tests including `dash_before_the_version_is_not_a_prerelease`) exist, but src/resources/cloud_instance/tests_storage.rs:59 `launch_io::token(` prevents `--lib` test compilation."
  },
  {
    "claim": "1.11",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59 breaks `--lib` test compilation, and static `#[test]` counts matching substring `cloud_instance` total 139 (133 in `src/resources/cloud_instance/` plus 6 in src/core/parser/tests_cloud_instance_validate.rs:30-164 via src/core/parser/mod.rs:30-31 `#[cfg(test)] mod tests_cloud_instance_validate;`), while `github_release` matches 41 and `tripwire::drift` matches 133."
  },
  {
    "claim": "1.12",
    "verdict": "SURVIVED",
    "evidence": "Cargo.toml:24 `edition = \"2021\"` and `rustfmt --edition 2021 --check` exits 0 on all 13 changed `.rs` files (and `cargo fmt --all -- --check` exits 0), while `cargo clippy -j 4 --locked --lib -- -D warnings` is unverified in the read-only tree and skips `#[cfg(test)]` modules like src/resources/cloud_instance/mod.rs:63-64."
  },
  {
    "claim": "1.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `gsub(/_/, \"-\", n)` is needed by src/resources/tests_package_e.rs:143-149 (1 of 2 `fj1189` tests) and src/resources/github_release_check.rs:107 `parse_semver_span(tag)` is needed by src/resources/tests_github_release_verified.rs:100-106 (1 of 8 tests), while running mutants at the merge commit is blocked by src/resources/cloud_instance/tests_storage.rs:59."
  },
  {
    "claim": "1.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/disk_budget/tests/execution.rs:189 `fn a_cargo_target_whose_binary_is_running_survives()` exists untouched by ANGLE.diff, `src/resources/` contains 961 `#[test]` functions, and earlier-head historical runs cannot be directly verified from the exported merge tree."
  },
  {
    "claim": "1.15",
    "verdict": "REFUTED",
    "evidence": "src/resources/github_release_check.rs:61-70,81-83 modifies `live_version` (wrapped by `live_version_unguarded`), which is also embedded by src/resources/github_release.rs:79,150,152,189 (`apply_script` and `downgrade_guard`) and src/resources/github_release.rs:209,212 (`state_query_script`), refuting the claim that this PR changes only the check script in `github_release_check.rs` plus the `parse_semver_span` re-export."
  },
  {
    "claim": "1.16",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match 182 + 61 = 243 changed lines total and 243 - 54 = 189 outside the roadmap."
  },
  {
    "claim": "2.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:52,59-62 calls removed `launch_io::token(` (deleted in src/resources/cloud_instance/launch_io.rs:97-102) while line 52 uses fixed `DAY` (`20_400`, src/resources/cloud_instance/tests_launch.rs:12), causing `E0425` and a token-day mismatch on line 62; no other callers of `token` or `tail` remain in `src/`."
  },
  {
    "claim": "2.2",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:11,19 claims the `TODAY_TAIL` golden test and `--lib` test suites pass, but src/resources/cloud_instance/tests_storage.rs:59-61 calls removed `launch_io::token(`, breaking `--lib` test compilation."
  },
  {
    "claim": "2.3",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `awk -v c={q} 'BEGIN {{ gsub(/_/, \"-\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \"-\", n); if (n == c && $2 == \"=\" && $3 ~ /^\"[0-9A-Za-z._+-]+\"$/)` normalizes `_` to `-` in both `c` and `$1` before comparing."
  },
  {
    "claim": "2.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:61-70,92-96,106-118, src/tripwire/drift/semver.rs:31-50, and src/tripwire/drift/version_pin.rs:49 implement `FJ_LIVEPRE` capture without altering `FJ_LIVE`, `parse_semver_span`-based `pin_prerelease` with unchanged `parse_semver`, and divergence when `FJ_LIVEPRE != pre`."
  },
  {
    "claim": "2.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 remove `token()`, parameterize `apply_script_on` and `tail_on` by `day: u64`, and have `apply_script` read `SystemTime::now()` once and delegate to `apply_script_on`."
  },
  {
    "claim": "2.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:290-294,307-311 execute the adoption and rerun assertions unconditionally after removing both `if run_tokens(&log)[0] ... t1` guards (ANGLE.diff:225-249)."
  },
  {
    "claim": "2.7",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8130,8166,8202 each contain exactly one entry for `PMAT-1189`, `PMAT-1190`, and `PMAT-1198` with no duplicates."
  },
  {
    "claim": "2.8",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:13,20-22 claims per-commit roadmap placement, `cargo clippy --lib`, mutation kills, and earlier-head results, which cannot be verified from the read-only exported merge tree lacking `.git` history and build execution."
  },
  {
    "claim": "2.9",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:20 `cargo fmt --all -- --check: clean` is confirmed: both `rustfmt --edition 2021 --check` on all 13 changed `.rs` files and `cargo fmt --all -- --check` on the workspace exit 0."
  },
  {
    "claim": "2.10",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/version_pin.rs:49,67,151,246 uses `parse_semver` and `(Live::Version(l), Some(e)) if *l != e`, so `version_pin` still compares only `N.N.N` (`Semver`)."
  },
  {
    "claim": "2.11",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 exactly."
  },
  {
    "claim": "2.12",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:60-66 still guards `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` even though `launch_with` injects fixed `DAY` via `fixture` (src/resources/cloud_instance/tests_launch.rs:104), so a changed `t2[0]` silently bypasses the assertion."
  },
  {
    "claim": "2.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102 `pub fn apply_script(resource: &Resource) -> String` has no test asserting the UTC day or token it computes; all remaining test callers of `apply_script` ignore the token value or only check `--client-token fj-` (src/resources/cloud_instance/tests_storage.rs:119)."
  },
  {
    "claim": "2.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` greedily consumes trailing `.` (verified in bash: `tool 1.2.3-rc1.` sets `FJ_LIVEPRE='-rc1.'`, `tool 1.2.3-rc.1. Built 2026` sets `FJ_LIVEPRE='-rc.1.'`), diverging against `pin_prerelease(\"v1.2.3-rc1\")` (`-rc1`) while `tool 1.2.3.` strips the trailing `.`."
  },
  {
    "claim": "2.15",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` and line 113 `Some(s) if s.starts_with(|c: char| c.is_ascii_alphabetic())` require an ASCII letter after `-`, so numeric-leading SemVer 2.0.0 pre-releases (`v1.2.3-0.3.7`, `v1.2.3-1`, `v1.2.3-20261118`) yield empty `pre` and `FJ_LIVEPRE`."
  },
  {
    "claim": "2.16",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:95 diverges on `\"$FJ_LIVEPRE\" != '{pre}'` when live `1.2.3` meets pin `v1.2.3-rc1`, whereas src/resources/github_release.rs:179-188 `downgrade_guard` compares only `$FJ_MAJ`/`$FJ_MIN`/`$FJ_PAT` (`1.2.3 > 1.2.3` is false) and allows installing `v1.2.3-rc1` over final `1.2.3`."
  },
  {
    "claim": "2.17",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:23-28 allows uppercase letters in `is_safe_cargo_token`, and line 274 `BEGIN {{ gsub(/_/, \"-\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \"-\", n); if (n == c` omits `tolower`, leaving `Foo_Bar` vs `foo-bar` and `inflector` vs `Inflector` unresolved (verified with `awk`)."
  },
  {
    "claim": "3.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `BEGIN {{ gsub(/_/, \"-\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \"-\", n); if (n == c` maps `_` to `-` in both declared `c` and registry `n`."
  },
  {
    "claim": "3.2",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:61-70 `FJ_PRESENT=0; FJ_LIVE=''; FJ_LIVEPRE=''; FJ_VERLINE=''`, `(-[A-Za-z][0-9A-Za-z.-]*)?`, and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` populate `FJ_LIVEPRE` while keeping `FJ_LIVE` identical."
  },
  {
    "claim": "3.3",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/semver.rs:31-49, src/tripwire/drift/version_pin.rs:49, and src/resources/github_release_check.rs:92-96,106-118 confirm `parse_semver_span`, unchanged `parse_semver`, `pin_prerelease` ignoring `-` before `N.N.N`, and divergence on `FJ_LIVEPRE != pre`."
  },
  {
    "claim": "3.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 confirm removal of `token`, UTC-day injection in `apply_script_on`/`tail_on`, and single clock read in `apply_script`."
  },
  {
    "claim": "3.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:51-66 `fn nothing_declared_is_byte_identical_to_today()` calls removed `launch_io::token(` at line 59 inside `#[cfg(test)] mod tests_storage;` (src/resources/cloud_instance/mod.rs:64), falsifying the PR body claim that the golden tests still pass."
  },
  {
    "claim": "3.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:52,59-61 computes `s` with `apply_script_on(..., DAY)` while line 59 still calls removed `launch_io::token(` instead of `tok`."
  },
  {
    "claim": "3.7",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:288-312 runs the adoption and rerun assertions unconditionally after removing both midnight guards."
  },
  {
    "claim": "3.8",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8130-8219 has all three roadmap rows, and the exported tree lacks `.git`, so per-commit placement cannot be verified."
  },
  {
    "claim": "3.9",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59-61 `launch_io::token(` fails `--lib` test compilation (`E0425`) against src/resources/cloud_instance/launch_io.rs:97-102, falsifying `## Tests` (note: `src/resources/cloud_instance/` has 133 `#[test]` functions, while `--lib cloud_instance` matches 139 including 6 in src/core/parser/tests_cloud_instance_validate.rs:30-164)."
  },
  {
    "claim": "3.10",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:20-22 separate-host `cargo clippy --lib` (which skips `#[cfg(test)]`), mutation kills, and earlier-head results cannot be re-executed/verified in this read-only review tree."
  },
  {
    "claim": "3.11",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:61-70,81-83 modifies `live_version` (via `live_version_unguarded`), which is embedded into `apply_script` at src/resources/github_release.rs:79,150,152,189 and `state_query_script` at src/resources/github_release.rs:209,212, falsifying \"this PR changes the check script only\"."
  },
  {
    "claim": "3.12",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29."
  },
  {
    "claim": "3.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:52,59-62 still calls deleted `launch_io::token(` (removed in src/resources/cloud_instance/launch_io.rs:97-102) inside `#[cfg(test)] mod tests_storage;` (src/resources/cloud_instance/mod.rs:64), breaking `--lib` test compilation (`E0425`) and mismatching `DAY` (`20_400`); no other callers of removed `token`, `tail`, or `is_todays` remain."
  },
  {
    "claim": "3.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:92-96 diverges on live `1.2.3` vs pin `v1.2.3-rc1`, and src/resources/github_release.rs:158,179-188 `downgrade_guard` ignores `FJ_LIVEPRE` and sees `1.2.3 > 1.2.3` as false, permitting a downgrade from final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "3.15",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:60-66 keeps `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` despite `launch_with` injecting fixed `DAY` via `fixture` (src/resources/cloud_instance/tests_launch.rs:104), allowing a determinism regression in `t2[0]` to skip the check."
  },
  {
    "claim": "3.16",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` consumes a trailing period on `tool 1.2.3-rc1.` into `FJ_LIVEPRE='-rc1.'` (verified in bash), causing false divergence against `tag: v1.2.3-rc1`, whereas `tool 1.2.3.` strips the period via `[.]([^0-9]|$)`."
  },
  {
    "claim": "3.17",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:4-5 states `a pre-release suffix (-rc1) on either side must equal the other side's;` without qualification, while lines 68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` and line 113 `s.starts_with(|c: char| c.is_ascii_alphabetic())` drop numeric-leading SemVer 2.0.0 pre-releases (`v1.2.3-0.3.7`, `v1.2.3-1`)."
  },
  {
    "claim": "3.18",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_launch.rs:11 `/// The UTC day every script under test is generated for.` and src/resources/cloud_instance/tests_chain.rs:4 `the token's UTC day is injected (DAY).` conflict with wall-clock `apply_script` calls at src/resources/cloud_instance/tests_chain.rs:29 and across sibling test modules (`tests_storage.rs`, `profile.rs`, `userdata.rs`, `tests.rs`, `tests_b1.rs`, `tests_b4.rs`), and src/resources/cloud_instance/launch_io.rs:97-99 links from publicly re-exported `pub fn apply_script` (src/lib.rs:41, src/resources/mod.rs:10, src/resources/cloud_instance/mod.rs:41) to `pub(crate) fn apply_script_on` (src/resources/cloud_instance/mod.rs:404)."
  }
]
VERDICT: FAIL
text_only: yes
refuter: 2
reviewer_model: gemini-4-neon-high

=== REFUTER 3 ===
[
  {
    "claim": "1.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59-61 `let quoted = sh_squote(&launch_io::token(` still calls `launch_io::token`, which was deleted in src/resources/cloud_instance/launch_io.rs:97-102, inside unconditional `#[cfg(test)] mod tests_storage;` at src/resources/cloud_instance/mod.rs:63-64; grep across src/ and tests/ confirms no other callers of `launch_io::token`, `launch_io::tail`, or `is_todays` remain."
  },
  {
    "claim": "1.2",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `awk -v c={q} 'BEGIN {{ gsub(/_/, \\\"-\\\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \\\"-\\\", n); if (n == c && $2 == \\\"=\\\" && $3 ~ /^\\\"[0-9A-Za-z._+-]+\\\"$/) {{ gsub(/\\\"/, \\\"\\\", $3); print $3; exit }} }}'` normalizes `_` to `-` in both `c` and `n`; verified with awk for both `foo_bar` vs `foo-bar` and `foo-bar` vs `foo_bar`."
  },
  {
    "claim": "1.3",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:67-70 `(-[A-Za-z][0-9A-Za-z.-]*)?([^.0-9]|[.]([^0-9]|$)|$)` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` capture `-<letter>...` into `FJ_LIVEPRE` while keeping `FJ_LIVE` (used by `downgrade_guard` at src/resources/github_release.rs:179-189 `if [ -n \"$FJ_LIVE\" ]; then`) identical."
  },
  {
    "claim": "1.4",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/semver.rs:31-50 `pub fn parse_semver(text: &str) -> Option<Semver> { parse_semver_span(text).map(|(v, _)| v) }`, src/tripwire/drift/version_pin.rs:49 `pub use super::semver::{parse_semver, parse_semver_span, Semver};`, and src/resources/github_release_check.rs:92-97,106-118 `let Some((_, end)) = parse_semver_span(tag)` and `if [ \"$FJ_NORM\" != '{want}' ] || [ \"$FJ_LIVEPRE\" != '{pre}' ]; then` confirm all stated behavior."
  },
  {
    "claim": "1.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102 `pub fn apply_script(resource: &Resource) -> String { let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH); super::apply_script_on(resource, day_of(now.map_or(0, |d| d.as_secs()))) }`, launch_io.rs:181,198 `pub(super) fn tail_on(s: &CloudInstance, day: u64) -> String`, and src/resources/cloud_instance/mod.rs:41,404,489 confirm `token()` removal and unchanged per-day output."
  },
  {
    "claim": "1.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:51-66 `let s = apply_script_on(&gated(std::path::Path::new(\"/t\")), DAY);` paired with line 59 `let quoted = sh_squote(&launch_io::token(` causes E0425 compilation failure blocking both tests, and would mismatch `DAY` (`20_400`) vs wall-clock day on lines 62-66 even if `token` existed."
  },
  {
    "claim": "1.7",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:52,59-61 generates `s` with `apply_script_on(..., DAY)` while line 59 still calls the removed wall-clock `launch_io::token(...)` instead of `tok(...)`."
  },
  {
    "claim": "1.8",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:289-312 runs `assert_eq!(rc, 0, \"{out}\");` and `assert_eq!(t[2 * round - 2..], [t1.clone(), t2.clone()], \"{log}\");` unconditionally with both `if run_tokens(&log)[0] ... t1` guards removed (confirmed in ANGLE.diff:225-249)."
  },
  {
    "claim": "1.9",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8130-8147 (`- id: PMAT-1189`), 8166-8183 (`- id: PMAT-1190`), and 8202-8219 (`- id: PMAT-1198`) contain the 54 inserted lines, and `tree/.git` is absent so per-commit placement cannot be verified."
  },
  {
    "claim": "1.10",
    "verdict": "SURVIVED",
    "evidence": "src/resources/tests_package_e.rs:134-149 (2 `fj1189` tests) and src/resources/tests_github_release_verified.rs:36-106 (8 tests including `dash_before_the_version_is_not_a_prerelease`) exist as stated, while src/resources/cloud_instance/tests_storage.rs:59 `launch_io::token(` breaks `--lib` test compilation."
  },
  {
    "claim": "1.11",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59 breaks `--lib` test compilation, and static `#[test]` counts match: `src/resources/cloud_instance/` (133) + `src/core/parser/tests_cloud_instance_validate.rs:30-164` (6, included at `src/core/parser/mod.rs:30-31`) = 139 for filter `cloud_instance`, 41 for `github_release`, and 133 for `tripwire::drift`."
  },
  {
    "claim": "1.12",
    "verdict": "SURVIVED",
    "evidence": "Cargo.toml:24 `edition = \"2021\"`; running `rustfmt --edition 2021 --check` on all 13 changed `.rs` files exited 0, and `clippy --lib` without `--tests` excludes `#[cfg(test)] mod tests_storage;` at src/resources/cloud_instance/mod.rs:63-64."
  },
  {
    "claim": "1.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `gsub(/_/, \"-\", n)` is required by src/resources/tests_package_e.rs:143-149 (`fj1189_hyphen_declared_matches_underscore_in_registry`, 1 of 2 `fj1189` tests), and src/resources/github_release_check.rs:106-118 `parse_semver_span(tag)` is required by src/resources/tests_github_release_verified.rs:99-106 (`dash_before_the_version_is_not_a_prerelease`, 1 of 8 tests), while execution is blocked by `tests_storage.rs:59`."
  },
  {
    "claim": "1.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/disk_budget/tests/execution.rs:189 `fn a_cargo_target_whose_binary_is_running_survives()` exists untouched by ANGLE.diff, `src/resources/` contains 961 `#[test]` functions, and historical earlier-head runs cannot be directly verified from the merge snapshot."
  },
  {
    "claim": "1.15",
    "verdict": "REFUTED",
    "evidence": "src/resources/github_release.rs:79,189,209 calls `super::github_release_check::live_version_unguarded(...)`, which wraps `live_version` (`src/resources/github_release_check.rs:81-83`) modified by this PR at `src/resources/github_release_check.rs:61-70`; therefore the claim that this PR changes only the check script in `github_release_check.rs` plus the `parse_semver_span` re-export is false because `github_release::apply_script` and `github_release::state_query_script` also emit the modified `live_version` shell."
  },
  {
    "claim": "1.16",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` confirm 182+61=243 changed lines, 54 roadmap insertions, and 189 non-roadmap changed lines."
  },
  {
    "claim": "2.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59-61 `let quoted = sh_squote(&launch_io::token(` still calls deleted `launch_io::token` (removed in src/resources/cloud_instance/launch_io.rs:97-102), causing E0425 and conflicting with line 52 `apply_script_on(..., DAY)` (`DAY = 20_400`); no other callers of `token` or `tail` remain in `src/`."
  },
  {
    "claim": "2.2",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:11,19 claims the golden `TODAY_TAIL` test and `--lib` test suites pass, contradicted by src/resources/cloud_instance/tests_storage.rs:51-66 calling removed `launch_io::token` at line 59 and preventing `--lib` test compilation."
  },
  {
    "claim": "2.3",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `BEGIN {{ gsub(/_/, \\\"-\\\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \\\"-\\\", n); if (n == c ...` resolves `foo_bar` against `foo-bar` and vice versa (verified with awk)."
  },
  {
    "claim": "2.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:61-70,92-96,106-118, src/tripwire/drift/semver.rs:31-50, and src/tripwire/drift/version_pin.rs:49 confirm `FJ_LIVEPRE` capture without altering `FJ_LIVE`, `pin_prerelease` using `parse_semver_span` (ignoring `-` before `N.N.N`), and divergence on mismatch."
  },
  {
    "claim": "2.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 confirm `token()` removal, `apply_script_on`/`tail_on` taking `day: u64`, and `apply_script` reading `SystemTime::now()` once and delegating to `apply_script_on`."
  },
  {
    "claim": "2.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:290-294,307-311 shows both `if run_tokens(&log)[0] ... t1` guards removed so all assertions execute unconditionally."
  },
  {
    "claim": "2.7",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8130 (`- id: PMAT-1189`), 8166 (`- id: PMAT-1190`), and 8202 (`- id: PMAT-1198`) are present with exactly one occurrence each."
  },
  {
    "claim": "2.8",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:13,20-22 claims per-commit roadmap placement, `clippy --lib`, mutation kills, and earlier-head runs; `tree/.git` is absent and `clippy --lib` skips `#[cfg(test)]`."
  },
  {
    "claim": "2.9",
    "verdict": "SURVIVED",
    "evidence": "Running `rustfmt --edition 2021 --check` on all 13 changed `.rs` files in `tree/` and `cargo fmt --all -- --check` on the workspace both exited 0."
  },
  {
    "claim": "2.10",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/version_pin.rs:49,67,151,246 (`parse_semver(tag)`, `(Live::Version(l), Some(e)) if *l != e`, `parse_semver(text)`) confirms `version_pin` still compares only `N.N.N` (`Semver`)."
  },
  {
    "claim": "2.11",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` matches PR_BODY.md:29 exactly."
  },
  {
    "claim": "2.12",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:64-66 `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` remains despite `launch_with` injecting fixed `DAY` via `fixture` at src/resources/cloud_instance/tests_launch.rs:104 (`apply_script_on(&r, DAY)`), silently skipping the determinism check if `t2[0] != t[0]`."
  },
  {
    "claim": "2.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102 computes `day_of(now.map_or(0, |d| d.as_secs()))`, and grep across `src/` and `tests/` confirms no test verifies the UTC day or full token emitted by `launch_io::apply_script` (only prefix `--client-token fj-` at src/resources/cloud_instance/tests_storage.rs:119)."
  },
  {
    "claim": "2.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` greedily consumes trailing punctuation; verified in bash that `tool 1.2.3-rc1.` sets `FJ_LIVEPRE=-rc1.` and `tool 1.2.3-rc.1. Built 2026` sets `FJ_LIVEPRE=-rc.1.`, whereas `tool 1.2.3.` strips the trailing `.`."
  },
  {
    "claim": "2.15",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` and line 113 `Some(s) if s.starts_with(|c: char| c.is_ascii_alphabetic())` require an ASCII letter after `-`, so numeric-leading SemVer 2.0.0 pre-releases like `v1.2.3-0.3.7` and `v1.2.3-1` yield empty `pre` and `FJ_LIVEPRE` (verified in bash)."
  },
  {
    "claim": "2.16",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:95 `if [ \"$FJ_NORM\" != '{want}' ] || [ \"$FJ_LIVEPRE\" != '{pre}' ]; then` diverges for pin `v1.2.3-rc1` vs live `1.2.3`, while src/resources/github_release.rs:179-188 only compares `$FJ_MAJ/$FJ_MIN/$FJ_PAT` (`1.2.3 > 1.2.3` is false) and ignores `FJ_LIVEPRE`, permitting downgrade from final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "2.17",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:23-28 permits `[A-Za-z0-9._-]` in crate names and line 274 `gsub(/_/, \"-\", c)` / `gsub(/_/, \"-\", n)` omits `tolower`, leaving case-differing crate names like `Foo_Bar` vs `foo-bar` unresolved (verified with awk)."
  },
  {
    "claim": "3.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/package/cargo.rs:274 `BEGIN {{ gsub(/_/, \\\"-\\\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \\\"-\\\", n); if (n == c ...` normalizes `_` to `-` on both sides and resolves `foo_bar` against `foo-bar` and vice versa (verified with awk)."
  },
  {
    "claim": "3.2",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:61-70 `FJ_PRESENT=0; FJ_LIVE=''; FJ_LIVEPRE=''; FJ_VERLINE=''` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` store `-<letter>[0-9A-Za-z.-]*` in `FJ_LIVEPRE` while leaving `FJ_LIVE` unchanged."
  },
  {
    "claim": "3.3",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/semver.rs:31-49, src/tripwire/drift/version_pin.rs:49, and src/resources/github_release_check.rs:92-96,106-118 confirm `parse_semver_span`, unchanged `parse_semver`, `pin_prerelease` ignoring leading dashes like `cli2-v1.2.3`, and divergence on `FJ_LIVEPRE != pre`."
  },
  {
    "claim": "3.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 confirm `launch_io::token` removal, `apply_script_on`/`tail_on` taking `day`, and `apply_script` delegating with a single `SystemTime::now()` read."
  },
  {
    "claim": "3.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:51-66 (`fn nothing_declared_is_byte_identical_to_today()`, line 59 `launch_io::token(`) and src/resources/cloud_instance/mod.rs:64 (`#[cfg(test)] mod tests_storage;`) prevent compilation at the merge commit."
  },
  {
    "claim": "3.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:52 uses `apply_script_on(..., DAY)` (`DAY = 20_400`) while lines 59-61 still call removed wall-clock `launch_io::token(...)` instead of `tok`."
  },
  {
    "claim": "3.7",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:288-312 removes both `if run_tokens(&log)[0] ... t1` midnight guards so all assertions run unconditionally."
  },
  {
    "claim": "3.8",
    "verdict": "SURVIVED",
    "evidence": "docs/roadmaps/roadmap.yaml:8130-8219 contains `PMAT-1189`, `PMAT-1190`, and `PMAT-1198`, and `tree/.git` is absent so per-commit placement cannot be verified."
  },
  {
    "claim": "3.9",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59-61 `launch_io::token(` causes `E0425` in `#[cfg(test)] mod tests_storage` so `cargo test -j 4 --locked --lib <filter>` cannot pass at the merge commit (note: `src/resources/cloud_instance/` has 133 `#[test]` functions, though `--lib cloud_instance` also matches 6 tests in `src/core/parser/tests_cloud_instance_validate.rs` for 139 total)."
  },
  {
    "claim": "3.10",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:20-22 lists separate-host `clippy --lib`, `cargo fmt`, mutant, and earlier-head results; `clippy --lib` without `--tests` does not compile `#[cfg(test)]` modules such as `tests_storage.rs`."
  },
  {
    "claim": "3.11",
    "verdict": "SURVIVED",
    "evidence": "src/tripwire/drift/version_pin.rs:63-70,146-154 compares only `N.N.N`, while src/resources/github_release.rs:79,189,209 calls `super::github_release_check::live_version_unguarded(...)`, meaning modifying `live_version` at src/resources/github_release_check.rs:61-70 also changes the shell emitted by `apply_script` and `state_query_script`."
  },
  {
    "claim": "3.12",
    "verdict": "SURVIVED",
    "evidence": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` matches PR_BODY.md:29 (`182 + 61 = 243`, `54` roadmap, `189` outside)."
  },
  {
    "claim": "3.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_storage.rs:59-61 calls deleted `launch_io::token` (removed in src/resources/cloud_instance/launch_io.rs:97-102) inside `#[cfg(test)] mod tests_storage;` (src/resources/cloud_instance/mod.rs:64), breaking compilation (`E0425`) and mismatching `DAY` at line 52; no other caller of `token`, `tail`, or `is_todays` remains."
  },
  {
    "claim": "3.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:92-96 diverges on `v1.2.3-rc1` pin vs live `1.2.3`, while src/resources/github_release.rs:158,179-188 `downgrade_guard` checks only `$FJ_MAJ/$FJ_MIN/$FJ_PAT` and ignores `FJ_LIVEPRE`, allowing silent downgrade from final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "3.15",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_chain.rs:64-66 `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` leaves a third midnight guard despite fixed `DAY` at src/resources/cloud_instance/tests_launch.rs:104, masking non-deterministic `t2[0]`."
  },
  {
    "claim": "3.16",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` consumes trailing `.` on `tool 1.2.3-rc1.` into `FJ_LIVEPRE='-rc1.'` (verified in bash), diverging against `v1.2.3-rc1`, whereas `tool 1.2.3.` strips the trailing `.` and converges against `v1.2.3`."
  },
  {
    "claim": "3.17",
    "verdict": "SURVIVED",
    "evidence": "src/resources/github_release_check.rs:4-5 (`a pre-release suffix (-rc1) on either side must equal the other side's`), lines 68-69 (`(-[A-Za-z][0-9A-Za-z.-]*)?`), and line 113 (`s.starts_with(|c: char| c.is_ascii_alphabetic())`) drop numeric-leading SemVer 2.0.0 pre-releases such as `v1.2.3-0.3.7` and `v1.2.3-1` (verified in bash)."
  },
  {
    "claim": "3.18",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_launch.rs:11-12 (`/// The UTC day every script under test is generated for.`) and src/resources/cloud_instance/tests_chain.rs:4 contradict wall-clock `apply_script(...)` calls at tests_chain.rs:29, tests_storage.rs, profile.rs, and userdata.rs; and src/resources/cloud_instance/launch_io.rs:97-99 links from `pub fn apply_script` (re-exported at mod.rs:41) to `pub(crate) fn apply_script_on` (mod.rs:404), triggering `rustdoc::private_intra_doc_links`."
  }
]
VERDICT: FAIL
text_only: yes
refuter: 3
reviewer_model: gemini-4-neon-high
