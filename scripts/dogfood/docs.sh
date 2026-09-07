#!/usr/bin/env bash
# Dogfood gate D — every documented invocation runs, and every claim reconciles.
#
# Exit code is the gate. The only line a caller must read is the final
# `GATE D PASS|FAIL <detail>`.
#
# README.md is the first thing anyone runs. A command in it that no longer
# parses is a defect that ships to every new user at once, and nothing in the
# test suite looks at README: `cargo test` compiles what is under `src/` and
# `tests/`, and prose is neither. So this gate extracts every fenced
# `forjar …` line and executes it.
#
# ------------------------------------------------------------------ sandboxing
#
# Two things are rebound before anything runs, and both are load-bearing:
#
#   HOME / XDG_DATA_HOME  `forjar cache`, `forjar store` and `forjar pin` read
#                         and write ~/.local/share/forjar/store. Measured: a
#                         bare `forjar cache push user@host:path` exits 0 after
#                         touching the operator's real store. A gate that
#                         mutates the machine it is auditing is not an audit.
#   file operands         `forjar.yaml`, `state` and `Makefile` in the README
#                         name files in the reader's tree, not this one. They
#                         are rebound to the fixtures under
#                         tests/fixtures/dogfood/ and to a temp state dir.
#
# What is therefore PROVEN is the argv SHAPE: this verb, these flags, in this
# order, against a config of this kind, exits 0. What is NOT proven is that the
# reader's `forjar.yaml` exists. That is the honest boundary and it is the one
# that catches doc rot — a renamed flag or a deleted subcommand fails here.
#
# `apply` and `make` are the two invocations whose argv is ADDED to: `--yes`,
# because a gate cannot answer a confirmation prompt. Both run against fixtures
# inside the temp dir and never against a real host. Nothing else is altered —
# an added flag is a change to what the README claims, so each one is named
# here and at its call site.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

FIXTURES="tests/fixtures/dogfood"
CSV="docs/audits/surface_audit.csv"
# The cookbook (paiml/forjar-cookbook, public) is a SEPARATE repository. It is
# expected as a sibling of the MAIN repository (git-common-dir's parent, so a
# worktree finds the same sibling), or wherever COOKBOOK points; ci.yml's
# dogfood job clones it into the runner's temp dir and sets COOKBOOK. Its
# absence is UNMEASURED, never a skip: a gate that said PASS about recipes it
# never saw would be the vacuous pass found by the PMAT-163 merge review.
_common="$(git rev-parse --git-common-dir 2>/dev/null || printf '.git')"
_common_abs="$(cd "$(dirname "$_common")" 2>/dev/null && pwd -P)"
COOKBOOK="${COOKBOOK:-${_common_abs}/../forjar-cookbook}"

# Vacuity floor: today's count of fenced `forjar` invocations in README.md.
# A README that stopped documenting anything would satisfy every loop below.
MIN_INVOCATIONS=15

fail() {
  echo "GATE D FAIL $1"
  exit 1
}

BIN="$(bash "$(dirname "${BASH_SOURCE[0]}")/lib/binary.sh")"

for f in local-files.yaml stack-a.yaml stack-b.yaml single-stack.yaml Makefile; do
  if [ ! -f "${FIXTURES}/${f}" ]; then
    fail "${FIXTURES}/${f} is missing — the documented invocations would have nothing to run against"
  fi
done
if [ ! -f "$CSV" ]; then
  fail "${CSV} is missing — run gate C first; the doc/surface reconciliation below reads it"
fi

SANDBOX="$(mktemp -d)"
trap 'rm -rf "${SANDBOX:?}"' EXIT

# ----------------------------------------------------- run the documented lines
report="$(
  python3 - "$BIN" "$SANDBOX" "$FIXTURES" "$CSV" <<'PY'
import os, re, shlex, subprocess, sys

BIN, SANDBOX, FIXTURES, CSV = sys.argv[1:5]
README = "README.md"

