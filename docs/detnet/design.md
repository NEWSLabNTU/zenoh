# Design: Deterministic Networking in Zenoh

## Goal and scope

Make Zenoh carry deterministic flows by adopting existing IETF DetNet and IEEE TSN
standards. A flow that declares its traffic and is admitted gets three guarantees:

- reserved bandwidth;
- a bounded end-to-end latency;
- a loss budget.

**Non-goals:** a new routing protocol, clock-synchronised scheduling across the
internet, and replacing an underlay's own traffic engineering where one exists.

**Positioning.** A DDS-TSN equivalent for Zenoh, plus overlay mechanisms only where no
underlay control exists. Nearly every mechanism here is prior art; the contributions
are four narrower claims, checked against the literature in
[prior-art.md](prior-art.md):

| Claim | Contribution | Type | Track |
| --- | --- | --- | --- |
| C1 | Per-key-expression stream identity visible to DetNet/TSN, with reservations driven by declarations | Engineering; follows OMG DDS-TSN | [B](track-b-map.md) |
| C2 | Admission when publisher and subscriber declarations are both wildcard key expressions, the receiver set being their intersection, expanded exactly and immediately inside the network (gate G2: not a gain over an ideal, frequently refreshed application-side expansion) | Research; main claim | [C](track-c-admit.md) |
| C3 | Firm bounds for flows across edge handover (mobile transit only if peer relaying returns upstream) | Research; stretch | [D](track-d-mobility.md) |
| C4 | Measured tail latency of a tokio-based Zenoh router, and of software stateless queuing in it | Empirical | [A](track-a-measure.md) |

Work is organised in tracks with measurement gates; see [roadmap.md](roadmap.md).

## Principles

- **Reuse standards.** Every mechanism maps to an RFC or an IETF working-group draft;
  Zenoh-specific work is limited to carrying it over Zenoh sessions and key expressions.
- **Out of tree first.** Build on a zenohd plugin, interceptors and configuration; send
  upstream only small, generally useful pull requests.
- **State at the edge.** Per-flow state lives at the ingress router (policing) and in
  the admission plugin. Transit routers keep at most one counter per link.
- **Incremental deployment.** Any new network-message extension is non-mandatory, so
  routers that do not understand it fall back to normal routing.
- **Measure before changing the wire.** Segment lists, BIER-TE trees, scheduling tags
  and replication ([track E](track-e-extensions.md)) are added only when a gate shows
  the simpler option — Zenoh `transport_weights`, one `?prio=` link per deterministic
  flow, or underlay/5G QoS — is not enough.

## Guarantee tiers

Hard bounds only hold where every queue and link on the path is controlled.

| Tier | Where | What Zenoh does |
| --- | --- | --- |
| Hard | Owned links with a TSN or DetNet underlay | Map flows to underlay DetNet/TSN streams (C1); regulate at the edge |
| Firm | Owned Zenoh routers over measured links | Admission (C2) plus router scheduling; viable only if gate G1 passes |
| Statistical | Internet or cellular segments | Percentile targets (e.g. p99.9), monitoring; replication only behind gate G4 |

## Standards map

| DetNet function | Standard | Zenoh `main` today | Gap | Track |
| --- | --- | --- | --- | --- |
| Architecture and terms | RFC 8655 | — | Adopt its vocabulary (service vs forwarding sub-layer) | all |
| Flow description | RFC 9016 | Publisher declares priority and `express` only | Flow contract and receive contract | B1 |
| Topic → stream mapping | OMG DDS-TSN (model) | Multilink by priority class (`?prio=`), `#dscp=` per endpoint; zenoh-pico raweth maps key → MAC/VLAN statically | Identity per key expression in the router | B2 |
| Admission and latency bound | RFC 9320 | — | Admission for wildcard-defined receiver sets | C |
| Mapping to an owned underlay | RFC 8939 (IP), RFC 9023 (IP over TSN), IEEE 802.1Qbv/Qav/Qcc | As above | DSCP or 5-tuple per deterministic flow; TSN emulation | B2, B3 |
| Topology and capacity export | DetNet YANG (RFC 9633), PCEP, BGP-LS (design references) | Admin space; router link-state | Link statistics in admin space | B4 |
| Explicit path | RFC 8402, RFC 9256 | Source shortest-path trees; static `transport_weights` | Segment-list extension | E1 (G3) |
| Stateless multicast tree | RFC 9262 (BIER-TE) | Source trees, re-rooted per region | Bitstring extension | E2 (G3) |
| Stateless core queueing | C-SCORE and TCQF drafts | 8 priority lanes, `qos_overwrite` interceptor | Scheduling tag and egress scheduler | A3/E3 (G1) |
| Replication and elimination | DetNet PREOF (RFC 8655, RFC 8938) | — | Copies over disjoint paths, elimination at receiver | E4 (G4) |

> RFC 8655, 9016, 9320 and 8939 were read during the literature check; other numbers
> and titles still need checking against the RFC Editor before publication.

## Zenoh baseline on `main`

### Routing behaviour

Checked in the code; details and line references in
[routing-semantics.md](routing-semantics.md).

- Samples are matched to subscribers by key-expression **intersection**; publishers
  may use wildcards.
- **Router regions** route each sample on the source router's shortest-path tree;
  every router knows which routers hold matching subscribers.
