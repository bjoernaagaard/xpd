#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p runs
work=$(mktemp -d "$PWD/.bench.XXXXXX")
trap 'rm -rf "$work"' EXIT
studies=(nine sxz boto docker fleet lemire stats struct posting vex django typing order module touch)
versions=(3.13.18 3.14.10 3.15.2)
if [[ ${1:-all} != all ]]; then studies=("$1"); fi
if [[ $# -gt 1 ]]; then versions=("$2"); fi
for study in "${studies[@]}"; do
  for version in "${versions[@]}"; do
    timeout 85 taskset -c 4 uv run --no-project --python "$version" --with-requirements bench/requirements.txt python bench/measure.py "$study" "$work"
  done
done
