#!/usr/bin/env bash
set -e
script_dir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "$script_dir"

source config.sh

# payload="$1"
# shift || {
#     echo "Usage: $0 PAYLOAD_SIZE PRIORITY..." >&2
#     exit 1
# }

for prio in ${priorities[@]}; do
    export PORT=$((port_base+prio))
    config_file="pub.$PORT.json5"
    envsubst < pub.json5.in > "$config_file"
    echo ../target/fast/examples/z_pub_prio "$payload" -c "$config_file" -p "$prio"
done | parallel --lb -j0