# ------------------------------------------------------------ extraction
#
# Only fences whose language is `bash`/`sh`/`console` hold commands. The two
# `toml` fences hold a dependency declaration (`forjar = "1.25"`), which is a
# CLAIM and is reconciled separately below, not a command.
SHELL_FENCES = {"bash", "sh", "shell", "console", ""}
invocations = []
claims = []
inside, lang = False, ""
for lineno, raw in enumerate(open(README, encoding="utf-8").read().splitlines(), 1):
    if raw.startswith("```"):
        inside, lang = (not inside), (raw[3:].strip() if not inside else "")
        continue
    if not inside:
        continue
    text = re.sub(r"^\$\s+", "", raw.strip())
    if lang == "toml":
        m = re.match(r'^forjar\s*=\s*(?:"([^"]+)"|\{[^}]*version\s*=\s*"([^"]+)")', text)
        if m:
            claims.append((lineno, m.group(1) or m.group(2)))
        continue
    if lang in SHELL_FENCES and re.match(r"^forjar(\s|$)", text):
        invocations.append((lineno, text))

if len(invocations) < 1:
    sys.exit("README.md holds no fenced `forjar` invocation at all")

# ------------------------------------------------------------ preparation
#
# One temp tree per run. The fixture config is copied in with its `root:`
# param repointed inside the sandbox, so a `file` resource cannot write outside
# it even if a verb believed read-only is not.
home = os.path.join(SANDBOX, "home")
work = os.path.join(SANDBOX, "work")
state = os.path.join(SANDBOX, "state")
for d in (home, work, state):
    os.makedirs(d, exist_ok=True)
cfg = os.path.join(work, "forjar.yaml")
src = open(os.path.join(FIXTURES, "local-files.yaml"), encoding="utf-8").read()
open(cfg, "w", encoding="utf-8").write(
    re.sub(r"^  root: .*$", f"  root: {os.path.join(SANDBOX, 'root')}", src, flags=re.M))
# The Makefile group gets its OWN directory. `forjar import-makefile Makefile
# -o forjar.yaml` writes a forjar.yaml, and if it wrote it beside the fixture
# config the very next `forjar apply -f forjar.yaml` would be applying the
# generated build graph instead of the fixture — a gate silently measuring
# something other than what it names.
mwork = os.path.join(SANDBOX, "make")
os.makedirs(mwork, exist_ok=True)
makefile = os.path.join(mwork, "Makefile")
open(makefile, "w", encoding="utf-8").write(
    open(os.path.join(FIXTURES, "Makefile"), encoding="utf-8").read())
mcfg = os.path.join(mwork, "forjar.yaml")

env = dict(os.environ)
env.update({"HOME": home, "XDG_DATA_HOME": os.path.join(home, ".local", "share"),
            "XDG_CONFIG_HOME": os.path.join(home, ".config"),
            "XDG_CACHE_HOME": os.path.join(home, ".cache"),
            "NO_COLOR": "1"})
# The binary store, empty but PRESENT. `forjar cache list` on a machine with no
# store directory exits 1 with "No such file or directory" rather than listing
# nothing, so a bare temp HOME would report the README's `cache list` line as
# broken when what is missing is the reader's store, not the command.
os.makedirs(os.path.join(env["XDG_DATA_HOME"], "forjar", "store"), exist_ok=True)

# The operands the README names in the reader's tree, and what they become
# here. `make` and `import-makefile` are the build-graph group and resolve
# `forjar.yaml` to the generated one in their own directory.
MAKE_VERBS = {"make", "import-makefile"}
REBIND = {"forjar.yaml": cfg, "state": state, "Makefile": makefile}
MAKE_REBIND = {"forjar.yaml": mcfg, "state": state, "Makefile": makefile}

# Documented invocations that DO NOT WORK, each with the defect that stops
# them. This is a ratchet, not an excuse list: a line named here that starts
# passing is a failure too ("remove it"), so the exception cannot outlive the
# defect, and a line NOT named here that fails is an ordinary gate failure.
#
# PMAT-163 may not edit `src/`, so the defect is pinned rather than fixed.
KNOWN_BROKEN = {
    "forjar make clean": (
        "a `phony: true` task generated by `import-makefile` can never converge: "
        "the command runs and exits 0, then forjar's own state query answers "
        "`task=pending` and JIDOKA reports NOT CONVERGED. Reproduce with "
        "`forjar import-makefile Makefile -o forjar.yaml && forjar make clean "
        "--state-dir <d> --yes` on tests/fixtures/dogfood/Makefile. Needs a "
        "ticket against the task provider's convergence check for phony targets."
    ),
}


