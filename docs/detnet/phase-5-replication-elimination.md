# Phase 5: Replication and elimination

> Superseded by [roadmap.md](roadmap.md); this phase now lives in E4, behind gate G4.

**Status:** not started · **Wire change:** yes ·
**Standards:** DetNet PREOF (RFC 8655), RFC 8938

Meet the loss budget against link failures, handover and wireless loss.

## Deliverables

1. **Flow sequence number** — per-flow sequence extension stamped at ingress.
2. **Replication** — ingress sends copies over two disjoint segment lists (e.g. Wi-Fi
   and cellular access).
3. **Elimination** — duplicate removal at the egress router or subscriber.
4. **FEC option** — for random wireless loss, sized to the loss budget; retransmit
   only when the deadline leaves room for a round trip.

## Open questions

- Elimination point: egress router or subscriber session.
- Disjoint-path computation in the PCE, including shared-risk groups for overlay links.
