# Roadmap

Revised 2026-11-15 after the literature check ([prior-art.md](prior-art.md)), the
routing code check ([routing-semantics.md](routing-semantics.md)) and the first
simulator results ([detnet-sim](../../tools/detnet-sim/README.md)). Supersedes the
five-phase plan; each track has its own document (track-a … track-e).

## Constraints

- **Free research, no deadline.** Order work by risk and value, not by a venue.
- **Testbed is [containerlab](https://containerlab.dev/).** Routers run as containers
  on veth links; capacity, delay, jitter and loss come from `tc` (netem, tbf) on those
  links. Handover is emulated by changing or cutting links at run time.
- **Host:** 4 cores, 39 GiB RAM, stock (non-real-time) kernel. Every emulated router
  shares these cores, so latency results are *relative* (policy A vs policy B on the
  same host), not absolute bounds. Absolute tail latency needs pinned dedicated cores,
  a PREEMPT_RT kernel, or a larger host.
- **No contact with ZettaScale.** Work out of tree first (zenohd plugin, interceptors,
  config); upstream only small, generally useful changes as pull requests.

## Tracks

| Track | Claim | Steps | Output |
| --- | --- | --- | --- |
| [A. Measure](track-a-measure.md) | C4 | A0 containerlab testbed; A1 baseline tail latency; A2 tuning; A3 software C-SCORE (gated) | Tail-latency report |
| [B. Map](track-b-map.md) | C1 | B1 contracts; B2 key expression → link → DSCP; B3 TSN emulation; B4 upstream PRs | Working mapping, PRs |
| [C. Admit](track-c-admit.md) | C2 | C1 model + simulator (first version done); C2 simulator extensions; C3 proofs; C4 admission plugin | Main paper |
| [D. Mobility](track-d-mobility.md) | C3 | Edge handover in a router region (stretch) | Extension of C, or dropped |
| [E. Extensions](track-e-extensions.md) | — | Explicit paths, BIER-TE, scheduling tags, PREOF — each behind a gate | Only if a gate triggers |

### A. Measure (C4)

- **A0 testbed.** containerlab topology files under `tools/detnet-lab/`: 3–5 `zenohd`
  routers built from this branch, publisher and subscriber containers, `tc` link
  shaping, cross-traffic generator. One-way latency from timestamps in payloads (all
  containers share the host clock, so no PTP is needed).
- **A1 baseline.** p50/p99/p99.9/max latency of a deterministic stream under 0–100%
  cross traffic: stock config vs a deterministic-lane profile (`express`, drop instead
  of block, dedicated `?prio=` link, DSCP; per-link `SO_PRIORITY` once upstream PR
  #2881 lands).
- **A2 tuning.** cpuset pinning per container, tokio worker settings, single-threaded
  data path; PREEMPT_RT host kernel if available. With 4 cores, keep the A topology at
  3 routers plus endpoints.
- **A3 software C-SCORE** in the transmit pipeline — only if gate G1 says queueing,
  not scheduling noise, dominates the tail.

### B. Map (C1)

- **B1 contracts.** Rust types for RFC 9016 flow contracts and a *receive contract*
  carrying the deterministic selector separately from the (possibly aggregated)
  subscription; announced via liveliness tokens and admin space — no wire change.
- **B2 mapping.** Deterministic key expression → dedicated link or session → DSCP or
  distinct 5-tuple, following DDS-TSN §8.3.1.
- **B3 TSN emulation.** Linux `mqprio`/`taprio` (software mode) on containerlab veth
  links, to show the mapping end to end without TSN hardware; check veth multi-queue
  support first.
- **B4 upstream PRs.** Candidates: link statistics in the admin space, per-link
  `SO_PRIORITY` (track #2881), hooks the out-of-tree plugin needs. Small and generic.

### C. Admit (C2)

- **C2 simulator extensions.** Envelope charging rules (most specific, first declared,
  split); preemption by priority; sensitivity to subscriber mix and topology;
  peer-region star delivery; capacity-aware routing vs shortest-path trees (feeds G3).
- **C3 proofs.** Lemmas 1–2 of [admission-model.typ](admission-model.typ) for Zenoh's
  actual declaration propagation.
- **C4 admission plugin.** zenohd plugin with exact router-level state driven by
  declarations, policing via an interceptor; validated in the A testbed against the
  simulator's predictions.

### D. Mobility (C3, stretch)

Upstream `main` has no link-state peer mode, so mobile *transit* nodes arise only if
peer relaying returns. Scope D to edge handover inside a router region (peers and
clients as leaves), emulated by moving a container's link between edge routers.

### E. Gated extensions

| Extension | Gate |
| --- | --- |
| E1 explicit paths, E2 BIER-TE trees | G3: capacity-aware routing in the simulator admits clearly more than shortest-path trees plus `transport_weights` |
| E3 per-message scheduling tags | G1: firm tier viable and queueing dominates |
| E4 replication and elimination | G4: loss budgets missed under emulated wireless loss |

## Gates

| Gate | After | Question | If no |
| --- | --- | --- | --- |
| G1 | A1–A2 | Is the tail dominated by queueing that scheduling can fix, rather than host noise? | Drop the firm tier; keep hard (owned TSN) and statistical tiers; focus on bandwidth admission |
| G2 | C2 | Does wildcard admission keep its advantage over exact identifiers across mixes and topologies? | Narrow C2 to the mixes where it holds |
| G3 | C2 | Does capacity-aware routing beat shortest-path trees enough to justify explicit paths? | Skip E1/E2 |
| G4 | A, D | Are loss budgets missed on lossy emulated links? | Skip E4 |

## Order

1. **Now, in parallel:** A0 (containerlab testbed) and C2 (simulator extensions).
2. **Next:** A1 baseline → G1; B1 contracts.
3. **Then:** C4 admission plugin in the testbed, with B2.
4. **Later:** A2/A3 and B3 as gates allow; D; B4 PRs as pieces stabilise.

## Mapping from the old phases (removed)

| Old phase | New place |
| --- | --- |
| 1 Flow contract and admission | A0–A1, B1, C4 |
| 2 Topology and underlay mapping | B2, B3 (capacity bands only if C2 needs them) |
| 3 Explicit paths | E1/E2 behind G3 |
| 4 Stateless core scheduling | A3/E3 behind G1 |
| 5 Replication and elimination | E4 behind G4 |
