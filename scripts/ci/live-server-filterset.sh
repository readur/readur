#!/usr/bin/env bash
# Prints the nextest filterset selecting the integration test binaries listed in
# .config/live-server-suites (the suites that need a running readur server or the
# WebDAV server). CI runs `-E "$(this)"` in the live-server job and
# `-E "not ($(this))"` in the in-process shards.
set -euo pipefail

list="$(cd "$(dirname "$0")/../.." && pwd)/.config/live-server-suites"
filterset="$(grep -Ev '^[[:space:]]*(#|$)' "$list" | awk '{ printf "%s%s", sep, "binary(=" $1 ")"; sep = " | " }')"
if [ -z "$filterset" ]; then
  echo "no suites listed in $list" >&2
  exit 1
fi
printf '%s\n' "$filterset"
