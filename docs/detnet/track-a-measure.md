# Track A: Measure (claim C4)

**Status:** A0 built (`tools/detnet-lab/`), first runs pending · **Wire change:** none · **Testbed:** containerlab

Measure the tail latency of a Zenoh router carrying a deterministic stream, and find
out whether the tail comes from queueing (which scheduling can fix) or from host and
runtime noise. No tail-latency data for Zenoh exists; every published evaluation
reports mean or median only ([prior-art.md](prior-art.md), C4).

## Steps

### A0 Testbed

- containerlab topologies under `tools/detnet-lab/`: 3 `zenohd` routers in a line or
  triangle (4-core host), publisher and subscriber containers, a cross-traffic
  generator.
- `zenohd` built from this branch; image built locally.
- Link shaping with `tc` on the veth pairs: `tbf` for capacity, `netem` for delay,
  jitter and loss.
- One-way latency from send timestamps in the payload; all containers share the host
  clock, so no PTP is needed.
- Recorder writes per-sample latency to CSV; analysis script reports p50, p99, p99.9,
  p99.99 and max.
- Built: `line3.clab.yml` (r1–r2–r3, bottleneck `r2:eth2` with netem plus tbf),
  `zprobe` probe, `lab.sh` driver with `stock` and `prio` profiles. See
  [detnet-lab](../../tools/detnet-lab/README.md).

### A1 Baseline

Deterministic stream (e.g. 2 Mb/s, 1 kB samples) plus best-effort cross traffic at
0, 50, 80 and 100% of the bottleneck link, for two configurations:

| Configuration | Settings |
| --- | --- |
| Stock | Default `DEFAULT_CONFIG.json5` |
| Deterministic lane | `express` publisher, `congestion_control: drop`, dedicated priority, `?prio=` link with `#dscp=`; per-link `SO_PRIORITY` once PR #2881 lands |

### A2 Tuning

- cpuset pinning per container; tokio worker count; single-threaded data path.
- PREEMPT_RT host kernel if one can be installed.
- Repeat A1; compare tails.

### A3 Software C-SCORE (gated)

Only if gate G1 shows queueing dominates: finish-time ordering for the deterministic
priority in the transmit pipeline, compared with strict priority.

## Gate G1

After A1–A2: is the tail dominated by queueing that a scheduler can fix, rather than
host noise? If not, drop the firm tier and keep the hard and statistical tiers.

## Limits

All routers share 4 cores, so results compare configurations on the same host; they are
not absolute bounds. Absolute characterisation needs dedicated cores, a real-time
kernel or a larger host.
