# Track B: Map (claim C1)

**Status:** not started · **Wire change:** none · **Model:** OMG DDS-TSN

Give each deterministic key expression a stream identity that a DetNet or TSN network
can see, with reservations created and removed by Zenoh declarations. DDS-TSN solves
this for DDS; zenoh-pico's raweth transport already maps key expressions to MAC and
VLAN statically; Zenoh multilink separates traffic by priority class only
([prior-art.md](prior-art.md), C1).

## Steps

### B1 Contracts

- `zenoh-detnet` crate: Rust types for RFC 9016 contracts — traffic specification
  (interval, max packets per interval, max payload size) and requirements (max latency,
  max latency variation, max loss, max consecutive loss, max misordering); serde and
  JSON5.
- **Flow contract** for a publisher, announced as a liveliness token plus admin-space
  entry, e.g. `@detnet/<zid>/pub/<id>`; the token's lifetime is the reservation's.
- **Receive contract** for a subscriber: deterministic selector and budget, announced
  the same way. It is separate from the subscription because session aggregation
  widens subscriptions ([routing-semantics.md](routing-semantics.md), finding 7).

### B2 Mapping

- Deterministic key expression → dedicated `?prio=` link or session → DSCP or distinct
  5-tuple, so each stream is identifiable on the wire (DDS-TSN §8.3.1 approach; RFC
  8939 asks not to mix DetNet and other traffic in one 5-tuple).
- Interceptor that selects the link for samples of admitted key expressions.

### B3 TSN emulation

- Linux `mqprio`/`taprio` (software mode) on containerlab veth links to show the mapping
  end to end without TSN hardware. First check veth multi-queue support.

### B4 Upstream pull requests

Small, generic changes only, for example:

- link statistics (rate, queue, drops) in the admin space;
- hooks the out-of-tree admission plugin needs;
- follow per-link `SO_PRIORITY` (PR #2881).

## Done when

A subscriber's receive contract and a publisher's flow contract produce a reserved
stream that the emulated network can identify and schedule, and undeclaring either
removes it.
