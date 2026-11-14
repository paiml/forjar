#!/usr/bin/env bash
# bashrs-pin-skew: every bashrs pin forjar and the fleet hosts carry, compared
# with the newest bashrs release tag. PMAT-1015.
#
# LAB check (C324): it cannot block a merge or a release. It runs at night on
# an idle machine (`make lab-bashrs-pin-skew`) and is wired into NO PR,
# merge-queue or release job. A red opens or updates its one ticket with its
# one owner (forjar-cop), then stays silent. Promoting it to a GATE is an
# operator ruling, after three green nights on main.
#
# Sources read (working tree, or git objects with --rev):
#   forjar  Cargo.toml `bashrs = "..."` requirement floor, Cargo.lock resolved
#   infra   machines/*/forjar.yaml: `bashrs_version:` params, the
#           `fleet-bashrs-pin` file content, literal `bashrs --version X.Y.Z`
#           in `cargo install` lines
#   latest  --latest VER, else $BASHRS_LATEST, else the highest vX.Y.Z tag
#           from `git ls-remote` of $BASHRS_REMOTE (git protocol, no REST call)
#
# Exit: 0 every pin == latest; 1 skew; 2 not_measured (latest unknown, or a
# source missing). Never decide anything on exit 2 (L23).
set -euo pipefail

usage() {
    echo "usage: $0 [--latest X.Y.Z] [--forjar DIR] [--infra DIR] [--rev REV]" >&2
    exit 2
}

LATEST="${BASHRS_LATEST:-}"
FORJAR_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
INFRA_DIR="${INFRA_DIR:-}"
REV=""
REMOTE="${BASHRS_REMOTE:-https://github.com/paiml/bashrs}"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --latest) LATEST="${2:-}"; shift 2 ;;
        --forjar) FORJAR_DIR="${2:-}"; shift 2 ;;
        --infra) INFRA_DIR="${2:-}"; shift 2 ;;
        --rev) REV="${2:-}"; shift 2 ;;
        -h|--help) usage ;;
        *) usage ;;
    esac
done

not_measured() {
    echo "bashrs-pin-skew: NOT_MEASURED $1" >&2
    exit 2
}

# rd REPO PATH: file bytes from the working tree, or from REV when given.
rd() {
    if [ -n "$REV" ]; then
        git -C "$1" show "$REV:$2" 2>/dev/null
    else
        cat -- "$1/$2" 2>/dev/null
    fi
}

# machines REPO: every machines/<name> directory that holds a forjar.yaml.
machines() {
    if [ -n "$REV" ]; then
        git -C "$1" ls-tree --name-only "$REV" machines/ 2>/dev/null | sed 's#^machines/##'
    else
        for d in "$1"/machines/*/; do
            [ -f "${d}forjar.yaml" ] && basename "$d"
        done
    fi
    return 0
}

semver_re='^[0-9]+\.[0-9]+\.[0-9]+$'

if [ -z "$LATEST" ]; then
    LATEST="$(git ls-remote --tags --refs "$REMOTE" 'v*' 2>/dev/null \
        | sed -n 's#.*refs/tags/v\([0-9]*\.[0-9]*\.[0-9]*\)$#\1#p' \
        | sort -V | tail -n 1)" || LATEST=""
fi
printf '%s\n' "$LATEST" | grep -Eq "$semver_re" || not_measured "latest bashrs tag unknown (got '${LATEST}')"

ROWS=""
add_row() { ROWS="${ROWS}$1	$2
"; }

# --- forjar ---------------------------------------------------------------
toml="$(rd "$FORJAR_DIR" Cargo.toml)" || not_measured "forjar Cargo.toml unreadable in ${FORJAR_DIR}"
req="$(sed -n -E 's/^bashrs[[:space:]]*=[[:space:]]*(\{[^}]*version[[:space:]]*=[[:space:]]*)?"[=^~]?([0-9]+\.[0-9]+\.[0-9]+)".*/\2/p' <<<"$toml" | sed -n 1p)"
[ -n "$req" ] || not_measured "no bashrs requirement in forjar Cargo.toml"
add_row "forjar Cargo.toml (requirement floor)" "$req"

lock="$(rd "$FORJAR_DIR" Cargo.lock)" || not_measured "forjar Cargo.lock unreadable"
resolved="$(awk '/^name = "bashrs"$/{getline; gsub(/^version = "|"$/, ""); print; exit}' <<<"$lock")"
[ -n "$resolved" ] || not_measured "no bashrs package in forjar Cargo.lock"
add_row "forjar Cargo.lock (resolved)" "$resolved"

# --- infra hosts ----------------------------------------------------------
[ -n "$INFRA_DIR" ] || not_measured "no infra checkout (--infra DIR or INFRA_DIR)"
host_rows=0
for m in $(machines "$INFRA_DIR"); do
    y="$(rd "$INFRA_DIR" "machines/$m/forjar.yaml")" || continue
    v="$(sed -n -E 's/^[[:space:]]+bashrs_version:[[:space:]]*"?([0-9]+\.[0-9]+\.[0-9]+)"?.*/\1/p' <<<"$y" | sed -n 1p)"
    if [ -n "$v" ]; then add_row "infra machines/$m params.bashrs_version" "$v"; host_rows=$((host_rows + 1)); fi
    # Here-strings, not `printf | awk ... exit`: an early exit closes the pipe,
    # printf dies of SIGPIPE, and pipefail + set -e kill the whole run silently.
    v="$(awk '
        /^  fleet-bashrs-pin:/ { inb = 1; next }
        inb && /^  [^ #]/      { exit }
        inb && /content:/      { c = 1; next }
        c && NF                { print $1; exit }' <<<"$y")"
    if [ -n "$v" ]; then add_row "infra machines/$m fleet-bashrs-pin content" "$v"; host_rows=$((host_rows + 1)); fi
    installs="$(grep -v '^[[:space:]]*#' <<<"$y" \
        | sed -n -E 's/.*cargo .*install .*bashrs .*--version[[:space:]]+([0-9]+\.[0-9]+\.[0-9]+).*/\1/p')" || installs=""
    while read -r v; do
        [ -n "$v" ] || continue
        add_row "infra machines/$m cargo install --version" "$v"; host_rows=$((host_rows + 1))
    done <<<"$installs"
done
[ "$host_rows" -gt 0 ] || not_measured "no bashrs pin found under ${INFRA_DIR}/machines"

# --- verdict --------------------------------------------------------------
skew=0
rows=0
printf 'source\tpin\tlatest\tstatus\n'
while IFS='	' read -r src pin; do
    [ -n "$src" ] || continue
    rows=$((rows + 1))
    if [ "$pin" = "$LATEST" ]; then st=OK; else st=SKEW; skew=$((skew + 1)); fi
    printf '%s\t%s\t%s\t%s\n' "$src" "$pin" "$LATEST" "$st"
done <<EOF
$ROWS
EOF

echo "bashrs-pin-skew: latest=${LATEST} rows=${rows} skew=${skew}${REV:+ rev=${REV}}"
[ "$skew" -eq 0 ] || exit 1
exit 0
