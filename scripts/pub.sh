#!/usr/bin/env bash
set -e
script_dir=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "$script_dir/.."

payload="$1"
shift || {
    echo "Usage: $0 PAYLOAD_SIZE PRIORITY..." >&2
    exit 1
}

for prio in $@; do
    port=$((7440+prio))
    echo ./target/fast/examples/z_pub_thr "$payload" -e "tcp/0.0.0.0:$port" -p "$prio"
done | parallel --lb -j0
