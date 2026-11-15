# detnet-lab: containerlab testbed (track A0)

Emulated network for measuring Zenoh router tail latency
([track A](../../docs/detnet/track-a-measure.md)). Routers and endpoints are Linux
containers joined by veth links; capacity and delay come from `tc` on those links.

```text
pub  --+                         +-- sub
       r1 ---- r2 ==(tbf)==> r3 -+
xpub --+                         +-- xsub
```

- `r2:eth2` towards `r3` is the bottleneck: `netem` (delay) with a `tbf` child
  (rate, 100 ms buffer).
- Endpoints are Zenoh clients of their edge router, and routers connect to their
  neighbours, so no IP forwarding is needed.
- `zprobe pub` sends `[seq][send_ns][padding]` at a fixed rate on an absolute
  schedule. `zprobe sub` stamps arrival times. All containers share the host clock,
  so the difference is one-way latency.
- Cross traffic is a second `zprobe pub` on `detnet/cross`, sized as a percentage of
  the bottleneck rate. It competes with the probe in r1, on the r1–r2 TCP link, and
  in the bottleneck queue.

## Files

| File | Purpose |
| --- | --- |
| `line3.clab.yml` | Topology and addressing |
| `image/Dockerfile` | Node image: Arch base (matches the host glibc) with `iproute2` and `iperf3` |
| `zprobe/` | Latency probe: standalone crate that depends on this branch's `zenoh` |
| `profiles/*.env` | Router arguments plus probe and cross-traffic QoS for each configuration |
| `lab.sh` | Driver |
| `results/` | Output (gitignored): one directory per run, plus `summary.csv` |

## Run

```sh
# binaries, bind-mounted read-only into every node at /opt/zenoh
export CARGO_TARGET_DIR=$PWD/../../target
cargo +1.97.1 build --release -p zenohd --manifest-path ../../Cargo.toml
cargo +1.97.1 build --release --manifest-path zprobe/Cargo.toml

./lab.sh image                 # docker build detnet-node:latest
./lab.sh deploy                # sudo containerlab deploy
RATE=100mbit DELAY=1ms ./lab.sh shape
./lab.sh run stock 50          # one run: profile, cross load in % of bottleneck
LOADS="0 50 90 100" ./lab.sh sweep stock prio
./lab.sh destroy               # sudo containerlab destroy
```

Environment knobs: `RATE`, `DELAY`, `SIZE` (payload bytes, default 1024),
`PROBE_RATE` (messages per second, default 1000), `SECS` (default 20).

Each line of `summary.csv` has the fields
`profile,load,n,loss_pct,p50_us,p99_us,p999_us,p9999_us,max_us`.

## Profiles

| Profile | Probe | Cross |
| --- | --- | --- |
| `stock` | priority 5, block | priority 5, block |
| `prio` | priority 1, express, drop | priority 5, block |

The A1 deterministic-lane profile will add a dedicated `?prio=` link per hop with
`#dscp=` and a matching `tc` class on the bottleneck.

## Limits

- 4-core host, non-real-time kernel. All seven containers share the cores, so compare
  profiles against each other on the same host rather than reading absolute bounds.
- `deploy` and `destroy` need root. Everything else uses `docker exec`, which only
  needs membership of the `docker` group.
- The running kernel must match the installed modules, otherwise veth creation fails
  with `operation not supported`. Reboot after a kernel upgrade.
