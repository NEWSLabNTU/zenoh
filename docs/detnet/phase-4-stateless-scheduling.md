# Phase 4: Stateless core scheduling

**Status:** not started · **Wire change:** yes ·
**Standards:** draft-ietf-detnet-stateless-fair-queuing (C-SCORE),
draft-ietf-detnet-tcqf (TCQF)

Turn the firm tier from measured into bounded: transit routers schedule by
per-message metadata instead of per-flow state.

## Deliverables

1. **Scheduling metadata extension** — C-SCORE finish time (set at ingress from the
   fluid model, updated per hop) or TCQF cycle ID.
2. **Egress scheduler** — router transmit path serves deterministic lanes in
   finish-time order (C-SCORE) or per-cycle buffers (TCQF).
3. **Bound calculator** — RFC 9320-style end-to-end bound for the chosen mechanism,
   fed back into Phase 1 admission.

C-SCORE needs no clock synchronisation, and its bound depends on the reserved rate, so
latency becomes a bandwidth reservation. TCQF gives tighter jitter for periodic
streams but needs coarse clock sync and controlled link delay.

**Gate to Phase 5:** the bound holds with no per-flow state in transit routers.

## Open questions

- Pin which draft revisions.
- Can the transmit pipeline reorder within a priority queue without breaking Zenoh's
  per-priority sequence numbers?
