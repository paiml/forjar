# Quorum receipt window: 1.34.2 (forjar#1158, part of #876)

Window: PRs on main first-parent between v1.34.1 and v1.34.2 (88 PRs, from the (#N) subjects of `git log --first-parent v1.34.1..v1.34.2`).
Gate E reads `.quorum/<head-branch with / -> ->.json` at the tag; floor is lanes >= 3, judges >= 3, refuters_per_claim >= 3, claims_refuted >= 1, evidence.files >= 1, no waiver/override key (scripts/dogfood/lib/receipt.sh).
Status key: committed = `.quorum/<slug>.json` at v1.34.2 that passes every floor; MISSING = no committed receipt, and no receipt in the store (`$QUORUM_STORE/quorum/forjar/<pr>/<headRefOid>.json`) or in `.quorum/evidence/PR-<n>-*`.

| PR | branch | receipt | lanes |
|---|---|---|---|
| #935 | `security/no-pull-request-target-p0` | **MISSING** | - |
| #951 | `seat-s3/492` | committed `.quorum/seat-s3-492.json` | 3 |
| #1025 | `feat/cloud-instance-spot-b1` | committed `.quorum/feat-cloud-instance-spot-b1.json` | 3 |
| #1026 | `ci/burst-x64-optin` | **MISSING** | - |
| #1036 | `fix/cloud-instance-lookup-hardening` | committed `.quorum/fix-cloud-instance-lookup-hardening.json` | 3 |
| #1054 | `fix/file-health-main-red` | **MISSING** | - |
| #1058 | `fix/1048-cargo-latest-cache-key` | **MISSING** | - |
| #1061 | `train/913-ci-release-hygiene` | **MISSING** | - |
| #1062 | `fix/workflow-shape-tests-match-rulings` | **MISSING** | - |
| #1063 | `fix/cloud-instance-1025-followups` | committed `.quorum/fix-cloud-instance-1025-followups.json` | 3 |
| #1066 | `fix/1049-check-verified-version` | **MISSING** | - |
| #1068 | `fix/1065-check-checksum` | committed `.quorum/fix-1065-check-checksum.json` | 3 |
| #1071 | `fix/894-pool-scale-decision` | committed `.quorum/fix-894-pool-scale-decision.json` | 3 |
| #1072 | `fix/858-e02-controlmaster` | committed `.quorum/fix-858-e02-controlmaster.json` | 3 |
| #1073 | `fix/1067-apply-live-version` | committed `.quorum/fix-1067-apply-live-version.json` | 3 |
| #1084 | `fix/858-untyped-ratchet-5` | committed `.quorum/fix-858-untyped-ratchet-5.json` | 3 |
| #1085 | `fix/858-untyped-ratchet-7` | **MISSING** | - |
| #1087 | `ci/proofs-drop-intel-pin` | **MISSING** | - |
| #1088 | `fix/858-untyped-ratchet-3` | committed `.quorum/fix-858-untyped-ratchet-3.json` | 3 |
| #1089 | `fix/858-untyped-ratchet-4` | **MISSING** | - |
| #1091 | `ci/881-provenance-security-x64` | **MISSING** | - |
| #1094 | `fix/847-inode-pseudo-mask` | **MISSING** | - |
| #1097 | `feat/cloud-instance-ami-pin` | committed `.quorum/feat-cloud-instance-ami-pin.json` | 3 |
| #1099 | `fix/cloud-instance-ami-unused-import` | **MISSING** | - |
| #1100 | `fix/1097-ami-unused-import` | **MISSING** | - |
| #1101 | `feat/cloud-instance-net-placement` | committed `.quorum/feat-cloud-instance-net-placement.json` | 3 |
| #1102 | `fix/cloud-instance-destroy-closed` | **MISSING** | - |
| #1103 | `fix/1078-typed-errors-registry-push` | **MISSING** | - |
| #1104 | `fix/1074-typed-errors-image-cmd` | committed `.quorum/fix-1074-typed-errors-image-cmd.json` | 3 |
| #1105 | `fix/1093-retry-test-host-independent` | **MISSING** | - |
| #1106 | `fix/876-572-quorum-receipt` | committed `.quorum/fix-876-572-quorum-receipt.json` | 3 |
| #1107 | `fix/858-remaining-tests` | committed `.quorum/fix-858-remaining-tests.json` | 3 |
| #1108 | `fix/859-pre-push-fmt-only` | **MISSING** | - |
| #1118 | `feat/894-pool-reconcile-plan` | committed `.quorum/feat-894-pool-reconcile-plan.json` | 3 |
| #1119 | `fix/858-untyped-sites-under-ceiling` | **MISSING** | - |
| #1120 | `refactor/cloud-instance-parse-validate` | committed `.quorum/refactor-cloud-instance-parse-validate.json` | 3 |
| #1126 | `fix/cloud-instance-validate-resolved` | committed `.quorum/fix-cloud-instance-validate-resolved.json` | 3 |
| #1127 | `feat/cloud-instance-profile` | **MISSING** | - |
| #1138 | `feat/894-pool-jit-config` | **MISSING** | - |
| #1140 | `feat/894-pool-interruption-orphans` | committed `.quorum/feat-894-pool-interruption-orphans.json` | 3 |
| #1142 | `feat/cloud-instance-storage` | **MISSING** | - |
| #1144 | `fix/858-pack-sigpipe-etxtbsy` | **MISSING** | - |
| #1145 | `feat/cloud-instance-userdata` | committed `.quorum/feat-cloud-instance-userdata.json` | 3 |
| #1146 | `feat/cloud-instance-launch-io` | committed `.quorum/feat-cloud-instance-launch-io.json` | 3 |
| #1148 | `fix/876-quorum-store-receipt` | **MISSING** | - |
| #1151 | `ci/route3-gate-seat-pin` | **MISSING** | - |
| #1152 | `release/v1.34.2` | committed `.quorum/release-v1.34.2.json` | 3 |
| #1154 | `fix/1153-audit-quinn` | **MISSING** | - |
| #1156 | `fix/audit-chrono-0543` | committed `.quorum/fix-audit-chrono-0543.json` | 3 |
| #1175 | `chore/postmerge-receipts-batches-1-3` | committed `.quorum/chore-postmerge-receipts-batches-1-3.json` | 3 |
| #1201 | `chore/postmerge-receipts-gh1176-1178` | committed `.quorum/chore-postmerge-receipts-gh1176-1178.json` | 3 |
| #1202 | `fix/postmerge-defects-pack1` | committed `.quorum/fix-postmerge-defects-pack1.json` | 3 |
| #1203 | `fix/postmerge-defects-pack2` | **MISSING** | - |
| #1204 | `fix/postmerge-defects-pack3` | committed `.quorum/fix-postmerge-defects-pack3.json` | 3 |
| #1205 | `fix/postmerge-defects-pack4` | committed `.quorum/fix-postmerge-defects-pack4.json` | 3 |
| #1207 | `fix/postmerge-defects-pack6` | committed `.quorum/fix-postmerge-defects-pack6.json` | 3 |
| #1209 | `fix/postmerge-defects-pack5b` | committed `.quorum/fix-postmerge-defects-pack5b.json` | 3 |
| #1211 | `fix/postmerge-defects-pack7b` | committed `.quorum/fix-postmerge-defects-pack7b.json` | 3 |
| #1212 | `fix/gate-t-paginate-recut-gh1164` | **MISSING** | - |
| #1217 | `ci/burst-optin-lockfile-examples-filehealth` | **MISSING** | - |
| #1218 | `fix/book-v1.34.1-cut` | **MISSING** | - |
| #1219 | `rungrp1-pr-spot-labels` | **MISSING** | - |
| #1220 | `fix/main-red-launch-io-token` | **MISSING** | - |
| #1221 | `fix/1173-release-check-paged-window` | **MISSING** | - |
| #1222 | `fix/andon-proofs-provers-label` | **MISSING** | - |
| #1223 | `pack/990-receipts-prpage-lint` | **MISSING** | - |
| #1224 | `fix/1174-gate-window-membership` | **MISSING** | - |
| #1225 | `pack/913-ci-hygiene` | **MISSING** | - |
| #1226 | `fix/858-failing-integration-tests` | **MISSING** | - |
| #1227 | `fix/1143-docker-completion-check` | **MISSING** | - |
| #1228 | `fix/1139-orphan-runner-cleanup` | **MISSING** | - |
| #1229 | `fix/1141-triage-at-filing` | **MISSING** | - |
| #1231 | `fix/golden-hash-network-field` | **MISSING** | - |
| #1233 | `ci/858-wire-seven-falsifiers` | **MISSING** | - |
| #1237 | `fix/gate-t-t8b-unpublished` | **MISSING** | - |
| #1238 | `fix/gate-t-944-alias` | **MISSING** | - |
| #1240 | `chore/release-labels-v1.34.2` | **MISSING** | - |
| #1241 | `docs/changelog-1.34.2-refresh` | **MISSING** | - |
| #1243 | `feat/s3-bucket` | **MISSING** | - |
| #1244 | `fix/s3-bucket-probe-tls` | **MISSING** | - |
| #1246 | `pmat-1245-cb200-refactor` | **MISSING** | - |
| #1247 | `fix/cb21xx-roadmap-ratchet` | **MISSING** | - |
| #1248 | `docs/crux-1342-row` | **MISSING** | - |
| #1249 | `gate/t-alias-1247` | **MISSING** | - |
| #1250 | `docs/s3-bucket-followup` | **MISSING** | - |
| #1251 | `fix/731-autoinstall-seed` | **MISSING** | - |
| #1252 | `fix/1239-drop-rustsec-0104` | **MISSING** | - |
| #1255 | `docs/changelog-1.34.2-unreleased` | **MISSING** | - |

Total: 88 PR(s); committed 31; missing 57. Store receipts for window PRs: 0.
