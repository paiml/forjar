=== JUDGE 1 ===
[
  {
    "claim": "1.1",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:59-61 `let quoted = sh_squote(&launch_io::token(&spec_of(&gated(std::path::Path::new(\"/t\"))).unwrap()));` calls `launch_io::token`, which was deleted at src/resources/cloud_instance/launch_io.rs:97-102, inside unconditional `#[cfg(test)] mod tests_storage;` at src/resources/cloud_instance/mod.rs:63-64; no other callers of `token`, `tail`, or `is_todays` remain."
  },
  {
    "claim": "1.2",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `awk -v c={q} 'BEGIN {{ gsub(/_/, \\\"-\\\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \\\"-\\\", n); if (n == c && $2 == \\\"=\\\" && $3 ~ /^\\\"[0-9A-Za-z._+-]+\\\"$/) ...` maps `_` to `-` in both `c` and `n = $1` before comparing `n == c` (verified in awk for both directions)."
  },
  {
    "claim": "1.3",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:68-70 `(-[A-Za-z][0-9A-Za-z.-]*)?([^.0-9]|[.]([^0-9]|$)|$)` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` capture `-<letter>...` into `FJ_LIVEPRE` while leaving `FJ_LIVE` (read by `downgrade_guard` at src/resources/github_release.rs:179-189 `if [ -n \"$FJ_LIVE\" ]; then`) unchanged."
  },
  {
    "claim": "1.4",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/semver.rs:31-50 `pub fn parse_semver(text: &str) -> Option<Semver> { parse_semver_span(text).map(|(v, _)| v) }` and `return Some((v, end));`, src/tripwire/drift/version_pin.rs:49 `pub use super::semver::{parse_semver, parse_semver_span, Semver};`, and src/resources/github_release_check.rs:95,107-118 (`let Some((_, end)) = parse_semver_span(tag)`, `if [ \"$FJ_NORM\" != '{want}' ] || [ \"$FJ_LIVEPRE\" != '{pre}' ]; then`) confirm all points."
  },
  {
    "claim": "1.5",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102 `pub fn apply_script(resource: &Resource) -> String { let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH); super::apply_script_on(resource, day_of(now.map_or(0, |d| d.as_secs()))) }`, launch_io.rs:181,198 `pub(super) fn tail_on(s: &CloudInstance, day: u64) -> String`, and src/resources/cloud_instance/mod.rs:41,404,489 confirm `token()` removal and unchanged per-day output."
  },
  {
    "claim": "1.6",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:51-66 `let s = apply_script_on(&gated(std::path::Path::new(\"/t\")), DAY);` paired with line 59 `let quoted = sh_squote(&launch_io::token(` causes `E0425` blocking both golden tests (including src/resources/cloud_instance/userdata.rs:186-194) and mixes fixed `DAY` (`20_400`) with wall-clock `token`."
  },
  {
    "claim": "1.7",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-61 `let s = apply_script_on(&gated(std::path::Path::new(\"/t\")), DAY);` vs `let quoted = sh_squote(&launch_io::token(` confirms `nothing_declared_is_byte_identical_to_today` still uses removed wall-clock `launch_io::token` instead of `tok(...)`."
  },
  {
    "claim": "1.8",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:289-312 runs `assert_eq!(rc, 0, \"{out}\");` and `assert_eq!(t[2 * round - 2..], [t1.clone(), t2.clone()], \"{log}\");` unconditionally after removing both `if run_tokens(&log)[0] ... t1` guards (ANGLE.diff:225-249)."
  },
  {
    "claim": "1.9",
    "verdict": "SURVIVED",
    "reason": "docs/roadmaps/roadmap.yaml:8130-8147 (`- id: PMAT-1189`), 8166-8183 (`- id: PMAT-1190`), and 8202-8219 (`- id: PMAT-1198`) contain the 54 inserted lines, and `tree/.git` is absent so per-commit placement cannot be verified."
  },
  {
    "claim": "1.10",
    "verdict": "SURVIVED",
    "reason": "src/resources/tests_package_e.rs:134-149 (2 `fj1189` tests) and src/resources/tests_github_release_verified.rs:36-106 (8 tests including `dash_before_the_version_is_not_a_prerelease`) exist as stated, but src/resources/cloud_instance/tests_storage.rs:59 `launch_io::token(` breaks `--lib` test compilation."
  },
  {
    "claim": "1.11",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:59 `launch_io::token(` breaks `--lib` test compilation, and filter `cloud_instance` matches 139 `--lib` tests (133 in `src/resources/cloud_instance/` plus 6 in src/core/parser/tests_cloud_instance_validate.rs:30-164 via src/core/parser/mod.rs:30-31 `#[cfg(test)] mod tests_cloud_instance_validate;`), while `github_release` matches 41 and `tripwire::drift` matches 133."
  },
  {
    "claim": "1.12",
    "verdict": "SURVIVED",
    "reason": "Cargo.toml:24 `edition = \"2021\"` and `rustfmt --edition 2021 --check` exits 0 on all 13 changed `.rs` files; `cargo clippy --lib` without `--tests` skips `#[cfg(test)] mod tests_storage;` at src/resources/cloud_instance/mod.rs:63-64."
  },
  {
    "claim": "1.13",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `gsub(/_/, \\\"-\\\", n)` is exercised by src/resources/tests_package_e.rs:143-149 (1 of 2 `fj1189` tests) and src/resources/github_release_check.rs:107 `parse_semver_span(tag)` by src/resources/tests_github_release_verified.rs:100-106 (1 of 8 tests), while execution is blocked by src/resources/cloud_instance/tests_storage.rs:59."
  },
  {
    "claim": "1.14",
    "verdict": "SURVIVED",
    "reason": "src/resources/disk_budget/tests/execution.rs:189 `fn a_cargo_target_whose_binary_is_running_survives()` exists untouched by ANGLE.diff, `src/resources/` contains 961 `#[test]` functions, and earlier-head historical runs cannot be directly verified from the merge tree."
  },
  {
    "claim": "1.15",
    "verdict": "REFUTED",
    "reason": "src/resources/github_release_check.rs:61-70,81-83 modifies `live_version` (wrapped by `live_version_unguarded`), which is called by src/resources/github_release.rs:79 (`let post = super::github_release_check::live_version_unguarded(&bin_path, false);` in `apply_script`), line 189 (`downgrade_guard` in `apply_script`), and line 209 (`state_query_script`), and src/tripwire/drift/semver.rs:31-50 is also changed; thus the assertion that this PR changes only the check script in `github_release_check.rs` plus the `parse_semver_span` re-export is false."
  },
  {
    "claim": "1.16",
    "verdict": "SURVIVED",
    "reason": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 (`182 insertions, 61 deletions = 243 changed lines in 14 files`, `54` roadmap, `189` outside) and ANGLE.diff exactly."
  },
  {
    "claim": "2.1",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-62 calls `apply_script_on(..., DAY)` (`DAY = 20_400` at tests_launch.rs:12) and deleted `launch_io::token(` (removed in src/resources/cloud_instance/launch_io.rs:97-102), causing `E0425` and a `DAY` vs today mismatch on line 62; no other callers of `token` or `tail` remain in `src/`."
  },
  {
    "claim": "2.2",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:11,19 claims `TODAY_TAIL` and `--lib` test suites pass, falsified by src/resources/cloud_instance/tests_storage.rs:51-66 calling removed `launch_io::token(` at line 59 and preventing `--lib` test compilation."
  },
  {
    "claim": "2.3",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `BEGIN {{ gsub(/_/, \\\"-\\\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \\\"-\\\", n); if (n == c ...` maps `_` to `-` in both declared and search crate names (verified in awk)."
  },
  {
    "claim": "2.4",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:61-70,92-96,106-118 (`FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"`, `pin_prerelease`), src/tripwire/drift/semver.rs:31-50 (`parse_semver_span`), and src/tripwire/drift/version_pin.rs:49 confirm `FJ_LIVEPRE` capture, unchanged `FJ_LIVE`/`parse_semver`, ignoring `-` before `N.N.N`, and divergence on mismatch."
  },
  {
    "claim": "2.5",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 remove `token()`, pass `day: u64` to `apply_script_on`/`tail_on`, and have `apply_script` read `SystemTime::now()` once and delegate to `apply_script_on`."
  },
  {
    "claim": "2.6",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:290-294,307-311 execute the adoption and rerun assertions unconditionally after removing both `if run_tokens(&log)[0] ... t1` guards (ANGLE.diff:225-249)."
  },
  {
    "claim": "2.7",
    "verdict": "SURVIVED",
    "reason": "docs/roadmaps/roadmap.yaml:8130 (`- id: PMAT-1189`), 8166 (`- id: PMAT-1190`), and 8202 (`- id: PMAT-1198`) each occur exactly once with no duplicates."
  },
  {
    "claim": "2.8",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:13,20-22 claims per-commit roadmap placement, `cargo clippy --lib`, mutation kills, and earlier-head results, which cannot be verified from the exported read-only merge tree without `.git` history or cargo execution."
  },
  {
    "claim": "2.9",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:20 `cargo fmt --all -- --check: clean`; both `rustfmt --edition 2021 --check` on all 13 changed `.rs` files and `cargo fmt --all -- --check` on `tree/Cargo.toml` exit 0."
  },
  {
    "claim": "2.10",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/version_pin.rs:49,67,151,246 (`parse_semver(tag)`, `(Live::Version(l), Some(e)) if *l != e`, `parse_semver(text)`) confirms `version_pin` still compares only `N.N.N` (`Semver`)."
  },
  {
    "claim": "2.11",
    "verdict": "SURVIVED",
    "reason": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 exactly."
  },
  {
    "claim": "2.12",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:64-66 `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` leaves a third obsolete midnight guard despite `launch_with` calling `fixture` -> `apply_script_on(&r, DAY)` at src/resources/cloud_instance/tests_launch.rs:104, silently skipping the check when `t2[0] != t[0]`."
  },
  {
    "claim": "2.13",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102 `super::apply_script_on(resource, day_of(now.map_or(0, |d| d.as_secs())))`; scanning all remaining `apply_script(` callers in `src/` and `tests/` confirms none checks the computed token or UTC day (only `--client-token fj-` at src/resources/cloud_instance/tests_storage.rs:119)."
  },
  {
    "claim": "2.14",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` greedily consumes trailing punctuation; verified in bash that `tool 1.2.3-rc1.` sets `FJ_LIVEPRE=-rc1.` and `tool 1.2.3-rc.1. Built 2026` sets `FJ_LIVEPRE=-rc.1.`, whereas `tool 1.2.3.` sets `FJ_LIVE=1.2.3`, `FJ_LIVEPRE=`."
  },
  {
    "claim": "2.15",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` and line 113 `Some(s) if s.starts_with(|c: char| c.is_ascii_alphabetic())` drop numeric-leading SemVer 2.0.0 pre-release suffixes (`1.2.3-0.3.7`, `1.2.3-1` both yield empty `FJ_LIVEPRE` and `pre`, verified in bash)."
  },
  {
    "claim": "2.16",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:95 diverges on `[ \"$FJ_LIVEPRE\" != '{pre}' ]` (triggering apply for pin `v1.2.3-rc1` against live `1.2.3`), while src/resources/github_release.rs:179-188 `downgrade_guard` compares only `$FJ_MAJ/$FJ_MIN/$FJ_PAT` (`1.2.3` vs `1.2.3`) and ignores `FJ_LIVEPRE`, permitting silent downgrade from final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "2.17",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:23-28 permits uppercase ASCII letters in `is_safe_cargo_token` (`c.is_ascii_alphanumeric()`), while line 274 `gsub(/_/, \\\"-\\\", c)` / `gsub(/_/, \\\"-\\\", n)` omits `tolower`, leaving `Foo_Bar` vs `foo-bar` and `inflector` vs `Inflector` unresolved and hitting lines 277-281 `_FJ_NC=1` (verified in awk)."
  },
  {
    "claim": "3.1",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `BEGIN {{ gsub(/_/, \\\"-\\\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \\\"-\\\", n); if (n == c ...` maps `_` to `-` in both `c` and `n` before comparing (verified in awk)."
  },
  {
    "claim": "3.2",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:61-70 `FJ_PRESENT=0; FJ_LIVE=''; FJ_LIVEPRE=''; FJ_VERLINE=''` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` store `-<letter>[0-9A-Za-z.-]*` in `FJ_LIVEPRE` while keeping `FJ_LIVE` identical."
  },
  {
    "claim": "3.3",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/semver.rs:31-49 (`parse_semver_span(text).map(|(v, _)| v)`), src/tripwire/drift/version_pin.rs:49 (`pub use super::semver::{parse_semver, parse_semver_span, Semver};`), and src/resources/github_release_check.rs:92-96,106-118 confirm `parse_semver_span`, unchanged `parse_semver`, `pin_prerelease` ignoring `-` before `N.N.N`, and divergence on mismatch."
  },
  {
    "claim": "3.4",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 confirm `launch_io::token` removal, `apply_script_on`/`tail_on` taking UTC `day`, and single clock read in `apply_script`."
  },
  {
    "claim": "3.5",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:51-66 (`fn nothing_declared_is_byte_identical_to_today()`, line 59 `launch_io::token(`) inside `#[cfg(test)] mod tests_storage;` (src/resources/cloud_instance/mod.rs:64) cannot compile at the merge commit."
  },
  {
    "claim": "3.6",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-61 computes `s` via `apply_script_on(..., DAY)` (`DAY = 20_400`) while line 59 still calls removed wall-clock `launch_io::token(` instead of `tok`."
  },
  {
    "claim": "3.7",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:288-312 runs the adoption and rerun assertions unconditionally after removing both `if run_tokens(&log)[0] ... t1` guards."
  },
  {
    "claim": "3.8",
    "verdict": "SURVIVED",
    "reason": "docs/roadmaps/roadmap.yaml:8130-8219 contains `PMAT-1189`, `PMAT-1190`, and `PMAT-1198`, while `tree/.git` is absent so per-commit placement cannot be verified."
  },
  {
    "claim": "3.9",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:59-61 `launch_io::token(` breaks `--lib` test compilation (`E0425`) against src/resources/cloud_instance/launch_io.rs:97-102, falsifying `## Tests` (and `src/resources/cloud_instance/` has 133 tests, though `--lib cloud_instance` matches 139 including 6 in src/core/parser/tests_cloud_instance_validate.rs)."
  },
  {
    "claim": "3.10",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:20-22 lists separate-host clippy, fmt, mutation, and earlier-head results not re-executed in this review; `clippy --lib` without `--tests` skips `#[cfg(test)]` modules like `tests_storage.rs`."
  },
  {
    "claim": "3.11",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/version_pin.rs:63-70,146-154 compares only `N.N.N`, while modifying `live_version` (src/resources/github_release_check.rs:61-70,81-83) also changes the shell emitted via `live_version_unguarded` in src/resources/github_release.rs:79,189 (`apply_script`) and line 209 (`state_query_script`)."
  },
  {
    "claim": "3.12",
    "verdict": "SURVIVED",
    "reason": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 (`182 + 61 = 243`, `54` roadmap, `189` outside)."
  },
  {
    "claim": "3.13",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-62 still calls deleted `launch_io::token(` (removed in src/resources/cloud_instance/launch_io.rs:97-102) inside `#[cfg(test)] mod tests_storage;` (src/resources/cloud_instance/mod.rs:64), causing `E0425` and mismatching `DAY` (`20_400`); no other callers of `token`, `tail`, or `is_todays` remain."
  },
  {
    "claim": "3.14",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:92-96 diverges on live `1.2.3` vs pin `v1.2.3-rc1`, while src/resources/github_release.rs:158,179-188 (`/// forjar#613: refuse to install a pinned version OLDER than the live binary.` / `$FJ_MAJ -gt {maj}` ... `$FJ_PAT -gt {pat}`) ignores `FJ_LIVEPRE` and allows downgrading final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "3.15",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:60-66 keeps `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` even though src/resources/cloud_instance/tests_launch.rs:104 injects fixed `DAY` via `apply_script_on(&r, DAY)`, masking a non-deterministic `t2[0]`."
  },
  {
    "claim": "3.16",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` consumes trailing `.` on `tool 1.2.3-rc1.` into `FJ_LIVEPRE='-rc1.'` (verified in bash), falsely diverging against `tag: v1.2.3-rc1`, whereas `tool 1.2.3.` strips the period via `[.]([^0-9]|$)`."
  },
  {
    "claim": "3.17",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:4-5 (`a pre-release suffix (-rc1) on either side must equal the other side's;`), lines 68-69 (`(-[A-Za-z][0-9A-Za-z.-]*)?`), and line 113 (`s.starts_with(|c: char| c.is_ascii_alphabetic())`) drop valid SemVer 2.0.0 numeric-leading pre-releases like `v1.2.3-0.3.7` or `v1.2.3-1`, converging them against final `1.2.3`."
  },
  {
    "claim": "3.18",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_launch.rs:11-12 (`/// The UTC day every script under test is generated for.`) and src/resources/cloud_instance/tests_chain.rs:4 conflict with wall-clock `apply_script(` calls at tests_chain.rs:29, tests_storage.rs, profile.rs, userdata.rs, etc., and src/resources/cloud_instance/launch_io.rs:97-99 links publicly re-exported `pub fn apply_script` (src/lib.rs:41, src/resources/mod.rs:10, src/resources/cloud_instance/mod.rs:41) to `pub(crate) fn apply_script_on` (mod.rs:404)."
  }
]
VERDICT: FAIL
text_only: yes
judge: 1
reviewer_model: gemini-4-neon-high

=== JUDGE 2 ===
[
  {
    "claim": "1.1",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:59-61 `let quoted = sh_squote(&launch_io::token(&spec_of(&gated(std::path::Path::new(\"/t\"))).unwrap()));` still calls `launch_io::token`, which was deleted in src/resources/cloud_instance/launch_io.rs:97-102 inside unconditional `#[cfg(test)] mod tests_storage;` (src/resources/cloud_instance/mod.rs:63-64), breaking `--lib` test compilation with E0425; no other callers of `launch_io::token`, `launch_io::tail`, or `is_todays` remain."
  },
  {
    "claim": "1.2",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `awk -v c={q} 'BEGIN {{ gsub(/_/, \"-\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \"-\", n); if (n == c && $2 == \"=\" && $3 ~ /^\"[0-9A-Za-z._+-]+\"$/)` normalizes `_` to `-` in both declared `c` and search `$1` before comparing `n == c` (verified in awk)."
  },
  {
    "claim": "1.3",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:67-70 `(-[A-Za-z][0-9A-Za-z.-]*)?` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` capture `-<letter>...` into `FJ_LIVEPRE` while keeping `FJ_LIVE` (read by `downgrade_guard` at src/resources/github_release.rs:179-189 `if [ -n \"$FJ_LIVE\" ]; then`) unchanged."
  },
  {
    "claim": "1.4",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/semver.rs:31-50 `pub fn parse_semver(text: &str) -> Option<Semver> { parse_semver_span(text).map(|(v, _)| v) }`, src/tripwire/drift/version_pin.rs:49 `pub use super::semver::{parse_semver, parse_semver_span, Semver};`, and src/resources/github_release_check.rs:95,106-118 `let Some((_, end)) = parse_semver_span(tag)` and `if [ \"$FJ_NORM\" != '{want}' ] || [ \"$FJ_LIVEPRE\" != '{pre}' ]; then` confirm unchanged `parse_semver`, suffix extraction only after `N.N.N`, and divergence on mismatch."
  },
  {
    "claim": "1.5",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102 `pub fn apply_script(resource: &Resource) -> String`, line 181 `pub(super) fn tail_on(s: &CloudInstance, day: u64) -> String`, and src/resources/cloud_instance/mod.rs:41,404,489 confirm `token()` is removed and `apply_script` reads `SystemTime::now()` once and delegates to `apply_script_on`."
  },
  {
    "claim": "1.6",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:51-66 `let s = apply_script_on(&gated(std::path::Path::new(\"/t\")), DAY);` and line 59 `let quoted = sh_squote(&launch_io::token(` fail compilation (E0425), blocking both `TODAY_TAIL` and src/resources/cloud_instance/userdata.rs:186 `fn unset_script_is_byte_identical_to_today()`, and mixing fixed `DAY` with wall-clock `token` would fail lines 62-66."
  },
  {
    "claim": "1.7",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-61 generates `s` with `apply_script_on(..., DAY)` while line 59 still calls removed wall-clock `launch_io::token(...)` instead of `tok(...)`."
  },
  {
    "claim": "1.8",
    "verdict": "SURVIVED",
    "reason": "ANGLE.diff:225-249 and src/resources/cloud_instance/tests_chain.rs:290-312 `assert_eq!(rc, 0, \"{out}\");` and `assert_eq!(t[2 * round - 2..], [t1.clone(), t2.clone()], \"{log}\");` confirm both `if run_tokens(&log)[0] ... t1` midnight guards were removed."
  },
  {
    "claim": "1.9",
    "verdict": "SURVIVED",
    "reason": "docs/roadmaps/roadmap.yaml:8130-8147 (`- id: PMAT-1189`), 8166-8183 (`- id: PMAT-1190`), and 8202-8219 (`- id: PMAT-1198`) contain the 54 inserted lines, and `tree/.git` is absent so per-commit placement cannot be verified."
  },
  {
    "claim": "1.10",
    "verdict": "SURVIVED",
    "reason": "src/resources/tests_package_e.rs:134-149 (2 `fj1189` tests) and src/resources/tests_github_release_verified.rs:36-106 (8 tests including `dash_before_the_version_is_not_a_prerelease`) exist as described, while src/resources/cloud_instance/tests_storage.rs:59 `launch_io::token(` prevents `--lib` test compilation."
  },
  {
    "claim": "1.11",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:59 `launch_io::token(` prevents `--lib` test compilation, and static `#[test]` counts for filter `cloud_instance` total 139 (133 in `src/resources/cloud_instance/` plus 6 in src/core/parser/tests_cloud_instance_validate.rs:30-164 via src/core/parser/mod.rs:30-31), while `github_release` matches 41 and `tripwire::drift` matches 133."
  },
  {
    "claim": "1.12",
    "verdict": "SURVIVED",
    "reason": "Cargo.toml:24 `edition = \"2021\"` and `rustfmt --edition 2021 --check` exited 0 on all 13 changed `.rs` files; `cargo clippy --lib` without `--tests` does not compile `#[cfg(test)] mod tests_storage;` (src/resources/cloud_instance/mod.rs:63-64) and was not executed."
  },
  {
    "claim": "1.13",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 (`gsub(/_/, \"-\", n)`) is exercised by 1 of 2 `fj1189` tests (src/resources/tests_package_e.rs:143-149) and src/resources/github_release_check.rs:107 (`parse_semver_span(tag)`) by 1 of 8 tests (src/resources/tests_github_release_verified.rs:100-106), while mutant execution at the merge commit is blocked by src/resources/cloud_instance/tests_storage.rs:59."
  },
  {
    "claim": "1.14",
    "verdict": "SURVIVED",
    "reason": "src/resources/disk_budget/tests/execution.rs:189 `fn a_cargo_target_whose_binary_is_running_survives()` exists untouched by ANGLE.diff, `src/resources/` contains 961 `#[test]` functions, and historical earlier-head runs cannot be directly verified from the merge tree."
  },
  {
    "claim": "1.15",
    "verdict": "REFUTED",
    "reason": "src/resources/github_release_check.rs:61-70,81-83 modifies `live_version` (wrapped by `live_version_unguarded`), which is also embedded into `apply_script` at src/resources/github_release.rs:79,150,152,189 (`let post = super::github_release_check::live_version_unguarded(&bin_path, false);` and `downgrade_guard`) and `state_query_script` at src/resources/github_release.rs:209,212, and src/tripwire/drift/semver.rs:31-50 is also modified; therefore the claim that this PR changes only the check script in `github_release_check.rs` plus the `parse_semver_span` re-export is false."
  },
  {
    "claim": "1.16",
    "verdict": "SURVIVED",
    "reason": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 (`182 + 61 = 243` total, `54` roadmap, `189` outside) and ANGLE.diff."
  },
  {
    "claim": "2.1",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-62 calls `apply_script_on(..., DAY)` (`DAY = 20_400` at src/resources/cloud_instance/tests_launch.rs:12) and deleted `launch_io::token(` (removed at src/resources/cloud_instance/launch_io.rs:97-102), causing E0425 and a token-day mismatch on line 62; no other callers of removed `token` or renamed `tail` remain in `src/`."
  },
  {
    "claim": "2.2",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:11,19 claims the `TODAY_TAIL` test and `--lib` test filters pass, contradicted by src/resources/cloud_instance/tests_storage.rs:51-66 still calling removed `launch_io::token(` at line 59 and breaking `--lib` compilation."
  },
  {
    "claim": "2.3",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `BEGIN {{ gsub(/_/, \"-\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \"-\", n); if (n == c && $2 == \"=\" && $3 ~ /^\"[0-9A-Za-z._+-]+\"$/)` maps `_` to `-` in both declared and search crate names (verified in awk)."
  },
  {
    "claim": "2.4",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:61-70,92-96,106-118, src/tripwire/drift/semver.rs:31-50, and src/tripwire/drift/version_pin.rs:49 capture `FJ_LIVEPRE` without changing `FJ_LIVE`, read pin suffixes after `parse_semver_span` without changing `parse_semver`, and diverge on mismatch."
  },
  {
    "claim": "2.5",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 remove `token()`, pass UTC `day` to `apply_script_on`/`tail_on`, and read `SystemTime::now()` once in `apply_script`."
  },
  {
    "claim": "2.6",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:290-294,307-311 execute the adoption and rerun assertions unconditionally after removing both `if run_tokens(&log)[0] ... t1` guards (ANGLE.diff:225-249)."
  },
  {
    "claim": "2.7",
    "verdict": "SURVIVED",
    "reason": "docs/roadmaps/roadmap.yaml:8130 (`- id: PMAT-1189`), 8166 (`- id: PMAT-1190`), and 8202 (`- id: PMAT-1198`) are each present once without duplicates."
  },
  {
    "claim": "2.8",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:13,20-22 claims per-commit roadmap placement, `cargo clippy --lib`, mutation kills, and earlier-head results, which cannot be verified from the exported read-only tree lacking `.git` history and build execution."
  },
  {
    "claim": "2.9",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:20 `cargo fmt --all -- --check`: clean; running `rustfmt --edition 2021 --check` on all 13 changed `.rs` files exited 0 with no diffs."
  },
  {
    "claim": "2.10",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/version_pin.rs:49,67,151,246 (`parse_semver(tag)`, `(Live::Version(l), Some(e)) if *l != e`) confirms the `version_pin` drift detector still compares only `N.N.N` (`Semver`)."
  },
  {
    "claim": "2.11",
    "verdict": "SURVIVED",
    "reason": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 (`SIZE`) exactly."
  },
  {
    "claim": "2.12",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:64-66 `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` leaves a third obsolete midnight guard despite `launch_with` calling `fixture` -> `apply_script_on(&r, DAY)` at src/resources/cloud_instance/tests_launch.rs:104, silently skipping the check whenever `t2[0] != t[0]`."
  },
  {
    "claim": "2.13",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102 `pub fn apply_script(resource: &Resource) -> String` has no test asserting the UTC day or full token it emits; remaining callers only check `--client-token fj-` (src/resources/cloud_instance/tests_storage.rs:119) or ignore the token."
  },
  {
    "claim": "2.14",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` greedily consumes trailing `.` into `FJ_LIVEPRE` (verified in bash: `tool 1.2.3-rc1.` -> `FJ_LIVEPRE=-rc1.`, `tool 1.2.3-rc.1. Built 2026` -> `FJ_LIVEPRE=-rc.1.`, whereas `tool 1.2.3.` -> `FJ_LIVE=1.2.3`, `FJ_LIVEPRE=`), causing perpetual divergence against `v1.2.3-rc1`."
  },
  {
    "claim": "2.15",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` and line 113 `Some(s) if s.starts_with(|c: char| c.is_ascii_alphabetic())` require an ASCII letter after `-`, so digit-leading SemVer 2.0.0 pre-releases (`v1.2.3-0.3.7`, `v1.2.3-1`, `v1.2.3-20261118`) yield empty `pre` and `FJ_LIVEPRE` (verified in bash)."
  },
  {
    "claim": "2.16",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:95 diverges on `\"$FJ_LIVEPRE\" != '{pre}'` for pin `v1.2.3-rc1` vs live `1.2.3`, triggering apply, while src/resources/github_release.rs:179-188 `downgrade_guard` compares only `$FJ_MAJ/$FJ_MIN/$FJ_PAT` (`1.2.3 > 1.2.3` is false) and allows silently downgrading final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "2.17",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:23-28 allows ASCII uppercase in `is_safe_cargo_token`, while line 274 `BEGIN {{ gsub(/_/, \"-\", c) }} ... n = $1; gsub(/_/, \"-\", n); if (n == c` omits `tolower`, leaving case-differing names (`Foo_Bar` vs `foo-bar`) unresolved and bypassing the cargo cache (verified in awk)."
  },
  {
    "claim": "3.1",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `BEGIN {{ gsub(/_/, \"-\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \"-\", n); if (n == c` maps `_` to `-` in both `c` and `n` before comparing (verified in awk)."
  },
  {
    "claim": "3.2",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:61-70 `FJ_PRESENT=0; FJ_LIVE=''; FJ_LIVEPRE=''; FJ_VERLINE=''` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` store `-<letter>[0-9A-Za-z.-]*` in `FJ_LIVEPRE` while leaving `FJ_LIVE` unchanged."
  },
  {
    "claim": "3.3",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/semver.rs:31-49 (`parse_semver_span(text).map(|(v, _)| v)`), src/tripwire/drift/version_pin.rs:49 (`pub use super::semver::{parse_semver, parse_semver_span, Semver};`), and src/resources/github_release_check.rs:92-96,106-118 confirm `parse_semver_span`, unchanged `parse_semver`, `pin_prerelease` ignoring `-` before `N.N.N`, and divergence on mismatch."
  },
  {
    "claim": "3.4",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 confirm `launch_io::token` removal, UTC-day injection in `apply_script_on`/`tail_on`, and single `SystemTime::now()` read in `apply_script`."
  },
  {
    "claim": "3.5",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:51-66 `fn nothing_declared_is_byte_identical_to_today()` calls deleted `launch_io::token(` at line 59 inside `#[cfg(test)] mod tests_storage;` (src/resources/cloud_instance/mod.rs:64), preventing compilation at the merge commit."
  },
  {
    "claim": "3.6",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52 uses `apply_script_on(&gated(std::path::Path::new(\"/t\")), DAY)` while lines 59-61 still call removed wall-clock `launch_io::token(...)` instead of `tok`."
  },
  {
    "claim": "3.7",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:288-312 removes both `if run_tokens(&log)[0] ... t1` guards in `adoption_uses_the_current_chain_token_and_a_rerun_walks_the_same_chain` so all assertions execute unconditionally."
  },
  {
    "claim": "3.8",
    "verdict": "SURVIVED",
    "reason": "docs/roadmaps/roadmap.yaml:8130-8219 contains `PMAT-1189`, `PMAT-1190`, and `PMAT-1198`, while `tree/.git` is absent so per-commit placement cannot be verified."
  },
  {
    "claim": "3.9",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:59-61 `launch_io::token(` fails `--lib` test compilation (E0425) against src/resources/cloud_instance/launch_io.rs:97-102, falsifying `## Tests` (note: `src/resources/cloud_instance/` has 133 `#[test]` functions, while `--lib cloud_instance` matches 139 including 6 in src/core/parser/tests_cloud_instance_validate.rs)."
  },
  {
    "claim": "3.10",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:20-22 separate-host `clippy --lib` (which skips `#[cfg(test)]` modules like `tests_storage.rs`), `cargo fmt`, mutation kills, and earlier-head results were not re-executed in this read-only tree."
  },
  {
    "claim": "3.11",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/version_pin.rs:63-70,146-154 compares only `N.N.N`, while modifying `live_version` (src/resources/github_release_check.rs:61-70,81-83) also changes the shell emitted via `live_version_unguarded` in src/resources/github_release.rs:79,189 (`apply_script`) and line 209 (`state_query_script`), falsifying \"this PR changes the check script only\"."
  },
  {
    "claim": "3.12",
    "verdict": "SURVIVED",
    "reason": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 (`182 + 61 = 243`, `54` roadmap, `189` non-roadmap)."
  },
  {
    "claim": "3.13",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-62 still calls deleted `launch_io::token(` (removed in src/resources/cloud_instance/launch_io.rs:97-102) inside `#[cfg(test)] mod tests_storage;` (src/resources/cloud_instance/mod.rs:64), breaking compilation (E0425) and mismatching fixed `DAY` (`20_400`) on line 52; no other caller of removed `token`, `tail`, or `is_todays` remains."
  },
  {
    "claim": "3.14",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:92-96 diverges on pin `v1.2.3-rc1` vs live `1.2.3`, while src/resources/github_release.rs:158,179-188 `downgrade_guard` ignores `FJ_LIVEPRE` and sees `1.2.3 > 1.2.3` as false, permitting silent downgrade of final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "3.15",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:64-66 keeps `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` despite fixed `DAY` in src/resources/cloud_instance/tests_launch.rs:104 (`let script = apply_script_on(&r, DAY);`), silently skipping `assert_eq!(t2, t)` if `t2[0]` changes."
  },
  {
    "claim": "3.16",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` consumes trailing `.` on `tool 1.2.3-rc1.` into `FJ_LIVEPRE='-rc1.'` (verified in bash), causing false divergence against `tag: v1.2.3-rc1`, whereas `tool 1.2.3.` strips the trailing `.` via `[.]([^0-9]|$)`."
  },
  {
    "claim": "3.17",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:4-5 (`a pre-release suffix (-rc1) on either side must equal the other side's`), lines 68-69 (`(-[A-Za-z][0-9A-Za-z.-]*)?`), and line 113 (`s.starts_with(|c: char| c.is_ascii_alphabetic())`) drop numeric-leading SemVer 2.0.0 pre-releases (`v1.2.3-0.3.7`, `v1.2.3-1`), falsely converging them against final `1.2.3`."
  },
  {
    "claim": "3.18",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_launch.rs:11 (`/// The UTC day every script under test is generated for.`) and src/resources/cloud_instance/tests_chain.rs:4 (`the token's UTC day is injected (DAY).`) conflict with wall-clock `apply_script(...)` at src/resources/cloud_instance/tests_chain.rs:29 and across sibling test modules (`tests_storage.rs`, `profile.rs`, `userdata.rs`, etc.), and src/resources/cloud_instance/launch_io.rs:97-99 links publicly re-exported `pub fn apply_script` (src/lib.rs:41, src/resources/mod.rs:10, src/resources/cloud_instance/mod.rs:41) to `pub(crate) fn apply_script_on` (src/resources/cloud_instance/mod.rs:404)."
  }
]
VERDICT: FAIL
text_only: yes
judge: 2
reviewer_model: gemini-4-neon-high

=== JUDGE 3 ===
[
  {
    "claim": "1.1",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:59-61 `let quoted = sh_squote(&launch_io::token(&spec_of(&gated(std::path::Path::new(\"/t\"))).unwrap()));` still calls `launch_io::token`, which was removed in src/resources/cloud_instance/launch_io.rs:97-102 (`pub fn apply_script`), inside unconditional `#[cfg(test)] mod tests_storage;` at src/resources/cloud_instance/mod.rs:63-64; no other callers of `token`, `tail`, or `is_todays` remain."
  },
  {
    "claim": "1.2",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `awk -v c={q} 'BEGIN {{ gsub(/_/, \"-\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \"-\", n); if (n == c && $2 == \"=\" && $3 ~ /^\"[0-9A-Za-z._+-]+\"$/)` normalizes `_` to `-` on both declared `c` and registry `n` (verified in awk for both directions)."
  },
  {
    "claim": "1.3",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:67-70 `(-[A-Za-z][0-9A-Za-z.-]*)?([^.0-9]|[.]([^0-9]|$)|$)` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` store `-<letter>...` in `FJ_LIVEPRE` without changing `FJ_LIVE` used by `downgrade_guard` at src/resources/github_release.rs:179-189."
  },
  {
    "claim": "1.4",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/semver.rs:31-50 `pub fn parse_semver(text: &str) -> Option<Semver> { parse_semver_span(text).map(|(v, _)| v) }`, src/tripwire/drift/version_pin.rs:49 `pub use super::semver::{parse_semver, parse_semver_span, Semver};`, and src/resources/github_release_check.rs:95,107-118 (`parse_semver_span(tag)`, `if [ \"$FJ_NORM\" != '{want}' ] || [ \"$FJ_LIVEPRE\" != '{pre}' ]; then`) confirm all points."
  },
  {
    "claim": "1.5",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102 `pub fn apply_script(resource: &Resource) -> String` reads `SystemTime::now()` once and delegates to `super::apply_script_on(resource, day_of(...))`, line 181 defines `pub(super) fn tail_on(s: &CloudInstance, day: u64) -> String`, and src/resources/cloud_instance/mod.rs:41,404,489 re-exports `apply_script` and wires `apply_script_on`."
  },
  {
    "claim": "1.6",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:51-66 `fn nothing_declared_is_byte_identical_to_today()` calls `apply_script_on(..., DAY)` at line 52 and removed `launch_io::token(` at line 59, causing `E0425` (blocking both it and src/resources/cloud_instance/userdata.rs:186 `fn unset_script_is_byte_identical_to_today()`) and mixing fixed `DAY` (`20_400`) with wall-clock `token`."
  },
  {
    "claim": "1.7",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-61 `let s = apply_script_on(&gated(std::path::Path::new(\"/t\")), DAY);` and `let quoted = sh_squote(&launch_io::token(` confirm `nothing_declared_is_byte_identical_to_today` was not updated to `tok(...)`."
  },
  {
    "claim": "1.8",
    "verdict": "SURVIVED",
    "reason": "ANGLE.diff:225-247 and src/resources/cloud_instance/tests_chain.rs:290-312 `assert_eq!(rc, 0, \"{out}\");` and `assert_eq!(t[2 * round - 2..], [t1.clone(), t2.clone()], \"{log}\");` confirm both `if run_tokens(&log)[0] ... t1` guards were removed and the assertions execute unconditionally."
  },
  {
    "claim": "1.9",
    "verdict": "SURVIVED",
    "reason": "docs/roadmaps/roadmap.yaml:8130-8147 (`- id: PMAT-1189`), 8166-8183 (`- id: PMAT-1190`), and 8202-8219 (`- id: PMAT-1198`) contain the 54 inserted lines, and `tree/.git` is absent so per-commit placement cannot be verified."
  },
  {
    "claim": "1.10",
    "verdict": "SURVIVED",
    "reason": "src/resources/tests_package_e.rs:134-149 (2 `fj1189` tests) and src/resources/tests_github_release_verified.rs:36-106 (8 tests including `dash_before_the_version_is_not_a_prerelease`) exist, while src/resources/cloud_instance/tests_storage.rs:59 `launch_io::token(` breaks `--lib` test compilation."
  },
  {
    "claim": "1.11",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:59 breaks `--lib` test compilation, and static `#[test]` counts match 139 for filter `cloud_instance` (133 in `src/resources/cloud_instance/` plus 6 in src/core/parser/tests_cloud_instance_validate.rs:30-164 via src/core/parser/mod.rs:30-31), 41 for `github_release`, and 133 for `tripwire::drift`."
  },
  {
    "claim": "1.12",
    "verdict": "SURVIVED",
    "reason": "Cargo.toml:24 `edition = \"2021\"`; `rustfmt --edition 2021 --check` exited 0 on all 13 changed `.rs` files, while `cargo clippy -j 4 --locked --lib -- -D warnings` is unverified and omits `#[cfg(test)] mod tests_storage;` at src/resources/cloud_instance/mod.rs:63-64."
  },
  {
    "claim": "1.13",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `gsub(/_/, \"-\", n)` is covered by 1 of 2 tests at src/resources/tests_package_e.rs:143-149, and src/resources/github_release_check.rs:107 `parse_semver_span(tag)` is covered by 1 of 8 tests at src/resources/tests_github_release_verified.rs:100-106, while mutant execution is blocked by src/resources/cloud_instance/tests_storage.rs:59."
  },
  {
    "claim": "1.14",
    "verdict": "SURVIVED",
    "reason": "src/resources/disk_budget/tests/execution.rs:189 `fn a_cargo_target_whose_binary_is_running_survives()` exists untouched by ANGLE.diff, `src/resources/` contains 961 `#[test]` functions, and earlier-head historical runs cannot be directly verified from the merge snapshot."
  },
  {
    "claim": "1.15",
    "verdict": "REFUTED",
    "reason": "src/resources/github_release_check.rs:61-70,81-83 modifies `live_version` (wrapped by `live_version_unguarded`), which is also embedded by src/resources/github_release.rs:79,150,152,189 (`apply_script` and `downgrade_guard`) and src/resources/github_release.rs:209,212 (`state_query_script`), and src/tripwire/drift/semver.rs:31-50 is also modified; therefore the claim that the PR body assertion is TRUE and that this PR changes only the check script in `github_release_check.rs` plus the `parse_semver_span` re-export is false."
  },
  {
    "claim": "1.16",
    "verdict": "SURVIVED",
    "reason": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match ANGLE.diff and PR_BODY.md:29 (`182 + 61 = 243`, `54` roadmap, `189` outside)."
  },
  {
    "claim": "2.1",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-62 calls `apply_script_on(..., DAY)` (`DAY = 20_400` at tests_launch.rs:12) alongside removed `launch_io::token(` (deleted in src/resources/cloud_instance/launch_io.rs:97-102), causing `E0425` and a day mismatch on line 62; no other callers of `token` or `tail` remain in `src/`."
  },
  {
    "claim": "2.2",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:11,19 claims the `TODAY_TAIL` test and `--lib` suites pass, contradicted by src/resources/cloud_instance/tests_storage.rs:51-66 calling removed `launch_io::token(` at line 59 and breaking `--lib` test compilation."
  },
  {
    "claim": "2.3",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `BEGIN {{ gsub(/_/, \"-\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \"-\", n); if (n == c` maps `_` to `-` in both declared and registry names (verified with awk)."
  },
  {
    "claim": "2.4",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:61-70,92-96,106-118, src/tripwire/drift/semver.rs:31-50, and src/tripwire/drift/version_pin.rs:49 capture `FJ_LIVEPRE` without changing `FJ_LIVE`, keep `parse_semver` unchanged, ignore `-` before `N.N.N`, and diverge on mismatch."
  },
  {
    "claim": "2.5",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 remove `token()`, pass `day: u64` to `apply_script_on`/`tail_on`, and read `SystemTime::now()` once in `apply_script`."
  },
  {
    "claim": "2.6",
    "verdict": "SURVIVED",
    "reason": "ANGLE.diff:225-247 and src/resources/cloud_instance/tests_chain.rs:290-294,307-311 confirm both `if run_tokens(&log)[0] ... t1` guards were removed and those assertions execute unconditionally."
  },
  {
    "claim": "2.7",
    "verdict": "SURVIVED",
    "reason": "docs/roadmaps/roadmap.yaml:8130 (`- id: PMAT-1189`), 8166 (`- id: PMAT-1190`), and 8202 (`- id: PMAT-1198`) each appear once with no duplicates."
  },
  {
    "claim": "2.8",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:13,20-22 claims per-commit roadmap placement, `cargo clippy --lib` (which skips `#[cfg(test)]`), mutation kills, and earlier-head runs, which cannot be verified from the exported read-only tree without `.git` history or cargo execution."
  },
  {
    "claim": "2.9",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:20 `cargo fmt --all -- --check: clean`; `rustfmt --edition 2021 --check` exited 0 on all 13 changed `.rs` files in `tree/`."
  },
  {
    "claim": "2.10",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/version_pin.rs:49,67,151,246 (`parse_semver(tag)`, `(Live::Version(l), Some(e)) if *l != e`) confirms `version_pin` still compares only `N.N.N` (`Semver`)."
  },
  {
    "claim": "2.11",
    "verdict": "SURVIVED",
    "reason": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 exactly."
  },
  {
    "claim": "2.12",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:57,63-66 `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` keeps an obsolete midnight guard despite `launch_with` calling `fixture` -> `apply_script_on(&r, DAY)` at src/resources/cloud_instance/tests_launch.rs:104, silently skipping the check if `t2[0] != t[0]`."
  },
  {
    "claim": "2.13",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102 `super::apply_script_on(resource, day_of(now.map_or(0, |d| d.as_secs())))` has no test asserting the UTC day or token value it computes; remaining `apply_script` callers only check `--client-token fj-` at src/resources/cloud_instance/tests_storage.rs:119 or ignore the token."
  },
  {
    "claim": "2.14",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` greedily consumes trailing `.` (verified in bash: `tool 1.2.3-rc1.` -> `FJ_LIVEPRE=-rc1.`, `tool 1.2.3-rc.1. Built 2026` -> `FJ_LIVEPRE=-rc.1.`, while `tool 1.2.3.` strips the trailing `.`), causing divergence against `v1.2.3-rc1`."
  },
  {
    "claim": "2.15",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` and line 113 `Some(s) if s.starts_with(|c: char| c.is_ascii_alphabetic())` drop numeric-leading SemVer 2.0.0 pre-releases (`v1.2.3-0.3.7`, `v1.2.3-1`, `v1.2.3-20261118` all yield empty `pre` and `FJ_LIVEPRE`)."
  },
  {
    "claim": "2.16",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:95 diverges on `\"$FJ_LIVEPRE\" != '{pre}'` for pin `v1.2.3-rc1` vs live `1.2.3`, while src/resources/github_release.rs:179-188 `downgrade_guard` compares only `$FJ_MAJ`/`$FJ_MIN`/`$FJ_PAT` (`1.2.3 > 1.2.3` is false) and permits downgrading final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "2.17",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:23-28 permits uppercase letters in `is_safe_cargo_token`, and line 274 `gsub(/_/, \"-\", c)` / `gsub(/_/, \"-\", n)` omits `tolower`, leaving `Foo_Bar` vs `foo-bar` and `inflector` vs `Inflector` unresolved (verified in awk)."
  },
  {
    "claim": "3.1",
    "verdict": "SURVIVED",
    "reason": "src/resources/package/cargo.rs:274 `BEGIN {{ gsub(/_/, \"-\", c) }} /^[^[:space:]]/ {{ n = $1; gsub(/_/, \"-\", n); if (n == c` maps `_` to `-` in both `c` and `n` before comparing (verified in awk)."
  },
  {
    "claim": "3.2",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:61-70 `FJ_PRESENT=0; FJ_LIVE=''; FJ_LIVEPRE=''; FJ_VERLINE=''` and `FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"` capture `-<letter>[0-9A-Za-z.-]*` in `FJ_LIVEPRE` while preserving `FJ_LIVE`."
  },
  {
    "claim": "3.3",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/semver.rs:31-49, src/tripwire/drift/version_pin.rs:49, and src/resources/github_release_check.rs:92-96,106-118 confirm `parse_semver_span`, unchanged `parse_semver`, `pin_prerelease` ignoring `-` before `N.N.N`, and divergence on `FJ_LIVEPRE != pre`."
  },
  {
    "claim": "3.4",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/launch_io.rs:99-102,181,198 and src/resources/cloud_instance/mod.rs:41,404,489 confirm `launch_io::token` removal, `apply_script_on`/`tail_on` taking UTC `day`, and `apply_script` reading `SystemTime::now()` once."
  },
  {
    "claim": "3.5",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:51-66 (`fn nothing_declared_is_byte_identical_to_today()`, line 59 `launch_io::token(`) and src/resources/cloud_instance/mod.rs:64 (`#[cfg(test)] mod tests_storage;`) prevent the `TODAY_TAIL` test from compiling at the merge commit."
  },
  {
    "claim": "3.6",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-61 generates `s` for fixed `DAY` (`20_400`) while line 59 still calls removed `launch_io::token(` instead of `tok`."
  },
  {
    "claim": "3.7",
    "verdict": "SURVIVED",
    "reason": "ANGLE.diff:225-247 and src/resources/cloud_instance/tests_chain.rs:288-312 confirm both `if run_tokens(&log)[0] ... t1` guards were removed and those assertions execute unconditionally."
  },
  {
    "claim": "3.8",
    "verdict": "SURVIVED",
    "reason": "docs/roadmaps/roadmap.yaml:8130-8219 contains `PMAT-1189`, `PMAT-1190`, and `PMAT-1198`, and `tree/.git` is absent so per-commit placement cannot be verified."
  },
  {
    "claim": "3.9",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:59-61 `launch_io::token(` vs src/resources/cloud_instance/launch_io.rs:97-102 confirms `## Tests` cannot compile or pass (`E0425`) at the merge commit (`src/resources/cloud_instance/` has 133 `#[test]`s, while substring `--lib cloud_instance` also includes 6 in src/core/parser/tests_cloud_instance_validate.rs)."
  },
  {
    "claim": "3.10",
    "verdict": "SURVIVED",
    "reason": "PR_BODY.md:20-22 separate-host `clippy --lib` (which skips `#[cfg(test)]` modules like `tests_storage.rs`), `cargo fmt`, mutation kills, and earlier-head runs cannot be verified from the exported read-only tree."
  },
  {
    "claim": "3.11",
    "verdict": "SURVIVED",
    "reason": "src/tripwire/drift/version_pin.rs:63-70,146-154 compares only `N.N.N`, while modifying `live_version` at src/resources/github_release_check.rs:61-70,81-83 also changes the shell emitted via `live_version_unguarded` in src/resources/github_release.rs:79,189 (`apply_script`) and line 209 (`state_query_script`)."
  },
  {
    "claim": "3.12",
    "verdict": "SURVIVED",
    "reason": "PR.stat:1,15 `docs/roadmaps/roadmap.yaml | 54 +++...` and `14 files changed, 182 insertions(+), 61 deletions(-)` match PR_BODY.md:29 (`182 + 61 = 243`, `54` roadmap, `189` non-roadmap)."
  },
  {
    "claim": "3.13",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_storage.rs:52,59-62 still calls deleted `launch_io::token(` (removed in src/resources/cloud_instance/launch_io.rs:97-102) inside `#[cfg(test)] mod tests_storage;` (mod.rs:64), causing `E0425` and a `DAY` vs today mismatch; no other caller of `token`, `tail`, or `is_todays` remains."
  },
  {
    "claim": "3.14",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:92-96 diverges for pin `v1.2.3-rc1` vs live `1.2.3`, while src/resources/github_release.rs:158,179-188 `downgrade_guard` ignores `FJ_LIVEPRE` and sees `1.2.3 > 1.2.3` as false, permitting a downgrade from final `1.2.3` to `v1.2.3-rc1`."
  },
  {
    "claim": "3.15",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_chain.rs:60-66 keeps `if t2[0] == t[0] { assert_eq!(t2, t, \"{dead}: same tokens\"); }` even though src/resources/cloud_instance/tests_launch.rs:104 `let script = apply_script_on(&r, DAY);` fixes `DAY`, silently skipping the determinism check if `t2[0]` changes."
  },
  {
    "claim": "3.16",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:68-69 `(-[A-Za-z][0-9A-Za-z.-]*)?` consumes trailing `.` on `tool 1.2.3-rc1.` into `FJ_LIVEPRE='-rc1.'` (verified in bash), diverging against `v1.2.3-rc1`, whereas `tool 1.2.3.` strips the trailing `.` and converges against `v1.2.3`."
  },
  {
    "claim": "3.17",
    "verdict": "SURVIVED",
    "reason": "src/resources/github_release_check.rs:4-5 (`a pre-release suffix (-rc1) on either side must equal the other side's`), lines 68-69 (`(-[A-Za-z][0-9A-Za-z.-]*)?`), and line 113 (`s.starts_with(|c: char| c.is_ascii_alphabetic())`) drop numeric-leading SemVer 2.0.0 pre-releases (`v1.2.3-0.3.7`, `v1.2.3-1`), converging them against final `1.2.3`."
  },
  {
    "claim": "3.18",
    "verdict": "SURVIVED",
    "reason": "src/resources/cloud_instance/tests_launch.rs:11 `/// The UTC day every script under test is generated for.` and src/resources/cloud_instance/tests_chain.rs:4 conflict with wall-clock `apply_script(...)` calls at tests_chain.rs:29, tests_storage.rs, profile.rs, and userdata.rs; and src/resources/cloud_instance/launch_io.rs:97-99 links publicly re-exported `pub fn apply_script` (src/lib.rs:41, src/resources/mod.rs:10, src/resources/cloud_instance/mod.rs:41) to `pub(crate) fn apply_script_on` (mod.rs:404)."
  }
]
VERDICT: FAIL
text_only: yes
judge: 3
reviewer_model: gemini-4-neon-high
