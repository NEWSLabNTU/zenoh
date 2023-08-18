#!/usr/bin/env bash
set -e
script_dir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "$script_dir"

source config.sh

dt="$(date '+%Y-%m-%d-%H-%M-%S')"
out_dir="result/$dt"

mkdir -p result
mkdir "$out_dir"

for prio in ${priorities[@]}; do
    export PORT=$((port_base+prio))
    config_file="sub.$PORT.json5"
    envsubst < sub.json5.in > "$config_file"
    echo "../target/fast/examples/z_sub_prio -c \"$config_file\" --no-stdin | tee \"${out_dir}/prio-${prio}.payload-${payload}.txt\""
done | parallel --lb -j0
