#!/usr/bin/env bash
#
# Fails when the committed frontend API types (frontend/src/types/generated)
# are out of date with the Rust models.
#
# Regenerates the types with scripts/generate-ts-bindings.sh, then checks git
# for modified, deleted or new files in the generated directory. Honours
# READUR_USE_NIX=1 in the same way as the generator.
#
# Usage: scripts/check-ts-bindings.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GEN_DIR="frontend/src/types/generated"

cd "$ROOT"

"$ROOT/scripts/generate-ts-bindings.sh"

drift=0

if ! git diff --exit-code --stat -- "$GEN_DIR"; then
    drift=1
fi

untracked="$(git ls-files --others --exclude-standard -- "$GEN_DIR")"
if [ -n "$untracked" ]; then
    echo "New generated files that are not committed:"
    echo "$untracked"
    drift=1
fi

if [ "$drift" -ne 0 ]; then
    echo >&2
    echo "The generated API types are out of date with the Rust models." >&2
    echo "Run scripts/generate-ts-bindings.sh and commit $GEN_DIR." >&2
    exit 1
fi

echo "Generated API types are up to date."
