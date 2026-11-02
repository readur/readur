#!/usr/bin/env bash
# Fails when a release tag doesn't match the version baked into the build.
# The UI reports Cargo.toml's version (env!("CARGO_PKG_VERSION")), so a tag
# that disagrees ships an image that lies about what it is.
set -euo pipefail

tag="${1:?usage: check-release-version.sh <tag>}"
want="${tag#v}"

cargo=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)
npm=$(sed -n 's/^  "version": "\(.*\)",/\1/p' frontend/package.json | head -n1)

status=0
if [ "$cargo" != "$want" ]; then
  echo "::error file=Cargo.toml::Cargo.toml version is $cargo but the tag is $tag"
  status=1
fi
if [ "$npm" != "$want" ]; then
  echo "::error file=frontend/package.json::frontend/package.json version is $npm but the tag is $tag"
  status=1
fi
[ "$status" -eq 0 ] && echo "Version $want matches tag $tag"
exit "$status"
