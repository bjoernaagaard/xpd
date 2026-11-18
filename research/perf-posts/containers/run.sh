#!/usr/bin/env bash
set -eu
cd "$(dirname "$0")"
export PYTHONDONTWRITEBYTECODE=1
studies=(mdp merge heapsort count deque membership cache group)
versions=(3.13.18 3.14.10 3.15.2)
if (( $# > 0 )); then studies=("$1"); fi
if (( $# > 1 )); then versions=("$2"); fi
for study in "${studies[@]}"; do
    for version in "${versions[@]}"; do
        if [[ $study == mdp ]]; then
            uv run --no-project --python "$version" python bench/hash_probe.py > "runs/hash-$version.json"
        fi
        timeout 85s taskset -c 8 uv run --no-project --with pyperf==2.9.0 \
            --python "$version" python bench/measure.py "$study" "$version"
    done
done
