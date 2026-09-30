#!/usr/bin/env bash
# Format the given Rust files in place, and ONLY those files.
#
# Used by the lefthook pre-commit hook (`format-rust` in .lefthook.toml) with
# the staged *.rs files as arguments. Deliberately not `cargo fmt --all`: the
# crate is not rustfmt-clean yet, so a repo-wide format on every commit would
# rewrite hundreds of files nobody touched and leave them as unstaged changes.
# Until a one-time repo-wide format commit lands, files are formatted as they
# are touched ("format on touch").
#
# Also deliberately not `rustfmt <file>`: given a path, rustfmt follows every
# out-of-line `mod foo;` declaration and reformats those files too (the option
# that stops it, `skip_children`, is nightly-only). Feeding the file on stdin
# formats exactly that one file, because rustfmt cannot resolve child modules
# without a path.
#
# The edition must match Cargo.toml's `[package] edition`.
set -euo pipefail

EDITION="${RUSTFMT_EDITION:-2021}"
status=0

for f in "$@"; do
  case "$f" in
    *.rs) ;;
    *) continue ;;
  esac
  [ -f "$f" ] || continue
  tmp="$(mktemp)"
  if rustfmt --edition "$EDITION" --emit stdout <"$f" >"$tmp"; then
    if ! cmp -s "$f" "$tmp"; then
      # cat rather than mv, so the file keeps its mode and inode.
      cat "$tmp" >"$f"
    fi
  else
    echo "format-staged-rust: rustfmt could not parse $f; left unchanged" >&2
    status=1
  fi
  rm -f "$tmp"
done

exit "$status"
