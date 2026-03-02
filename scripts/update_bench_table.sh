#!/usr/bin/env bash
# Update the README.md benchmark table from Criterion output.
#
# Usage: ./scripts/update_bench_table.sh
# Or:    make bench-update

set -euo pipefail

README="README.md"
MARKER_START="<!-- BENCH-TABLE-START -->"
MARKER_END="<!-- BENCH-TABLE-END -->"

if ! grep -q "$MARKER_START" "$README"; then
    echo "error: $MARKER_START marker not found in $README" >&2
    exit 1
fi

echo "Running cargo bench (this may take a few minutes)..."
BENCH_OUTPUT=$(cargo bench 2>&1)

# Parse Criterion output lines like:
#   store_path_hash     time:   [1.234 µs 1.250 µs 1.267 µs]
# Extract: name, median (middle value), unit
TABLE="| Operation | Input | Mean | 95% CI |"
TABLE="$TABLE
|---|---|---|---|"

while IFS= read -r line; do
    # Match lines with timing data
    if [[ "$line" =~ ^([a-zA-Z_]+)/([^ ]+)[[:space:]]+time:[[:space:]]+\[([0-9.]+)[[:space:]]+(ns|us|µs|ms|s)[[:space:]]+([0-9.]+)[[:space:]]+(ns|us|µs|ms|s)[[:space:]]+([0-9.]+)[[:space:]]+(ns|us|µs|ms|s)\] ]]; then
        name="${BASH_REMATCH[1]}"
        input="${BASH_REMATCH[2]}"
        median="${BASH_REMATCH[5]}"
        unit="${BASH_REMATCH[6]}"
        low="${BASH_REMATCH[3]}"
        high="${BASH_REMATCH[7]}"
        # Normalize unit display
        [[ "$unit" == "µs" ]] && unit="us"
        ci_low=$(echo "$median - $low" | bc 2>/dev/null || echo "?")
        ci_high=$(echo "$high - $median" | bc 2>/dev/null || echo "?")
        TABLE="$TABLE
| $name | $input | $median $unit | +/- $ci_high $unit |"
    elif [[ "$line" =~ ^([a-zA-Z_]+)[[:space:]]+time:[[:space:]]+\[([0-9.]+)[[:space:]]+(ns|us|µs|ms|s)[[:space:]]+([0-9.]+)[[:space:]]+(ns|us|µs|ms|s)[[:space:]]+([0-9.]+)[[:space:]]+(ns|us|µs|ms|s)\] ]]; then
        name="${BASH_REMATCH[1]}"
        median="${BASH_REMATCH[4]}"
        unit="${BASH_REMATCH[5]}"
        high="${BASH_REMATCH[6]}"
        [[ "$unit" == "µs" ]] && unit="us"
        ci_high=$(echo "$high - $median" | bc 2>/dev/null || echo "?")
        TABLE="$TABLE
| $name | — | $median $unit | +/- $ci_high $unit |"
    fi
done <<< "$BENCH_OUTPUT"

# Replace content between markers
{
    sed -n "1,/$MARKER_START/p" "$README"
    echo ""
    echo "$TABLE"
    echo ""
    sed -n "/$MARKER_END/,\$p" "$README"
} > "${README}.tmp"

mv "${README}.tmp" "$README"
echo "Updated $README benchmark table."
