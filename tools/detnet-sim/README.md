# detnet-sim

Simulator for claim C2 of the DetNet campaign: admission control when the receivers of
a stream are defined by intersecting wildcard key expressions. Model:
[`docs/detnet/admission-model.typ`](../../docs/detnet/admission-model.typ); routing
assumptions: [`docs/detnet/routing-semantics.md`](../../docs/detnet/routing-semantics.md).

Matching uses Zenoh's own `zenoh-keyexpr` crate (`intersects`).

## Run

```sh
cd tools/detnet-sim
cargo run --release -- --seed=1            # human-readable summary
cargo run --release -- --seed=1 --csv      # one CSV row per policy
cargo run --release -- --help              # all options
./sweep.sh [experiment ...]                # 5-seed sweeps into results/*.csv
./summarize.py results/charging.csv --by charging \
    --metrics admitted_frac,budget_util,forced --pct admitted_frac,budget_util
```

The crate is standalone (own `[workspace]`, stable toolchain). `sweep.sh` runs
`JOBS=2` simulations in parallel by default; all seven experiments take about 35 CPU
minutes.

Source layout: `params.rs` (options), `topology.rs` (graph, static trees),
`workload.rs` (declarations and trace), `state.rs` (reservations, charging, routing,
preemption), `main.rs` (sampling and output).

## What it simulates

- **Network:** one Zenoh router region; random ring-plus-chords graph (default 30
  routers, 58 links), links 100 Mb/s or 1 Gb/s (`--fast-share`, default 30% at
  1 Gb/s) with 0.5–5 ms propagation.
- **Publishers:** `fleet/r<R>/v<V>/base` (2 Mb/s, 50 ms deadline, priority 2),
  `fleet/r<R>/v<V>/enh` (6 Mb/s, 100 ms, priority 1), and 5% wildcard publishers
  `fleet/r<R>/v<V>/*` (8 Mb/s, priority 1).
- **Subscribers** (deterministic selector, budget), weights set by `--mix`
  (default `2,10,30,15,43`): operations centre `fleet/*/*/base` 400 Mb/s; region
  monitor `fleet/r<R>/**` 100 Mb/s; nearby peer `fleet/r<R>/v<V>/**` 10 Mb/s;
  cross-region vehicle `fleet/*/v<V>/base` 5 Mb/s; exact stream
  `fleet/r<R>/v<V>/base` 5 Mb/s.
- **Faces:** with `--shared-face=F`, a share F of subscriptions is declared on an
  existing session face about the same vehicle (e.g. `fleet/r1/v3/**` next to
  `fleet/r1/v3/base`). The router delivers one copy per publisher per face.
- **Peer regions:** with `--peer-share=F`, a share F of new faces are peers in a peer
  region behind one of the first `--peer-routers` routers (default 5), sharing a
  `--peer-cap` access segment (default 100 Mb/s).
- **Trace:** declarations and undeclarations churning around the target populations
  (default 200 publishers, 300 subscribers), 20 000 events; metrics sampled after the
  first quarter. Mean lifetime is about 800 events for a publisher and 1 200 for a
  subscriber.
- **Per-hop delay bound:** `L/r_p + L/c_e + prop` (rate-based, C-SCORE-like),
  L = 1500 bytes; one extra hop for peer access.

## Policies and options

| Policy | Rule |
| --- | --- |
| `none` | No admission: every eligible pair is delivered; pairs whose link capacity, deadline or receive budget is exceeded count as violated |
| `exactid` | Prior art (RSVP/MSRP): only pairs whose selector is a concrete key equal to the publisher's key are admitted |
| `snapshot` | Prior art plus application-side expansion: a wildcard subscription is expanded into exact reservations for the publishers present at declaration and at every `--refresh=N` events (RSVP-like refresh); 0 = never |
| `wildcard` | C2 model: any pair with `k_p intersects d_s` is a candidate on every declaration event, admitted greedily (priority, then rate) against capacity × (1 − guard), deadline and budget; rejected pairs are retried after releases |

| Option | Values | Meaning |
| --- | --- | --- |
| `--charging` | `per-pair` (default), `most-specific`, `first-declared`, `split` | Who pays when one copy reaches a face with several matching subscriptions; `per-pair` charges every subscription the full rate |
| `--preempt` | flag | On a declaration, a priority-2 pair that does not fit evicts lower-priority pairs (RFC 4829 order: lowest priority, then largest rate) from the blocking link, peer segment or budget; evicted pairs go to pending; rolled back if eviction cannot make room |
| `--routing` | `spt` (default), `weighted`, `capacity` | `spt`: per-source hop-count trees (Zenoh default); `weighted`: per-source trees on static weights 1 Gb/s ÷ capacity (`transport_weights`); `capacity`: per-pair branch grafted onto the publisher's tree by Dijkstra over links with residual capacity (load-weighted cost, then minimum delay), keeping one incoming link per node (explicit paths or BIER-TE, E1/E2) |
| `--peer-mode` | `star` (default), `tree` | `star`: one copy per subscribing peer face on the access segment (Zenoh peers do not relay); `tree`: one copy per publisher per peer region (relaying or multicast) |

## Results (5 seeds; mean ± sd)

