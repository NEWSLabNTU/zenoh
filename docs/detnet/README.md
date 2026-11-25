# Deterministic Networking in Zenoh

Campaign to make Zenoh carry deterministic flows by adopting existing IETF DetNet
and IEEE TSN standards rather than inventing new mechanisms. An admitted flow gets
reserved bandwidth, a bounded end-to-end latency and a loss budget.

Base: upstream `eclipse-zenoh/zenoh` `main` at `b180cc6e2` (1.10.2 + 10 commits).

## Documents

| Document | Content |
| --- | --- |
| [roadmap.md](roadmap.md) | Tracks, gates and order of work (current plan) |
| [design.md](design.md) | Goals, principles, standards map, Zenoh baseline, architecture, risks |
| [track-a-measure.md](track-a-measure.md) | Track A (C4): containerlab testbed, tail latency, gate G1 |
| [track-b-map.md](track-b-map.md) | Track B (C1): flow and receive contracts, key expression → stream mapping, PRs |
| [track-c-admit.md](track-c-admit.md) | Track C (C2): admission model, simulator, plugin, gate G2 |
| [track-d-mobility.md](track-d-mobility.md) | Track D (C3, stretch): edge handover |
| [track-e-extensions.md](track-e-extensions.md) | Track E: protocol extensions behind gates G1, G3, G4 |
| [prior-art.md](prior-art.md) | Literature check: contribution claims C1–C4, dropped claims, sources |
| [admission-model.typ](admission-model.typ) | C2 formal model: key-expression matching, admission constraints, aggregation lemmas, envelope charging, open problems |
| [routing-semantics.md](routing-semantics.md) | Zenoh `main` routing behaviour the admission model depends on, with code references |
| [../../tools/detnet-sim](../../tools/detnet-sim/README.md) | C2 admission simulator (Rust, uses zenoh-keyexpr) and first results |
| [proposal.typ](proposal.typ) | Research proposal and progress report with diagrams |
| [architecture.typ](architecture.typ) | Architecture diagrams and use cases (preemption, multiple subscribers, stateless transit, mobility); build with `typst compile` (typst 0.15) |

## Roadmap

See [roadmap.md](roadmap.md): four tracks — measure (C4), map (C1), admit (C2),
mobility (C3, stretch) — plus protocol extensions behind measurement gates, on a
containerlab testbed. One document per track is listed above.
