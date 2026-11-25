#!/usr/bin/env bash
# test-reap.sh -- the RAID reaper must go red on each bad fixture and green on each clean one, and the
# fixture suite itself must go red against mutated copies of the scripts (falsification).
# Fixtures live in mktemp only; no reaper ever sees a real /mnt/nvme-raid0 path.
# mutation: change the 15-day fixture age below to 13 and the "tmp 15d reaped" case goes red.
set -euo pipefail
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
tmp=$(mktemp -d)
[ -n "$tmp" ] && [ -d "$tmp" ] || exit 2
pids=()
cleanup() {
  local p
  for p in "${pids[@]:-}"; do [ -n "$p" ] && kill "$p" 2>/dev/null || true; done
  chmod -R u+rwx "${tmp:?}" 2>/dev/null || true
  rm -rf "${tmp:?}"
}
trap cleanup EXIT

fail=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fail=$((fail + 1)); }
check() { # check <name> <cmd...> : cmd must succeed
  local n=$1; shift
  if "$@"; then ok "$n"; else bad "$n"; fi
}
ago() { touch -h -d "$1 days ago" "${@:2}"; }
age_tree() { find "$2" -exec touch -h -d "$1 days ago" {} +; }
gitq() { git -c core.hooksPath=/dev/null -c user.email=t@t -c user.name=t -c init.defaultBranch=main "$@"; }

