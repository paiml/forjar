#!/usr/bin/env bash
# The crates.io version index, as gate T's T8b exemption reads it (PMAT-1150).
#
# SOURCED by scripts/dogfood/tagged.sh; kept apart so the predecessor selection
# can be exercised against many-version indexes directly, which a fixture whose
# only tag is v0.0.1 cannot do.

# The newest NON-YANKED STABLE version strictly below $1, compared as numbers
# (not as text, not with sort -V), from the index document on stdin. Prints
# nothing when there is none. Pre-releases and yanked versions never count, and
# a version at or above $1 never does either.
cratesio_newest_stable_below() {
  jq -r --arg v "$1" '
    ($v | split(".") | map(tonumber)) as $t
    | [.versions[]
       | select(.yanked == false and (.num | test("^[0-9]+[.][0-9]+[.][0-9]+$")))
       | .num | split(".") | map(tonumber)]
    | map(select(. < $t)) | sort | last // empty | map(tostring) | join(".")'
}