- Trees are **re-rooted at region boundaries**.
- **Peer regions** do not relay: a publishing peer sends one copy per subscribing peer,
  so a mobile publisher needs an edge router to send one copy on its uplink.
- **Session aggregation** widens subscriptions; the deterministic selector must be
  signalled separately.
- Data may be forwarded before interests are finalized.

### Hooks to build on

- **Priority lanes and multilink:** 8 priorities with separate queues; endpoints accept
  `?prio=<range>;rel=<0|1>` to dedicate a link to a priority range.
- **DSCP:** `#dscp=0x..` on TCP/UDP endpoints (`io/zenoh-link-commons/src/dscp.rs`);
  per-link `SO_PRIORITY` proposed upstream (PR #2881, open).
- **Interceptors:** `access_control`, `downsampling`, `low_pass`, `qos_overwrite` in
  `zenoh/src/net/routing/interceptor/` — the place for policing and demotion.
- **Regions and gateways:** hierarchical, multi-path inter-region routing (#2096).
- **Admin space and plugins:** `@/<zid>/…` queryable state and Rust zenohd plugins —
  the host for the admission plugin.

### Blockers for bounded latency

| Behaviour | Default on `main` | Why it breaks a bound | Fix direction |
| --- | --- | --- | --- |
| `lowlatency` transport | `false`; incompatible with `qos` | Fast path loses priority lanes | Low-latency path that keeps priorities |
| Adaptive batching | `enabled: true`, `time_limit: 1` ms | Up to 1 ms extra per hop under back-pressure | `express` or batching off for deterministic lanes |
| `congestion_control: block` | `wait_before_close: 5000000` µs | Unbounded wait | Drop plus admission for deterministic lanes |
| Queue sizing | 2 batches per priority | Not derived from any flow's burst | Size from the RFC 9320 calculation |
| tokio work stealing | multi-threaded runtime | Judged unsuitable for real time (Škoudlil et al., ECRTS 2025) | Measured in track A; single-threaded data path if needed |

TCP and QUIC streams add head-of-line blocking and retransmission delay.

## Architecture

Three timescales, so that fast decisions use exact local state and only slow-changing
information is flooded.

| Timescale | Mechanism | Who holds state |
| --- | --- | --- |
| Minutes | Link capacities and statistics; guard bands resized | Admission plugin |
| Seconds | Admission per declaration event; handover re-admission | Admission plugin, ingress router |
| Milliseconds | Policing, scheduling, congestion control, layer dropping | Ingress router, endpoints |

Roles:

- **Publisher** declares an RFC 9016 contract on its key expression.
- **Subscriber** declares a receive contract: deterministic selector and budget.
- **Admission plugin** (zenohd) tracks declarations, computes the eligible pairs by
  key-expression intersection and admits them against capacity, deadline and budget
  ([admission-model.typ](admission-model.typ)).
- **Ingress router** polices admitted flows with a token bucket in an interceptor.
- **Transit routers** forward on Zenoh's normal trees; one load counter per link.

Diagrams and use cases (preemption, multiple subscribers, mobility, many mobile
publishers): [architecture.typ](architecture.typ).

## Risks and open questions

- **Scheduler jitter:** does the tail come from queueing or from host and runtime
  noise? Decides the firm tier (gate G1). No tail-latency data for Zenoh exists yet.
- **Relative-only latency results:** the containerlab testbed shares 4 cores among all
  routers; absolute bounds need dedicated cores or a real-time kernel.
- **Wildcard admission benefit:** first simulator results (70% of eligible pairs
  admitted safely vs 3% with exact identifiers) come from a synthetic mix; gate G2
  checks other mixes and topologies.
- **Overlapping envelopes:** no charging rule yet for reservations whose key
  expressions overlap without inclusion.
- **Hidden shared bottlenecks:** overlay links over the same physical link do not add
  up in capacity.
- **Draft churn:** C-SCORE and TCQF are drafts; pin draft versions if used.

## References

- [RFC 8655 DetNet architecture](https://www.rfc-editor.org/rfc/rfc8655.html)
- [RFC 9016 DetNet flow and service information model](https://www.rfc-editor.org/rfc/rfc9016.html)
- [RFC 9320 DetNet bounded latency](https://www.rfc-editor.org/rfc/rfc9320.html)
- [RFC 8939 DetNet data plane: IP](https://www.rfc-editor.org/rfc/rfc8939.html)
- [RFC 9023 DetNet IP over TSN](https://www.rfc-editor.org/rfc/rfc9023.html)
- [OMG DDS-TSN 1.0](https://www.omg.org/spec/DDS-TSN/1.0)
- [RFC 8402 Segment Routing architecture](https://www.rfc-editor.org/rfc/rfc8402.html)
- [RFC 9256 SR Policy architecture](https://www.rfc-editor.org/rfc/rfc9256.html)
- [C-SCORE draft](https://datatracker.ietf.org/doc/draft-ietf-detnet-stateless-fair-queuing/)
- [TCQF draft](https://datatracker.ietf.org/doc/draft-ietf-detnet-tcqf/)
- [Zenoh spec: routing](https://spec.zenoh.io/spec/1.0.0/architecture/routing.html)
- [Zenoh spec: QoS](https://spec.zenoh.io/spec/1.0.0/concepts/qos.html)
- Full literature list: [prior-art.md](prior-art.md)
