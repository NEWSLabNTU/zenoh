#!/usr/bin/env bash
# Parameter sweeps for detnet-sim. Writes results/<experiment>.csv (5 seeds each).
# Usage: ./sweep.sh [experiment ...]   (default: all)   JOBS=N for parallel runs.
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release -q
BIN="${CARGO_TARGET_DIR:-target}/release/detnet-sim"
JOBS="${JOBS:-2}"
SEEDS="1 2 3 4 5"
OUT=results
TMP=$OUT/tmp
mkdir -p "$TMP"

# Subscriber mix with exact-stream weight E (percent); wildcard classes keep 2:10:30:15.
mix_exact() { local e=$1 w=$((100 - $1)); echo "$((2*w)),$((10*w)),$((30*w)),$((15*w)),$((57*e))"; }

jobs_for() {
  local exp=$1 base="--policies=exactid,snapshot,wildcard --refresh=100"
  case $exp in
    charging)
      for c in per-pair most-specific first-declared split; do
        echo "--policies=wildcard --shared-face=0.4 --charging=$c"; done ;;
    preempt)
      echo "--policies=snapshot,wildcard --refresh=100"
      echo "--policies=snapshot,wildcard --refresh=100 --preempt" ;;
    refresh)
      for r in 10 50 100 200 1000 0; do echo "--policies=snapshot --refresh=$r"; done
      echo "--policies=exactid,wildcard" ;;
    mix)
      for e in 10 30 50 70 90; do echo "$base --mix=$(mix_exact $e)"; done
      # Breadth: scale the broad classes (ops centre, region monitor).
      for b in 0.25 1 4; do
        echo "$base --mix=$(python3 -c "b=$b; print(','.join(str(x) for x in [2*b,10*b,30,15,43]))")"; done ;;
    topo)
      for r in 20 30 50; do echo "$base --routers=$r --extra-links=$r"; done
      for x in 10 60; do echo "$base --extra-links=$x"; done
      for f in 0 1; do echo "$base --fast-share=$f"; done ;;
    peer)
      for m in star tree; do echo "--policies=snapshot,wildcard --refresh=100 --peer-share=0.4 --peer-mode=$m"; done ;;
    routing)
      for t in "" "--extra-links=10 --fast-share=0" "--pubs=300 --subs=450"; do
        for r in spt weighted capacity; do echo "--policies=wildcard --routing=$r $t"; done; done ;;
    *) echo "unknown experiment $exp" >&2; exit 2 ;;
  esac
}

EXPS=("$@")
[ ${#EXPS[@]} -eq 0 ] && EXPS=(charging preempt refresh mix topo peer routing)
mkdir -p "$OUT"
for exp in "${EXPS[@]}"; do
  i=0
  while read -r args; do
    for s in $SEEDS; do
      i=$((i + 1))
      echo "$BIN --csv --no-header --seed=$s $args > $TMP/$exp-$i.csv"
    done
  done < <(jobs_for "$exp")
done | xargs -P "$JOBS" -I{} sh -c '{}'
for exp in "${EXPS[@]}"; do
  { "$BIN" --csv-header; cat "$TMP/$exp"-*.csv; } > "$OUT/$exp.csv"
  rm -f "$TMP/$exp"-*.csv
  echo "wrote $OUT/$exp.csv ($(($(wc -l < "$OUT/$exp.csv") - 1)) rows)"
done
