# Design: Deterministic Networking in Zenoh

## Goal and scope

Make Zenoh carry deterministic flows by adopting existing IETF DetNet and IEEE TSN
standards. A flow that declares its traffic and is admitted gets three guarantees:

- reserved bandwidth;
- a bounded end-to-end latency;
- a loss budget.

**Non-goals:** a new routing protocol, clock-synchronised scheduling across the
internet, and replacing an underlay's own traffic engineering where one exists.

## Principles

- **Reuse standards.** Every mechanism maps to an RFC or an IETF working-group draft;
  Zenoh-specific work is limited to carrying it over Zenoh sessions and key expressions.
- **State at the edge and in the packet.** Transit routers keep at most one counter per
  link, never per-flow state. Per-flow state lives at the ingress router (policing) and
  in per-message metadata (path, scheduling tag, sequence number).
- **Incremental deployment.** New network-message extensions are non-mandatory, so
  routers that do not understand them fall back to normal routing.
- **Measure before changing the wire.** Phases that need no protocol change come first.

## Guarantee tiers

Hard bounds only hold where every queue and link on the path is controlled.

| Tier | Where | What Zenoh does |
| --- | --- | --- |
| Hard | Owned links with a TSN or DetNet underlay | Map flows to underlay DetNet flows; regulate at the edge |
| Firm | Owned Zenoh routers over measured links | Stateless fair-queuing scheduling on router egress; per-link delay bounds from measurement |
| Statistical | Internet or cellular segments | Percentile targets (e.g. p99.9), replication and FEC for loss, continuous monitoring |

## Standards map

| DetNet function | Standard | Zenoh `main` today | Gap | Phase |
| --- | --- | --- | --- | --- |
| Architecture and terms | RFC 8655 | — | Adopt its vocabulary (service vs forwarding sub-layer) | all |
| Flow description | RFC 9016 | Publisher declares priority and `express` only | Flow-contract declaration | 1 |
| Admission and latency bound | RFC 9320 | — | Edge admission, per-hop latency calculator | 1 |
| Mapping to an owned underlay | RFC 8939 (IP), RFC 9023 (IP over TSN), IEEE 802.1Qbv/Qav | `#dscp=` per TCP/UDP endpoint; `?prio=` pins a priority range to a link | DSCP per flow | 2 |
| Topology and capacity export | DetNet YANG (RFC 9633), PCEP (RFC 5440), BGP-LS (RFC 7752) | Admin space, `linkstate_peer` OAM messages | Capacity and usage bands in link-state | 2 |
| Explicit path | RFC 8402, RFC 9256 | Declaration-driven trees; NodeId extension for loop prevention only | Segment-list network-message extension | 3 |
| Stateless core queueing | draft-ietf-detnet-stateless-fair-queuing (C-SCORE), draft-ietf-detnet-tcqf (TCQF) | 8 priority lanes, `qos_overwrite` interceptor | Finish-time or cycle tag per message; egress scheduling by it | 4 |
| Replication and elimination | DetNet PREOF (RFC 8655, RFC 8938) | — | Copies over disjoint paths, duplicate elimination at the receiver | 5 |

> RFC numbers and titles were collected from memory and earlier design notes; verify
> each against the RFC Editor before citing it in code or publications.

## Zenoh baseline on `main`

### Hooks to build on

- **Priority lanes:** 8 priorities with separate queues and sequence numbers. Endpoints
  accept `?prio=<range>;rel=<0|1>` to dedicate a link to a priority range.
- **DSCP:** `#dscp=0x..` on TCP/UDP endpoints sets the IP header field
  (`io/zenoh-link-commons/src/dscp.rs`).
- **Interceptors:** `access_control`, `downsampling`, `low_pass`, `qos_overwrite` in
  `zenoh/src/net/routing/interceptor/` — the place for per-link counters, policing and
  demotion.
- **Regions and gateways:** hierarchical, multi-path inter-region routing (#2096;
  `zenoh/src/net/routing/gateway.rs`, `zenoh/tests/regions/`).
