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
```

Options: `--routers`, `--extra-links`, `--events`, `--regions`, `--vehicles`,
`--pubs` and `--subs` (target live populations), `--guard` (capacity fraction held
back), `--seed`, `--csv`. The crate is standalone (own `[workspace]`, stable toolchain).

## What it simulates

- **Network:** one Zenoh router region; random ring-plus-chords graph (default 30
  routers, 58 links), links 100 Mb/s or 1 Gb/s with 0.5–5 ms propagation; data follows
  each source router's shortest-path tree by hop count (Zenoh's default equal weights).
- **Publishers:** `fleet/r<R>/v<V>/base` (2 Mb/s, 50 ms deadline, priority 2),
  `fleet/r<R>/v<V>/enh` (6 Mb/s, 100 ms, priority 1), and 5% wildcard publishers
  `fleet/r<R>/v<V>/*` (8 Mb/s).
- **Subscribers** (deterministic selector, budget): operations centre
  `fleet/*/*/base` 400 Mb/s (2%); region monitor `fleet/r<R>/**` 100 Mb/s (10%); nearby
  peer `fleet/r<R>/v<V>/**` 10 Mb/s (30%); cross-region vehicle `fleet/*/v<V>/base`
  5 Mb/s (15%); exact stream `fleet/r<R>/v<V>/base` 5 Mb/s (43%).
- **Trace:** declarations and undeclarations churning around the target populations
  (default 200 publishers, 300 subscribers), 20 000 events; metrics sampled after the
  first quarter.
- **Per-hop delay bound:** `L/r_p + L/c_e + prop` (rate-based, C-SCORE-like),
  L = 1500 bytes.

## Policies

| Policy | Rule |
| --- | --- |
| `none` | No admission: every eligible pair is delivered; pairs whose link capacity, deadline or receive budget is exceeded count as violated |
| `exactid` | Prior-art style (RSVP/MSRP): only pairs whose selector is a concrete key equal to the publisher's key are admitted |
| `wildcard` | C2 model: any pair with `k_p intersects d_s` is a candidate, admitted greedily per declaration event (priority, then rate) against capacity × (1 − guard), deadline and budget; rejected pairs are retried after releases |

## First results (5 seeds, defaults)

| Policy | Admitted, % of eligible pairs | Admitted but violated | Max link utilisation | Event cost mean / max |
| --- | --- | --- | --- | --- |
| none | 100 | 90.8 ± 1.2 % | 3.34 | 13 / 182 µs |
| exactid | 2.6 ± 0.1 % | 0 | 0.10 | 6 / 51 µs |
| wildcard | 69.9 ± 0.7 % | 0 | 0.80 | 80 / 1127 µs |

About 2 300 pairs are eligible at a time, and a publisher declaration makes on average
11 subscribers (p95 19, max 26) eligible at once.

Reading:

- Without admission, the demand overloads links about 3× and breaks 91% of
  guarantees.
- Exact-identifier admission is safe but serves under 3% of the demand, because about
  97% of eligible pairs exist only through wildcard matching.
- Wildcard admission serves about 70% safely, filling links up to the guard limit
  (0.8).

These numbers come from a synthetic subscriber mix and show feasibility and scale, not
expected deployment performance.

## Limitations and next steps

- One router region; no peers, clients, regions or session aggregation.
- No preemption, no envelope reservations or charging rules yet.
- `retry()` rescans every pending pair after each release; the event cost is an upper
  bound for this naive implementation.
- Hop-count trees; `transport_weights` and capacity-aware routing not modelled.
- Next: envelope charging rules (most specific, first declared, split), preemption by
  priority, sensitivity to the subscriber mix, and a peer-region star model.
