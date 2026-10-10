#!/usr/bin/env bash
# What a RELEASE tag is, for every script that picks one (PMAT-607, forjar#607).
#
# SOURCED, NOT RUN. A release is a tag spelled vX.Y.Z, the one shape
# docs/roadmaps/releases.yaml admits for `tag:` (lib/releases.sh). A
# pre-release (v1.33.0-rc.1) is not one: it has no row and it does not open a
# window. Both orders the scripts use rank it ABOVE its release
# (`git tag --sort=-v:refname` and `sort -V` alike put v1.33.0-rc.1 after
# v1.33.0). So an rc left in the list became the newest "release". Gate T
# then looked for its row, and the window of v1.33.0 came out as the one PR
# merged after the rc.

# The release tags of the newline-separated tag list $1, order kept ->
# DOGFOOD_RELEASE_TAGS. One awk over a captured list, never a pipe (PMAT-239).
dogfood_release_tags() {
  DOGFOOD_RELEASE_TAGS="$(awk '/^v[0-9]+\.[0-9]+\.[0-9]+$/' <<<"$1")"
}
