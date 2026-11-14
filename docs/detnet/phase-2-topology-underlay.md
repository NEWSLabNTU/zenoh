# Phase 2: Topology and underlay mapping

**Status:** not started · **Wire change:** OAM (link-state) extension ·
**Standards:** RFC 8939, RFC 9023, IEEE 802.1Qbv/Qav; PCEP (RFC 5440) and BGP-LS
(RFC 7752) as design references

Give admission a network-wide view of capacity, and map deterministic Zenoh lanes onto
an owned underlay where one exists.

## Deliverables

1. **Capacity and usage bands in link-state** — extend `linkstate_peer` OAM messages
   with link capacity and a usage band (<50%, 50–80%, 80–95%, >95%); re-flood only on a
   band crossing, with hold-down timers (as MPLS-TE flooding thresholds do).
2. **PCE plugin** — zenohd plugin owning per-link reservations across ingress routers;
   CSPF over the link-state graph (petgraph) with exponential link cost.
3. **Capacity input** — quinn `Connection::stats()` (RTT, cwnd, loss) for measured
   links; static or imported capacity for owned links.
4. **Underlay mapping** — DSCP per deterministic flow (today DSCP is per endpoint);
   document mapping to RFC 8939 IP DetNet flows and RFC 9023 TSN streams.

**Gate to Phase 3:** PCE paths agree with measured link capacity.

## Open questions

- Interaction with Regions (#2096): flood bands only inside a region, and aggregate at
  gateways?
- Hidden shared bottlenecks between overlay links: correlated delay/loss detection?