All CSVs are in `results/` (`none.csv` was run by hand: `--policies=none`, seeds 1–5). "Admitted" is the share of eligible pairs (M^det)
admitted; no admission policy ever violates a guarantee (checked every sample).

### Baseline and G2: wildcard vs exact identifiers and expansion

| Policy | Admitted | Priority 2 admitted | Event cost mean |
| --- | --- | --- | --- |
| none | 100% (90.8 ± 1.2% of them violated) | 100% | 16 µs |
| exactid | 2.6 ± 0.1% | 4.1% | 7 µs |
| snapshot, never refreshed | 35.9 ± 0.8% | 41.7% | 45 µs |
| snapshot, refresh 1000 | 53.8 ± 0.5% | 64.1% | 90 µs |
| snapshot, refresh 200 | 66.0 ± 0.7% | 83.5% | 159 µs |
| snapshot, refresh 100 | 68.6 ± 0.7% | 88.1% | 174 µs |
| snapshot, refresh 10 | 69.9 ± 0.7% | 91.1% | 195 µs |
| wildcard | 70.0 ± 0.7% | 91.3% | 192 µs |

Snapshot event cost excludes the refresh scans themselves.

Across subscriber mixes (exact-stream share 10–90%, broad classes ×0.25–×4) and
topologies (20–50 routers, 10–60 chords, 0–100% fast links), wildcard admits
59–76% and exact identifiers 0.4–24%. Snapshot with refresh every 100 events stays
1.0–1.6 points below wildcard in every case (`results/mix.csv`, `results/topo.csv`).

### Envelope charging (`--shared-face=0.4`, wildcard)

| Charging | Admitted | Mean budget use | Forced evictions per run |
| --- | --- | --- | --- |
| per-pair | 70.3 ± 1.2% | 35.5% | 0 |
| most-specific | 70.8 ± 1.3% | 32.6% | 502 ± 119 |
| first-declared | 70.8 ± 1.3% | 32.0% | 453 ± 103 |
| split | 71.3 ± 1.4% | 32.1% | 511 ± 94 |

Sharing the charge saves about 3 points of budget and admits up to 1 point more, but
when a payer leaves, its charge moves to a subscription that may not afford it: about
500 admitted pairs per 20 000 events are then evicted. Per-pair charging
over-reserves and never evicts.

### Preemption

| Policy | Preempt | Admitted | Priority 2 | Priority 1 | Pairs preempted | Re-admitted later |
| --- | --- | --- | --- | --- | --- | --- |
| wildcard | no | 70.0 ± 0.7% | 91.3% | 35.2% | 0 | – |
| wildcard | yes | 71.7 ± 0.7% | 95.5% | 32.8% | 8 261 ± 568 | 80.2% |
| snapshot (100) | no | 68.6 ± 0.7% | 88.1% | 36.8% | 0 | – |
| snapshot (100) | yes | 69.7 ± 0.8% | 90.4% | 35.9% | 1 552 ± 331 | 57.2% |

Preemption raises priority 2 to 95% at a cost of 2.4 points for priority 1, but evicts
about 0.4 pairs per event.

### Peer regions (`--peer-share=0.4`)

| Delivery into peer region | Admitted (all) | Admitted (peer subscribers) |
| --- | --- | --- |
| star (Zenoh today) | 50.9 ± 4.0% | 19.7 ± 1.3% |
| tree (relaying) | 56.0 ± 3.6% | 32.4 ± 1.5% |

One copy per peer face cuts admitted peer pairs by 40% compared with relaying.

### G3: routing

| Scenario | spt | weighted | capacity | Hops spt / capacity | Event cost spt / capacity |
| --- | --- | --- | --- | --- | --- |
| default | 70.0 ± 0.7% | 66.7 ± 3.5% | 73.5 ± 1.3% | 2.45 / 3.18 | 191 / 807 µs |
| sparse (10 chords, all 100 Mb/s) | 54.7 ± 3.6% | 54.6 ± 3.6% | 56.9 ± 3.9% | 2.92 / 3.39 | 332 / 2 710 µs |
| heavy (300 pubs, 450 subs) | 57.6 ± 2.4% | 56.4 ± 2.1% | 62.8 ± 1.5% | 2.39 / 3.19 | 619 / 3 026 µs |

Capacity-aware branches admit 2–5 points more (4–9% more pairs), mostly priority 2
(91 → 96% at default). Static inverse-capacity weights are worse than hop count,
because they pull traffic onto the few fast links.

## Limitations

- One router region; no clients behind aggregate interests, no multi-region gateways.
- Greedy, incremental admission; no re-optimisation of admitted pairs, so the
  capacity-aware gain is a lower bound for what explicit paths could achieve.
- Snapshot's refresh period is in events, not seconds; how many events a real
  refresh interval spans depends on the churn rate.
- No signalling cost: snapshot's re-reservation messages and wildcard's per-event
  re-evaluation are not counted.
- Peer publishers and peer-to-peer delivery inside a peer region are not modelled.
- Preemption runs only on declaration events, not on retries.
- `retry()` rescans every pending pair after each release; event costs are upper
  bounds for this naive implementation and roughly double the first version's
  (extra bookkeeping for faces, trees and peer segments).
