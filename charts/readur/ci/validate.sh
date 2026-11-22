#!/bin/sh
# Render the chart with every ci/*-values.yaml file and validate the manifests
# against the Kubernetes API schemas, then run the helm-unittest suites.
#
#   charts/readur/ci/validate.sh            # from anywhere in the repo
#
# Needs helm (with the helm-unittest plugin) and kubeconform on PATH; set
# SKIP_KUBECONFORM=1 to skip schema validation (it downloads schemas), and
# SKIP_UNITTEST=1 to skip the unit tests. Run `helm dependency build` first.
set -eu

chart_dir=$(cd "$(dirname "$0")/.." && pwd)
k8s_version=${KUBERNETES_VERSION:-1.31.0}
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

helm lint "$chart_dir"

for values in "$chart_dir"/ci/*-values.yaml; do
  name=$(basename "$values" -values.yaml)
  echo "==> rendering with ci/$(basename "$values")"
  helm lint "$chart_dir" -f "$values" >/dev/null
  helm template readur "$chart_dir" -f "$values" >"$out/$name.yaml"
  if ! grep -q 'image: ghcr.io/readur/readur:' "$out/$name.yaml"; then
    echo "error: ci/$(basename "$values"): readur image is not ghcr.io/readur/readur" >&2
    exit 1
  fi
  if [ "${SKIP_KUBECONFORM:-0}" != 1 ]; then
    kubeconform -strict -summary -kubernetes-version "$k8s_version" "$out/$name.yaml"
  fi
done

# Every env var the chart sets, or shows as a commented example in values.yaml,
# must be one the server reads: a renamed or removed variable fails here
# instead of being silently ignored by Readur.
src_dir="$chart_dir/../../src"
if [ -d "$src_dir" ]; then
  echo "==> checking env var names against src/"
  names=$(
    {
      cat "$out"/*.yaml | sed -n 's/^ *- name: \([A-Z][A-Z0-9_]*\)$/\1/p'
      sed -n 's/^ *# *\([A-Z][A-Z0-9_]*\): .*/\1/p' "$chart_dir/values.yaml"
    } | sort -u
  )
  missing=""
  for name in $names; do
    grep -rqF "\"$name\"" "$src_dir" || missing="$missing $name"
  done
  if [ -n "$missing" ]; then
    echo "error: the chart sets env vars the server never reads:$missing" >&2
    exit 1
  fi
  echo "$(echo "$names" | wc -w) env vars, all read by the server"
fi

if [ "${SKIP_UNITTEST:-0}" != 1 ]; then
  helm unittest "$chart_dir"
fi