def known_broken(text):
    for prefix, reason in KNOWN_BROKEN.items():
        if text.startswith(prefix):
            return prefix, reason
    return None, None


# A token the README leaves for the reader to fill in. There is no honest value
# to substitute — `<hash>` names a store entry that does not exist and
# `user@host:path` names a machine this gate must not contact — so these are
# asserted to PARSE rather than to succeed.
PLACEHOLDER = re.compile(r"<[^>]+>|\buser@host\b")


def rebind(argv, table):
    return [table.get(a, a) for a in argv]


def clap_rejected(proc):
    """Did clap refuse the argv, as opposed to forjar refusing the request?

    clap exits 2 on a usage error; a verb that ran and failed exits 1 with a
    domain message. That is the whole distinction this gate turns on: a
    documented flag that no longer exists is a usage error, a documented
    command that needs a store entry the sandbox has not got is not.
    """
    blob = proc.stdout + proc.stderr
    return proc.returncode == 2 or "unrecognized subcommand" in blob \
        or "unexpected argument" in blob or "error: invalid value" in blob


rows, failures = [], []
surface = {ln.split(",")[2] for ln in open(CSV, encoding="utf-8").read().splitlines()[1:]
           if ln.startswith("cli,")}

# `import-makefile` must run before `make`, which consumes what it writes. The
# README presents them in the other order because it is explaining the feature,
# not scripting it; the ordering here is the gate's, and it is declared rather
# than inferred.
ORDER_FIRST = "import-makefile"
invocations.sort(key=lambda iv: (ORDER_FIRST not in iv[1], iv[0]))

for lineno, text in invocations:
    # `forjar init my-infra && cd my-infra` is a shell conjunction. Only the
    # forjar half is executable here; the `cd` is the README telling a reader
    # where to stand.
    head = text.split("&&")[0].strip()
    # comments=True drops the README's trailing `# what this does`,
    # which shlex would otherwise hand to clap as an argument named `#`.
    argv = shlex.split(head, comments=True)[1:]

    verb = argv[0] if argv else ""
    sub = " ".join(argv[:2]) if len(argv) > 1 and not argv[1].startswith("-") else verb
    if verb not in surface and sub not in surface:
        failures.append(f"README:{lineno}: `{text}` names `{verb}`, which is not in {CSV} — "
                        f"the docs promise a command the binary does not ship")
        rows.append((lineno, "UNKNOWN-VERB", text))
        continue

    if PLACEHOLDER.search(head):
        proc = subprocess.run([BIN] + argv, cwd=work, env=env,
                              capture_output=True, text=True, timeout=300)  # placeholder arm
        if clap_rejected(proc):
            failures.append(f"README:{lineno}: `{text}` is rejected by clap "
                            f"(exit {proc.returncode}): {(proc.stderr or proc.stdout).splitlines()[:1]}")
        rows.append((lineno, "PARSE", text))
        continue

    in_make = verb in MAKE_VERBS
    run_argv = rebind(argv, MAKE_REBIND if in_make else REBIND)
    if verb == "apply":
        run_argv = run_argv + ["--state-dir", state, "--yes"]
    if verb == "make":
        # Same reason as `apply`: `forjar make` prompts `Apply N change(s)?`
        # and a gate has no one to answer it. The changes are three `task`
        # resources whose `working_dir` is the sandbox.
        run_argv = run_argv + ["--yes"]
    if verb == "make":
        # `make` converges the imported graph and so needs a state dir;
        # `import-makefile` only translates and does not accept one.
        run_argv = run_argv + ["--state-dir", os.path.join(SANDBOX, "make-state")]
    proc = subprocess.run([BIN] + run_argv, cwd=mwork if in_make else work, env=env,
                          capture_output=True, text=True, timeout=600)
    prefix, reason = known_broken(text)
    if prefix is not None:
        if proc.returncode == 0:
            failures.append(f"README:{lineno}: `{text}` now SUCCEEDS but is still listed "
                            f"in KNOWN_BROKEN — remove it; an exception that outlives its "
                            f"defect is how a gate stops measuring")
            rows.append((lineno, "FIXED", text))
        else:
            rows.append((lineno, "KNOWN-BROKEN", text))
        continue
    if proc.returncode != 0:
        failures.append(f"README:{lineno}: `{text}` exited {proc.returncode}\n"
                        f"        as run: {BIN} {' '.join(run_argv)}\n"
                        f"        {(proc.stderr or proc.stdout).strip().splitlines()[:3]}")
        rows.append((lineno, "FAIL", text))
    else:
        rows.append((lineno, "RUN", text))