- **Admin space and plugins:** `@/<zid>/…` queryable state and Rust zenohd plugins —
  the host for admission and path computation.

### Blockers for bounded latency

| Behaviour | Default on `main` | Why it breaks a bound | Fix direction |
| --- | --- | --- | --- |
| `lowlatency` transport | `false`; incompatible with `qos` | Fast path loses priority lanes | Low-latency path that keeps priorities |
| Adaptive batching | `enabled: true`, `time_limit: 1` ms | Up to 1 ms extra per hop under back-pressure | `express` or batching off for deterministic lanes |
| `congestion_control: block` | `wait_before_close: 5000000` µs | Unbounded wait | Drop plus admission for deterministic lanes |
| Queue sizing | 2 batches per priority | Not derived from any flow's burst | Size from the RFC 9320 calculation |

TCP and QUIC streams add head-of-line blocking and retransmission delay, and async
user-space scheduling adds jitter. The firm tier needs UDP or QUIC datagrams, pinned
cores and a real-time kernel on analysed routers.

## Architecture

Three timescales, so that fast decisions use exact local state and only slow-changing
information is flooded.

| Timescale | Mechanism | Who holds state |
| --- | --- | --- |
| Minutes | Capacity and usage bands flooded; edge-to-sink trunks resized | Link-state, PCE |
| Seconds | Admission against trunk or link headroom; handover re-admission | Ingress (edge) router |
| Milliseconds | Policing, scheduling, congestion control, layer dropping | Edge router, endpoints |

Roles:

- **Publisher** declares an RFC 9016 contract.
- **Ingress router** admits the flow (RFC 9320), polices it with a token bucket and,
  from Phase 3, stamps the path and scheduling metadata.
- **Transit routers** forward and schedule by per-message metadata; one load counter
  per link.
- **PCE** (zenohd plugin) holds the topology view, computes paths and resizes trunks.

## Risks and open questions

- **Scheduler jitter:** can a tokio-based router meet a firm bound, or does the firm
  tier need a dedicated data-plane thread? Phase 1 measures this.
- **Pub/sub fan-out:** RFC 9320 assumes point-to-point flows; a publisher feeding a
  tree needs a bound per subscriber path.
- **Hidden shared bottlenecks:** overlay links over the same physical link do not add
  up in capacity; a detection method is needed.
- **Regions overlap:** does #2096's multi-path inter-region routing host explicit paths
  or conflict with a segment-list extension? Raise with ZettaScale early.
- **C-SCORE or TCQF:** C-SCORE needs no clock sync; TCQF gives tighter jitter but needs
  coarse sync and controlled link delay. Likely one per tier.
- **Draft churn:** C-SCORE and TCQF are drafts; pin draft versions in code.

## References

- [RFC 8655 DetNet architecture](https://www.rfc-editor.org/rfc/rfc8655.html)
- [RFC 9016 DetNet flow and service information model](https://www.rfc-editor.org/rfc/rfc9016.html)
- [RFC 9320 DetNet bounded latency](https://www.rfc-editor.org/rfc/rfc9320.html)
- [RFC 8939 DetNet data plane: IP](https://www.rfc-editor.org/rfc/rfc8939.html)
- [RFC 9023 DetNet IP over TSN](https://www.rfc-editor.org/rfc/rfc9023.html)
- [RFC 8402 Segment Routing architecture](https://www.rfc-editor.org/rfc/rfc8402.html)
- [RFC 9256 SR Policy architecture](https://www.rfc-editor.org/rfc/rfc9256.html)
- [C-SCORE draft](https://datatracker.ietf.org/doc/draft-ietf-detnet-stateless-fair-queuing/)
- [TCQF draft](https://datatracker.ietf.org/doc/draft-ietf-detnet-tcqf/)
- [Zenoh spec: routing](https://spec.zenoh.io/spec/1.0.0/architecture/routing.html)
- [Zenoh spec: QoS](https://spec.zenoh.io/spec/1.0.0/concepts/qos.html)