# run_suite <label> <script-dir> : build a fresh fixture, run every case; adds to $fail.
run_suite() {
  local label=$1 sd=$2 f; f=$tmp/$label
  mkdir -p "$f"/{tmp,rescue,state,inbox,rep,wtroot}
  export REAP_TMP_ROOT=$f/tmp REAP_RESCUE_DIR=$f/rescue REAP_STATE_DIR=$f/state REAP_LEDGER=$f/ledger.tsv
  export REAP_INBOX=$f/inbox/inbox.md REAP_REPORT_DIR=$f/rep REAP_RAID_ROOT=$f REAP_WT_ROOTS=$f/wtroot
  unset FORJAR_BUDGET_EXECUTE FORJAR_DF_CMD REAP_DF_CMD REAP_TTL_DAYS REAP_BUILD_DAYS REAP_WT_IDLE_DAYS REAP_ALARM_PCT
  local t=$f/tmp

  # --- tmp TTL fixtures ---
  mkdir "$t"/{old15,old14,new13,held,deliv,sub}; echo x >"$t"/old15/a; echo x >"$t"/old14/a; echo x >"$t"/new13/a
  echo x >"$t"/held/a; mkdir "$t/deliv/receipts"; echo x >"$t"/deliv/clip.mp4; echo x >"$t/deliv/receipts/r.json"
  age_tree 15 "$t/old15"; age_tree 14 "$t/old14"; age_tree 13 "$t/new13"; age_tree 20 "$t/held"; age_tree 20 "$t/deliv"
  ( exec 9<"$t/held/a"; exec sleep 300 >/dev/null 2>&1 ) & pids+=($!)
  # build output: 4 d reaped, 2 d kept; target without a cargo marker is not build output
  mkdir -p "$t/proj/target" "$t/proj2/target" "$t/proj3/target"
  touch "$t/proj/target/CACHEDIR.TAG" "$t/proj2/target/CACHEDIR.TAG" "$t/proj3/target/x"
  age_tree 4 "$t/proj/target"; age_tree 2 "$t/proj2/target"; age_tree 4 "$t/proj3/target"
  # burst: record present -> output deleted; absent -> kept and unmeasured
  mkdir "$t"/burst-1-out "$t"/burst-2-out; echo out >"$t"/burst-1-out/big.bin; echo out >"$t"/burst-2-out/big.bin
  printf '{"sha256":"%064d"}\n' 0 >"$t"/burst-1-out/result.json
  age_tree 30 "$t/burst-1-out"; age_tree 30 "$t/burst-2-out"
  local before after rc
  before=$(cd "$t" && find . -printf '%p %T@ %s\n' | sort | sha256sum)

  rc=0; "$sd/reap-tmp.sh" >"$f/dry.out" 2>&1 || rc=$?
  after=$(cd "$t" && find . -printf '%p %T@ %s\n' | sort | sha256sum)
  check "$label dry-run default changes nothing" [ "$before" = "$after" ]
  check "$label dry-run lists candidates" grep -q "WOULD-REMOVE R1-tmp $t/old15" "$f/dry.out"

  rc=0; FORJAR_BUDGET_EXECUTE=1 "$sd/reap-tmp.sh" >"$f/exec.out" 2>&1 || rc=$?
  check "$label tmp 15d reaped" [ ! -e "$t/old15" ]
  check "$label tmp exactly 14d reaped" [ ! -e "$t/old14" ]
  check "$label tmp 13d kept" [ -e "$t/new13/a" ]
  check "$label open file holds the tree" [ -e "$t/held/a" ]
  check "$label deliverable moved to rescue not deleted" [ -e "$f/rescue/$(date -u +%F)/deliv/clip.mp4" ]
  check "$label build 4d reaped" [ ! -e "$t/proj/target" ]
  check "$label build 2d kept" [ -e "$t/proj2/target" ]
  check "$label target without cargo marker kept" [ -e "$t/proj3/target" ]
  check "$label burst with record deleted, record kept" [ ! -e "$t/burst-1-out/big.bin" ] && [ -e "$t/burst-1-out/result.json" ]
  check "$label burst without record kept" [ -e "$t/burst-2-out/big.bin" ]
  check "$label unrecorded burst is unmeasured (rc 1)" [ "$rc" -eq 1 ]
  check "$label ledger records the removals" grep -q "R1-tmp	$t/old15" "$f/ledger.tsv"

  # --- worktrees ---
  local o=$f/origin.git r=$f/repo w=$f/wtroot
  git init -q --bare "$o"; gitq clone -q "$o" "$r" 2>/dev/null
  (cd "$r" && echo 1 >f && gitq add f && gitq commit -qm init && gitq push -q origin main 2>/dev/null)
  mkwt() { # mkwt <name> : pushed clean worktree
    gitq -C "$r" worktree add -q -b "$1" "$w/$1" && gitq -C "$w/$1" push -q origin "$1" 2>/dev/null
  }
  mkwt merged; mkwt dirty; mkwt unpushed; mkwt idle8; mkwt idle6; mkwt openpr
  echo u >"$w/dirty/untracked"
  (cd "$w/unpushed" && echo 2 >g && gitq add g && gitq commit -qm local)
  age_tree 1 "$w/merged"; age_tree 8 "$w/idle8"; age_tree 6 "$w/idle6"; age_tree 20 "$w/dirty"; age_tree 20 "$w/unpushed"
  printf 'merged done\nopenpr open\nidle6 none\n' >"$f/prmap"
  cat >"$f/prcmd" <<PRC
#!/usr/bin/env bash
awk -v b="\$2" '\$1==b{print \$2; f=1} END{exit !f}' "$f/prmap"
PRC
  chmod +x "$f/prcmd"
  export REAP_REPOS=$r REAP_PR_CMD=$f/prcmd
  rc=0; "$sd/reap-worktrees.sh" >"$f/wdry.out" 2>&1 || rc=$?
  check "$label worktree dry-run removes nothing" [ -d "$w/merged" ] && [ -d "$w/idle8" ]
  rc=0; FORJAR_BUDGET_EXECUTE=1 "$sd/reap-worktrees.sh" >"$f/wexec.out" 2>&1 || rc=$?
  check "$label merged PR worktree removed" [ ! -e "$w/merged" ]
  check "$label removed worktree gone from git worktree list" bash -c "! git -C '$r' worktree list | grep -q '$w/merged'"
  check "$label idle 8d clean+pushed removed" [ ! -e "$w/idle8" ]
  check "$label idle 6d (PR none) kept" [ -d "$w/idle6" ]
  check "$label open PR kept" [ -d "$w/openpr" ]
  check "$label dirty worktree kept" [ -d "$w/dirty" ]
  check "$label unpushed worktree kept" [ -d "$w/unpushed" ]
  check "$label KEEP list names dirty and unpushed" bash -c "grep -q dirty '$f/state/keep-worktrees.tsv' && grep -q unpushed '$f/state/keep-worktrees.tsv'"
  # PR lookup failure = not_measured, no action
  mkwt flaky; age_tree 1 "$w/flaky"; printf '#!/usr/bin/env bash\nexit 3\n' >"$f/prcmd"
  rc=0; FORJAR_BUDGET_EXECUTE=1 "$sd/reap-worktrees.sh" >"$f/wnm.out" 2>&1 || rc=$?
  check "$label PR lookup failure keeps worktree, rc 1" [ -d "$w/flaky" ] && [ "$rc" -eq 1 ]
  check "$label worktree script never calls rm -rf" bash -c "! grep -q 'rm -rf' '$sd/reap-worktrees.sh'"

  # --- alarm 69 vs 70 ---
  printf '#!/usr/bin/env bash\necho "69 500"\n' >"$f/df69"; printf '#!/usr/bin/env bash\necho "70 480"\n' >"$f/df70"
  chmod +x "$f/df69" "$f/df70"
  printf "#!/usr/bin/env bash\necho \"75 400\"\n" >"$f/df75"; printf "#!/usr/bin/env bash\necho \"50 900\"\n" >"$f/df50"
  chmod +x "$f/df75" "$f/df50"
  rc=0; FORJAR_DF_CMD=$f/df69 "$sd/alarm.sh" >/dev/null 2>&1 || rc=$?
  check "$label alarm 69% green, no inbox line" [ "$rc" -eq 0 ] && [ ! -s "$f/inbox/inbox.md" ]
  rc=0; FORJAR_DF_CMD=$f/df70 "$sd/alarm.sh" >/dev/null 2>&1 || rc=$?
  check "$label alarm 70% red with inbox line" [ "$rc" -eq 1 ] && grep -q '| reaper | RAID 70% used 480G free' "$f/inbox/inbox.md"
  FORJAR_DF_CMD=$f/df70 "$sd/alarm.sh" >/dev/null 2>&1 || true
  check "$label alarm dedupes on unchanged state" [ "$(wc -l <"$f/inbox/inbox.md")" -eq 1 ]

  # --- weekly report: golden rows ---
  FORJAR_DF_CMD=$f/df50 REAP_CLASSES="tmp" "$sd/report.sh" >/dev/null
  local rep; rep=$(ls "$f/rep"/raid-weekly-*.md)
  check "$label report names used/free and 7d reclaimed" grep -Eq '^used: 50%  free: 900G  reclaimed_7d_gb: [0-9]+\.[0-9]{3}$' "$rep"
  check "$label report lists KEEP and not_measured" bash -c "grep -q '## KEEP' '$rep' && grep -q 'unpushed' '$rep' && grep -q 'no result record' '$rep'"

  # --- anti-inertness: used 75%, nothing eligible -> rc non-zero; 50% -> rc 0 ---
  local e=$f/empty; mkdir -p "$e/tmp"
  rc=0; REAP_TMP_ROOT=$e/tmp REAP_REPOS='' REAP_INBOX=$f/inbox/x.md FORJAR_DF_CMD=$f/df75 "$sd/reap-all.sh" >/dev/null 2>&1 || rc=$?
  check "$label used 75% and nothing reclaimed exits non-zero" [ "$rc" -ne 0 ]
  rc=0; REAP_TMP_ROOT=$e/tmp REAP_REPOS='' REAP_INBOX=$f/inbox/x.md FORJAR_DF_CMD=$f/df50 "$sd/reap-all.sh" >/dev/null 2>&1 || rc=$?
  check "$label used 50% idle pass is green" [ "$rc" -eq 0 ]

  unset REAP_REPOS REAP_PR_CMD
}