# ------------------------------------------------ the README's version claims
#
# `forjar = "1.25"` is a caret requirement. It reconciles if the version this
# tree builds satisfies it; a README pinning a version the crate has moved past
# sends a new user to an older API than the one the page describes.
cargo_version = re.search(r'^version = "(.*)"', open("Cargo.toml", encoding="utf-8").read(),
                          re.M).group(1)
have = [int(x) for x in re.findall(r"\d+", cargo_version)[:3]]
for lineno, req in claims:
    want = [int(x) for x in re.findall(r"\d+", req)[:3]]
    want += [0] * (3 - len(want))
    if want[0] != have[0] or (want[0], want[1], want[2]) > tuple(have):
        failures.append(f"README:{lineno}: claims `forjar = \"{req}\"`, which "
                        f"{cargo_version} does not satisfy")

print(f"COUNT {len(invocations)}")
print(f"BROKEN {sum(1 for r in rows if r[1] == 'KNOWN-BROKEN')}")
for prefix, reason in sorted(KNOWN_BROKEN.items()):
    print(f"NOTE known-broken `{prefix}`: {reason}")
for lineno, kind, text in sorted(rows):
    print(f"ROW {lineno} {kind} {text}")
for f in failures:
    print(f"FAILURE {f}")
PY
)"

printf '%s\n' "$report" | grep '^ROW ' | sed 's/^ROW /  /'
n_inv="$(printf '%s\n' "$report" | sed -n 's/^COUNT //p')"
if [ "${n_inv:-0}" -lt "$MIN_INVOCATIONS" ]; then
  fail "README.md holds ${n_inv:-0} fenced forjar invocation(s), floor is ${MIN_INVOCATIONS} — a README that documents nothing passes every check above"
fi
if printf '%s\n' "$report" | grep -q '^FAILURE '; then
  printf '%s\n' "$report" | grep '^FAILURE ' | sed 's/^FAILURE /  /'
  fail "$(printf '%s\n' "$report" | grep -c '^FAILURE ') documented invocation(s) or claim(s) do not hold"
fi

# --------------------------------------------------------------- the cookbook
#
# Every recipe and example config in the cookbook must validate against THIS
# binary, and there must be some: a cookbook check over zero recipes reports
# success about nothing.
if [ ! -d "$COOKBOOK" ]; then
  fail "cookbook not found at ${COOKBOOK} — UNMEASURED: check out paiml/forjar-cookbook beside the repository or set COOKBOOK=<path>; the documented-claims gate cannot pass over recipes it never saw"
fi
if [ -d "$COOKBOOK" ]; then
  shopt -s nullglob
  recipes=("$COOKBOOK"/recipes/*.yaml "$COOKBOOK"/examples/*.yaml)
  shopt -u nullglob
  if [ "${#recipes[@]}" -eq 0 ]; then
    fail "${COOKBOOK} exists but holds no recipe yaml — a cookbook check over zero recipes reports success about nothing"
  fi
  bad=0
  for r in "${recipes[@]}"; do
    if ! "$BIN" validate -f "$r" >/dev/null 2>&1; then
      echo "  cookbook INVALID ${r}"
      bad=$((bad + 1))
    fi
  done
  if [ "$bad" -ne 0 ]; then
    fail "${bad} of ${#recipes[@]} cookbook config(s) under ${COOKBOOK} do not validate against this binary"
  fi
  cookbook_note="${#recipes[@]} cookbook config(s) validate"
fi

n_broken="$(printf '%s\n' "$report" | sed -n 's/^BROKEN //p')"
printf '%s\n' "$report" | grep '^NOTE ' | sed 's/^NOTE /  /'

echo "GATE D PASS ${n_inv} documented invocation(s) from README.md run or parse against the fixtures (${n_broken:-0} pinned as known-broken above); version claims reconcile with Cargo.toml; ${cookbook_note}"

# mutation: change `forjar plan -f forjar.yaml` in README.md to
# `forjar plan --f forjar.yaml` — clap then exits 2 on the rebound argv and the
# gate reports it as a documented invocation that does not hold.
