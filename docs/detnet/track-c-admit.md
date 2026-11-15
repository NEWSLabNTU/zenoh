# Track C: Admit (claim C2)

**Status:** model and simulator extensions (C2) done; gates G2 and G3 evaluated · **Wire change:** none

Admission control when the receivers of a stream are defined by intersecting wildcard
key expressions, where any new publisher or subscriber declaration can create many
candidate pairs at once. Prior art (RSVP, MSRP/RAP, IndiQoS, SOME/IP-over-TSN) admits
receivers of exact identifiers only ([prior-art.md](prior-art.md), C2).

## Done

- **C1 model** — [admission-model.typ](admission-model.typ): key-expression semantics,
  constraints (capacity once per tree edge, per-path deadline, receive budget), event
  complexity (new publisher: polynomial; new subscriber and preemption: NP-hard),
  aggregation lemmas, envelope charging problem. Assumptions checked against the code
  in [routing-semantics.md](routing-semantics.md).
- **Simulator** — [detnet-sim](../../tools/detnet-sim/README.md): one router region,
  churning declarations; first results over 5 seeds:

| Policy | Admitted, % of eligible | Violated |
| --- | --- | --- |
| No admission | 100% | 90.8% of admitted |
| Exact identifiers (prior art) | 2.6% | 0 |
| Wildcard (C2) | 69.9% | 0 |

- **C2 simulator extensions** — charging rules, preemption, sensitivity sweeps,
  peer-region star delivery, capacity-aware routing, and a stronger baseline
  (`snapshot`: wildcard subscriptions expanded into exact reservations at declaration
  and on every refresh). Full tables in the
  [simulator README](../../tools/detnet-sim/README.md); CSVs in
  `tools/detnet-sim/results/`. 5 seeds each:

| Finding | Numbers |
| --- | --- |
| Wildcard vs exact identifiers | 59–76% vs 0.4–24% admitted across all mixes and topologies |
| Wildcard vs snapshot expansion | Refresh every 100 events: 1.0–1.6 points behind; every 1000: 16 behind; never: 34 behind |
| Charging rules (40% shared faces) | Most-specific, first-declared or split admit at most 1 point more than per-pair and use 3 points less budget, but evict about 500 pairs per run when the payer leaves; per-pair never evicts |
| Preemption | Priority 2 admitted 91 → 96%, priority 1 35 → 33%; about 0.4 evictions per event, 80% re-admitted later |
| Peer regions (star vs relaying) | Peer subscribers admitted 20% vs 32%: one copy per peer face costs 40% of peer admissions |
| Routing | Capacity-aware branches +2 to +5 points over hop-count trees, 4× event cost; static inverse-capacity weights 0 to −3 points |

## Gate verdicts

- **G2: passes against exact identifiers, narrowed against expansion.** Wildcard
  admission keeps a large advantage over exact-identifier admission in every mix and
  topology tried. Against an application that expands its wildcards into exact
  reservations and refreshes them often, the gain is only 1–2 points; it grows to
  16–34 points when the refresh period approaches the publisher lifetime (about 800
  events). C2 is therefore claimed as *exact, immediate expansion inside the network*
  (no refresh, no application logic, correct under declaration aggregation), not as
  admitting more than an ideal expansion. The simulator does not yet measure what
  that buys in signalling or reaction time; the C4 plugin should.
- **G3: not triggered.** Capacity-aware routing admits 2–5 points more than Zenoh's
  shortest-path trees (most in the heavy-load case) at 4× the event cost; setting
  `transport_weights` inversely to capacity does not help. This is not a clear enough
  gain to justify explicit paths or BIER-TE (E1/E2). Caveat: admission is greedy and
  never re-optimised, so this is a lower bound; revisit if a re-optimising variant
  or a real topology shows more.
- **Side result for track B / E:** star delivery into peer regions is the largest
  single loss found. Relaying or multicast inside a peer region is worth more than
  capacity-aware routing.

## Next

### C2 follow-ups

- Clients behind aggregate interests; multi-region gateways.
- Signalling and reaction-time cost of `snapshot` refresh vs wildcard re-evaluation
  (the quantity G2 now rests on).
- Incremental retry instead of rescanning all pending pairs.
- A re-optimising capacity-aware variant, to bound the G3 gain from above.

### C3 Proofs

Lemmas 1–2 (OR-bit for bandwidth, min-slack for deadlines) for Zenoh's actual
declaration propagation, and the weakest scheduling assumption under which they hold.

### C4 Admission plugin

- zenohd plugin with exact router-level state driven by declarations and the contracts
  of track B.
- Policing in an interceptor at the ingress router.
- Validated in the track A testbed against the simulator's predictions.
