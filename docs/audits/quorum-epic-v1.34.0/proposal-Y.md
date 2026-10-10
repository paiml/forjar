PROPOSAL X — paiml/forjar next tagged release
Facts: last tag v1.33.0 (cut 2026-09-28, Cargo.toml version 1.33.0). 8 PRs merged since (#636 I8 bashrs 7.4 SC2105, #677 #417 ratchet, #676 aarch64 cross PATH, #682 store seal #410, #690/#693 cross RUSTUP_HOME #683, #681 apply -r secret test #680, #675 plan --output-dir selectors #674). releases.yaml has no v1.33.0 row and `next:` still says v1.33.0 (gate T stale). bashrs on crates.io is 7.4.1; 7.4.2 (carries bashrs#441: SC2075 false positive that makes forjar's I8 gate refuse infra's fw16-wired-10g-nm-owner completion_check) is not yet published. Cargo.toml says bashrs = "7.4", lock 7.4.1.
next_tag: v1.33.0 — move the existing v1.33.0 tag forward to current main so the bashrs fix ships under the version already on crates.io consumers pin; following_tag: v1.34.0.
CORE (epic "forjar v1.33.0 (re-tag)", milestone 1.33.0):
 C1 (no ticket needed, fold into the re-tag): `cargo update -p bashrs --precise 7.4.2` once 7.4.2 is on crates.io, plus a committed fixture copy of the fw16-wired-10g-nm-owner completion_check and a test that the I8 gate lints it with 0 errors. Blocked on the bashrs publish. Label must-carry.
 C2 already merged in the window, label release:v1.34.0: PMAT-633, PMAT-671, PMAT-683, PMAT-680, PMAT-674.
 C3 books: PMAT-652 (release 1.33.0) + PMAT-604 (book v1.32.0/v1.33.0 rows): releases.yaml gets the v1.33.0 row; PMAT-642/648/651 (shipped in v1.33.0) close with it.
 Release gate: cut on main, tag there, merge back; clean-room green on exactly the tagged commit before any upload; gate T (scripts/dogfood/tagged.sh) green; dogfood-1.34.0 receipt + crux-1.34.0; cookbook commit recorded; crates.io publish automatic after.
LOOK-AHEAD (epic "forjar v1.35.0", kit docs/lookahead/v1.35.0.yaml; no PRs during the v1.34.0 cut):
 PMAT-594 (gate B ratchet over ceiling), PMAT-614 (Windows nightly OpenSSL), PMAT-131 (overlay_interface capstone); candidate must_carry from open issues, untriaged, not minted: secret/I8 cluster #679 #686 #687, #694 completion_check on package never run, #663 service state enabled.
Standing theme epics stay: #670, #668, #655, #620.