run_suite real "$here"
echo "--- falsification: the suite must go red on mutated scripts ---"
mutate() { # mutate <name> <file> <sed-expr>
  local name=$1 file=$2 expr=$3 d=$tmp/mut-$1 outer=$fail
  mkdir -p "$d"; cp "$here"/*.sh "$d"/
  sed -i -E "$expr" "$d/$file"
  cmp -s "$here/$file" "$d/$file" && { echo "FAIL mutant $name did not change $file"; fail=$((fail + 1)); return; }
  fail=0; run_suite "m-$name" "$d" >"$tmp/m-$name.log" 2>&1 || true
  local got=$fail; fail=$outer
  if [ "$got" -gt 0 ]; then ok "mutant $name killed ($got cases red)"; else bad "mutant $name SURVIVED"; fi
}
mutate ttl-14-to-140 reap-tmp.sh 's/REAP_TTL_DAYS:-14/REAP_TTL_DAYS:-140/'
mutate build-3-to-30 reap-tmp.sh 's/REAP_BUILD_DAYS:-3/REAP_BUILD_DAYS:-30/'
mutate wt-idle-7-to-70 reap-worktrees.sh 's/REAP_WT_IDLE_DAYS:-7/REAP_WT_IDLE_DAYS:-70/'
mutate alarm-70-to-71 alarm.sh 's/REAP_ALARM_PCT:-70/REAP_ALARM_PCT:-71/'
mutate drop-open-check lib.sh 's/if has_open "\$p"; then note_keep "\$p" "open file"; return 0; fi//'
mutate drop-dirty-check reap-worktrees.sh 's/\[ -n "\$\(git -C "\$wt" status --porcelain\)" \]/false/; s/worktree remove "\$wt"/worktree remove --force "\$wt"/'
mutate drop-unpushed-check reap-worktrees.sh 's/if ! pushed "\$wt" "\$br"; then/if false; then/'
mutate drop-record-check reap-tmp.sh 's/has_record "\$d" \|\| \{ note_nm/true || { note_nm/'

if [ "$fail" -ne 0 ]; then echo "REAP-TEST FAIL $fail case(s)"; exit 1; fi
echo "REAP-TEST PASS"
