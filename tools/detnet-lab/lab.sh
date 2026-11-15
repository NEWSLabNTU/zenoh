#!/usr/bin/env bash
# Driver for the DetNet containerlab testbed (track A, step A0).
# Only `deploy` and `destroy` need root; everything else uses `docker exec`.
set -euo pipefail
cd "$(dirname "$0")"

TOPO=${TOPO:-line3.clab.yml}
LAB=detnet-line3
BIN=/opt/zenoh
RATE=${RATE:-100mbit}          # bottleneck rate r2 -> r3
DELAY=${DELAY:-0ms}            # extra one-way delay on the bottleneck
SIZE=${SIZE:-1024}             # payload bytes, probe and cross
PROBE_RATE=${PROBE_RATE:-1000} # probe msg/s (1000 x 1 KiB = 8.2 Mb/s)
SECS=${SECS:-20}

n() { echo "clab-$LAB-$1"; }
x() { local node=$1; shift; docker exec "$(n "$node")" "$@"; }
xd() { local node=$1; shift; docker exec -d "$(n "$node")" "$@"; }

rate_bps() { # 100mbit -> 100000000
  local v=${1%bit}; case $v in
    *g) echo $(( ${v%g} * 1000000000 ));; *m) echo $(( ${v%m} * 1000000 ));;
    *k) echo $(( ${v%k} * 1000 ));; *) echo "$v";; esac
}

cmd_image() { docker build -t detnet-node:latest image; }
cmd_deploy() { mkdir -p results; sudo containerlab deploy -t "$TOPO" --reconfigure; }
cmd_destroy() { sudo containerlab destroy -t "$TOPO" --cleanup; }

cmd_shape() {
  # netem (delay) as root, tbf (rate, 100 ms of buffer) as its child.
  x r2 tc qdisc del dev eth2 root 2>/dev/null || true
  x r2 tc qdisc add dev eth2 root handle 1: netem delay "$DELAY" limit 100000
  x r2 tc qdisc add dev eth2 parent 1: handle 2: tbf rate "$RATE" burst 64kb latency 100ms
  x r2 tc qdisc show dev eth2
}

cmd_stop() {
  for r in r1 r2 r3 pub xpub sub xsub; do
    x "$r" pkill -x zenohd 2>/dev/null || true
    x "$r" pkill -x zprobe 2>/dev/null || true
  done
}

cmd_routers() { # $1 = profile
  # shellcheck disable=SC1090
  source "profiles/$1.env"
  cmd_stop
  local common="--no-multicast-scouting -l tcp/0.0.0.0:7447 $ROUTER_ARGS"
  xd r2 sh -c "$BIN/zenohd $common >/tmp/zenohd.log 2>&1"
  sleep 1
  xd r1 sh -c "$BIN/zenohd $common -e tcp/10.12.0.2:7447 >/tmp/zenohd.log 2>&1"
  xd r3 sh -c "$BIN/zenohd $common -e tcp/10.23.0.2:7447 >/tmp/zenohd.log 2>&1"
  sleep 2
}

cmd_run() { # $1 = profile, $2 = cross load in % of bottleneck
  local profile=$1 load=$2
  # shellcheck disable=SC1090
  source "profiles/$profile.env"
  [[ -f results/summary.csv ]] || echo "profile,load,n,loss_pct,p50_us,p99_us,p999_us,p9999_us,max_us" > results/summary.csv
  local id; id="$(date +%Y%m%d-%H%M%S)-$profile-L$load"
  mkdir -p "results/$id"
  cmd_routers "$profile"
  local cross_rate=$(( $(rate_bps "$RATE") * load / 100 / (SIZE * 8) ))
  local sub_secs=$(( SECS + 6 ))
  xd sub sh -c "$BIN/zprobe -e tcp/10.3.1.1:7447 sub -k detnet/probe -d $sub_secs \
      -o /results/$id/probe.csv --label probe >/results/$id/probe.txt 2>&1"
  xd xsub sh -c "$BIN/zprobe -e tcp/10.3.2.1:7447 sub -k detnet/cross -d $sub_secs \
      --label cross >/results/$id/cross.txt 2>&1"
  sleep 1
  if (( load > 0 )); then
    xd xpub sh -c "$BIN/zprobe -e tcp/10.1.2.1:7447 pub -k detnet/cross -r $cross_rate \
        -s $SIZE -d $SECS $CROSS_ARGS >/results/$id/xpub.txt 2>&1"
  fi
  x pub "$BIN/zprobe" -e tcp/10.1.1.1:7447 pub -k detnet/probe -r "$PROBE_RATE" \
      -s "$SIZE" -d "$SECS" $PROBE_ARGS 2>"results/$id/pub.txt" || true
  sleep 5
  printf 'profile=%s load=%s rate=%s delay=%s cross_rate=%s\n' \
      "$profile" "$load" "$RATE" "$DELAY" "$cross_rate" > "results/$id/params.txt"
  local line; line=$(cat "results/$id/probe.txt")
  echo "$id $line"
  # profile,load,n,loss,p50,p99,p999,p9999,max
  echo "$line" | sed -E "s/^probe: n=([0-9]+) loss=([0-9.]+)% p50=([0-9]+)us p99=([0-9]+)us p99.9=([0-9]+)us p99.99=([0-9]+)us max=([0-9]+)us.*/$profile,$load,\1,\2,\3,\4,\5,\6,\7/" \
      >> results/summary.csv
}

cmd_sweep() { # profiles... ; loads from LOADS
  for p in "$@"; do for l in ${LOADS:-0 25 50 75 90 100 120}; do cmd_run "$p" "$l"; done; done
}

case ${1:-} in
  image|deploy|destroy|shape|stop) "cmd_$1";;
  routers|run|sweep) c=$1; shift; "cmd_$c" "$@";;
  *) echo "usage: $0 image|deploy|shape|routers <p>|run <p> <load%>|sweep <p>...|stop|destroy"; exit 1;;
esac
