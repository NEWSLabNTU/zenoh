# Track C: Admit (claim C2)

**Status:** model and first simulator done · **Wire change:** none

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

## Next

### C2 Simulator extensions

- Envelope reservations with charging rules: most specific, first declared, split.
- Preemption by priority (RFC 4829-style victim selection).
- Sensitivity to subscriber mix, publisher wildcards and topology.
- Peer regions with star delivery; clients behind aggregate interests.
- Capacity-aware routing vs shortest-path trees (feeds gate G3).
- Incremental retry instead of rescanning all pending pairs.

### C3 Proofs

Lemmas 1–2 (OR-bit for bandwidth, min-slack for deadlines) for Zenoh's actual
declaration propagation, and the weakest scheduling assumption under which they hold.

### C4 Admission plugin

- zenohd plugin with exact router-level state driven by declarations and the contracts
  of track B.
- Policing in an interceptor at the ingress router.
- Validated in the track A testbed against the simulator's predictions.

## Gate G2

After C2: does wildcard admission keep its advantage over exact identifiers across
subscriber mixes and topologies? If not, narrow the claim to the mixes where it holds.
