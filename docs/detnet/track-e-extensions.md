# Track E: Gated protocol extensions

**Status:** not started; each item starts only when its gate triggers

Protocol extensions are prior art adapted to Zenoh. Each needs a non-mandatory
network-message extension, so it is added only when a measurement shows the simpler
option is not enough.

| Item | Standard model | What | Gate |
| --- | --- | --- | --- |
| E1 Explicit paths | RFC 8402, RFC 9256 | Segment list of router IDs on PUSH; loose segments name only waypoints | G3 |
| E2 Stateless trees | RFC 9262 (BIER-TE) | Bitstring of adjacencies set at ingress; branch routers replicate; re-encoded per region | G3 |
| E3 Scheduling tags | C-SCORE / TCQF drafts | Finish time or cycle ID per message; egress scheduler | G1 |
| E4 Replication and elimination | DetNet PREOF (RFC 8655, RFC 8938) | Flow sequence number; copies over disjoint paths; elimination at egress | G4 |

## Gates

- **G3** (from track C): capacity-aware routing in the simulator admits clearly more
  than Zenoh's shortest-path trees plus `transport_weights`. **Not triggered**
  (2026-11-16): +2 to +5 points only, so E1/E2 stay parked unless a re-optimising
  variant or a real topology shows more. Relaying inside peer regions matters more.
- **G1** (from track A): the firm tier is viable and queueing dominates the tail.
- **G4** (from tracks A and D): loss budgets are missed on lossy emulated links.

## Notes carried over from the earlier phase plans

- Segment lists: one per subscriber vs a tree encoding; for 2–3 subscribers a list may
  be shorter than a bitstring.
- BIER-TE bitstrings have fixed length, so trees are encoded per region and re-encoded
  at gateways, which matches Zenoh's re-rooting of trees at region boundaries.
- Scheduling tags must not break Zenoh's per-priority sequence numbers when the
  transmit pipeline reorders within a priority.
- Replication over Wi-Fi and cellular should duplicate the base layer only.

Design background: use cases 1–3 in [architecture.typ](architecture.typ).
