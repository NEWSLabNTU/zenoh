# Phase 3: Explicit paths

**Status:** not started · **Wire change:** yes (non-mandatory extension) ·
**Standards:** RFC 8402, RFC 9256

Carry the admitted path in each message so transit routers forward it without
per-flow state, the segment-routing model.

## Deliverables

1. **Segment-list network-message extension** — list of router IDs (or region IDs at
   the coarse level) on PUSH messages; non-mandatory so legacy routers ignore it and
   fall back to declaration-driven routing.
2. **Forwarding** — a router that finds itself at the head of the list pops it and
   forwards to the next segment.
3. **Path installation** — the PCE returns a segment list per subscriber path; the
   ingress router stamps it (SR Policy model, RFC 9256).
4. **Fan-out** — per-subscriber CSPF paths merged into a tree; measure against
   Steiner-tree optimum on test topologies.

**Gate to Phase 4:** legacy routers ignore the segment list and still deliver.

## Open questions

- One segment list per subscriber vs a tree encoding.
- Fit with the Regions multi-path inter-region routing.
