# Deterministic Networking in Zenoh

Campaign to make Zenoh carry deterministic flows by adopting existing IETF DetNet
and IEEE TSN standards rather than inventing new mechanisms. An admitted flow gets
reserved bandwidth, a bounded end-to-end latency and a loss budget.

Base: upstream `eclipse-zenoh/zenoh` `main` at `b180cc6e2` (1.10.2 + 10 commits).

## Documents

| Document | Content |
| --- | --- |
| [design.md](design.md) | Goals, principles, standards map, Zenoh baseline, architecture, risks |
| [prior-art.md](prior-art.md) | Literature check: contribution claims C1–C4, dropped claims, sources |
| [admission-model.typ](admission-model.typ) | C2 formal model: key-expression matching, admission constraints, aggregation lemmas, envelope charging, open problems |
| [routing-semantics.md](routing-semantics.md) | Zenoh `main` routing behaviour the admission model depends on, with code references |
| [architecture.typ](architecture.typ) | Architecture diagrams and use cases (preemption, multiple subscribers, stateless transit, mobility); build with `typst compile` (typst 0.15) |
| [phase-1-flow-contract-admission.md](phase-1-flow-contract-admission.md) | RFC 9016 flow contract + RFC 9320 admission on stock Zenoh |
| [phase-2-topology-underlay.md](phase-2-topology-underlay.md) | Capacity in link-state, PCE plugin, DSCP/TSN underlay mapping |
| [phase-3-explicit-paths.md](phase-3-explicit-paths.md) | Segment-list network-message extension |
| [phase-4-stateless-scheduling.md](phase-4-stateless-scheduling.md) | C-SCORE / TCQF per-message scheduling metadata |
| [phase-5-replication-elimination.md](phase-5-replication-elimination.md) | DetNet PREOF over disjoint paths |

## Roadmap

Wire changes start only after admission is proven on stock Zenoh.

```mermaid
flowchart TD
    P1["Phase 1 · Flow contract and admission<br/>RFC 9016, RFC 9320 · no wire change"]
    P2["Phase 2 · Topology and underlay mapping<br/>RFC 8939, RFC 9023 · OAM extension"]
    P3["Phase 3 · Explicit paths<br/>RFC 8402, RFC 9256 · wire change"]
    P4["Phase 4 · Stateless core scheduling<br/>C-SCORE / TCQF drafts · wire change"]
    P5["Phase 5 · Replication and elimination<br/>RFC 8655 PREOF, RFC 8938 · wire change"]
    P1 -->|"Gate: admitted flow stays under its RFC 9320 bound on the testbed"| P2
    P2 -->|"Gate: PCE paths agree with measured link capacity"| P3
    P3 -->|"Gate: legacy routers ignore the segment list and still deliver"| P4
    P4 -->|"Gate: bound holds with no per-flow state in transit routers"| P5
```

| Phase | Status |
| --- | --- |
| 1 | Planning |
| 2–5 | Not started |
