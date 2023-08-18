#!/usr/bin/env bash
set -e
script_dir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "$script_dir"

source config.sh

for prio in ${priorities[@]}; do
    export PORT=$((port_base+prio))
    config_file="pong.$PORT.json5"
    envsubst < pong.json5.in > "$config_file"
    echo "../target/fast/examples/z_pong_prio -c \"$config_file\" --no-stdin"
done | parallel --lb -j0
