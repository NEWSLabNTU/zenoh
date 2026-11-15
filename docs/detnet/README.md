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
| [prior-art.md](prior-art.md) | Literature check: contribution claims C1–C4, dropped claims, sources |
| [admission-model.typ](admission-model.typ) | C2 formal model: key-expression matching, admission constraints, aggregation lemmas, envelope charging, open problems |
| [routing-semantics.md](routing-semantics.md) | Zenoh `main` routing behaviour the admission model depends on, with code references |
| [../../tools/detnet-sim](../../tools/detnet-sim/README.md) | C2 admission simulator (Rust, uses zenoh-keyexpr) and first results |
| [architecture.typ](architecture.typ) | Architecture diagrams and use cases (preemption, multiple subscribers, stateless transit, mobility); build with `typst compile` (typst 0.15) |
| [phase-1-flow-contract-admission.md](phase-1-flow-contract-admission.md) | RFC 9016 flow contract + RFC 9320 admission on stock Zenoh |
| [phase-2-topology-underlay.md](phase-2-topology-underlay.md) | Capacity in link-state, PCE plugin, DSCP/TSN underlay mapping |
| [phase-3-explicit-paths.md](phase-3-explicit-paths.md) | Segment-list network-message extension |
| [phase-4-stateless-scheduling.md](phase-4-stateless-scheduling.md) | C-SCORE / TCQF per-message scheduling metadata |
| [phase-5-replication-elimination.md](phase-5-replication-elimination.md) | DetNet PREOF over disjoint paths |

## Roadmap

See [roadmap.md](roadmap.md): four tracks — measure (C4), map (C1), admit (C2),
mobility (C3, stretch) — plus protocol extensions behind measurement gates, on a
containerlab testbed. The phase documents below are background; their place in the
new plan is listed in the roadmap.
