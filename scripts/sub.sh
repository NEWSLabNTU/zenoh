#!/usr/bin/env bash
set -e
script_dir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "$script_dir/.."

for prio in $@; do
    port=$((7440+prio))
    echo ./target/fast/examples/z_sub_thr -l "tcp/0.0.0.0:$port"
done | parallel --lb -j0
