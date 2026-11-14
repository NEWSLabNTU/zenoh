# Phase 1: Flow contract and admission

**Status:** planning · **Wire change:** none · **Standards:** RFC 9016, RFC 9320

Add RFC 9016 flow contracts and RFC 9320 admission on stock Zenoh, and measure
whether a firm latency bound holds on a Zenoh router before any protocol work.

## Deliverables

1. **`zenoh-detnet` crate** — Rust types for the RFC 9016 model, serde + JSON5 config:
    - traffic specification: interval, max packets per interval, max payload size
      (min values optional);
    - traffic requirements: max latency, max latency variation, max loss,
      max consecutive loss tolerance, max misordering.
2. **Contract announcement** — the publisher announces its contract as a liveliness
   token plus an admin-space entry, e.g. `@detnet/<zid>/<flow-id>`. The token's
   lifetime is the reservation's lifetime.
3. **Admission plugin (zenohd)** — reads the topology from the admin space, computes
   the RFC 9320 worst-case latency along the current route, checks per-link reserved
   rate and buffer, and accepts or rejects.
4. **Edge policer interceptor** — token bucket per admitted flow at the ingress
   router; non-conforming messages are demoted with `qos_overwrite` or dropped.
5. **Deterministic-lane profile** — config preset: batching off or `express`,
   `congestion_control: drop`, a dedicated priority, and a `?prio=` link with DSCP.

## Design notes

- Admission runs where the flow enters: the ingress router of the publisher. No
  transit router learns about the flow.
- The latency model per hop follows RFC 9320's per-hop bound for the queueing
  mechanism in use; in Phase 1 that is strict-priority queueing on a dedicated
  priority lane, measured rather than proven.
- Per-link reserved rate is held by the admission plugin; links shared by several
  ingress routers need a single owner (the PCE in Phase 2). Phase 1 tests one ingress.

## Testbed and exit criteria

- [ ] 3 to 5 routers on owned Ethernet; one deterministic flow plus best-effort cross
      traffic at 50%, 80% and 100% link load.
- [ ] p99.99 and max one-way latency of the admitted flow stay under the RFC 9320
      bound computed by the plugin.
- [ ] Zero congestion loss for admitted flows; rejected flows never enter the network.
- [ ] Jitter from async scheduling reported with and without pinned cores and a
      real-time kernel — decides whether the firm tier is realistic.

**Gate to Phase 2:** an admitted flow stays under its RFC 9320 bound on the testbed.

## Open questions

- Should the testbed include a cellular or Wi-Fi hop from the start?
- Contract carried in a liveliness token key vs a declaration extension: the token
  needs no wire change but limits contract size to what fits in a key expression.
- Clock source for one-way latency measurement (PTP on the testbed?).
