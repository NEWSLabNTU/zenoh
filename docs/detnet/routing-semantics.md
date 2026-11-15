# Zenoh routing semantics relevant to admission (C2)

Code check of upstream `main` (`b180cc6e2`, branch `detnet`) on 2026-11-15, to pin down
the assumptions of [admission-model.typ](admission-model.typ). Line numbers refer to
that revision.

## Findings

| # | Question | Answer | Evidence |
| --- | --- | --- | --- |
| 1 | Can a publisher use a wildcard key expression? | Yes. A publisher declared on `test/*` delivers to a subscriber on `test/cache` | `zenoh/src/tests/interceptor_cache.rs:191-199` |
| 2 | How is a sample matched to subscribers? | By key-expression intersection: the router collects all resources matching the sample's key expression, wildcards on either side | `Resource::get_matches`, `zenoh/src/net/routing/dispatcher/resource.rs:809` |
| 3 | What path does data take between routers? | A shortest-path tree rooted at the source router: next hop toward each router that holds a matching subscriber is `net.trees[source].directions[sub]` | `zenoh/src/net/routing/hat/router/pubsub.rs:311` |
| 4 | What does a router know about remote subscribers? | Each subscriber declaration is flooded along the origin router's tree with that router's node id, so every router knows the set of routers holding matching subscribers (`router_subs`) | `hat/router/pubsub.rs:110`, `:128` (`propagate_sourced_subscriber`) |
| 5 | What happens across regions? | Traffic from another region is routed on the local router's own tree (`router_source` = own index), so trees are re-rooted at the region boundary | `hat/router/pubsub.rs:352` |
| 6 | How do peers deliver? | Directly to every peer face that has a matching subscriber, one copy per face; peers do not relay to other peers in the same region | `zenoh/src/net/routing/hat/peer/pubsub.rs:255` |
| 7 | What does session `aggregation` do? | If a subscriber's key expression is included in a configured aggregation key expression, the session declares that wider key expression once (and shares it among local subscribers); identical key expressions are also declared once | `zenoh/src/api/session.rs:421` |
| 8 | What does an aggregate interest do? | A router answering a client/peer interest with the `aggregate` option sends declarations summarised at the interest's key expression instead of one per resource; peers may not request it | `hat/broker/pubsub.rs:74`; `dispatcher/interests.rs:220` |
| 9 | Is data ever sent before subscribers are known? | Yes: toward south faces whose subscriber interest is not finalized, data is forwarded regardless of matches | `hat/peer/pubsub.rs:278` |

## Consequences for the model

1. **Wildcard publishers are real** (finding 1). The footnote in the model is resolved:
   kₚ may contain wildcards and is matched by intersection, as assumed.
2. **Inside a router region, there is no hidden fan-out at router granularity**
   (findings 3–4). Every router knows which routers hold matching subscribers, and
   paths are fixed by the source router's shortest-path tree (link weights via
   `transport_weights`). The admission unit there is the pair
   (publisher's router, subscriber's router); the tree Tₚ of the model is exactly
   the union of source-tree paths to those routers.
3. **Information is lost only at the network's edge**, in two places:
   - *Session aggregation widens the declaration* (finding 7): the network sees
     k̂ₛ ⊇ kₛ and delivers, and would reserve for, publishers the
     application filters out locally. The deterministic selector dₛ must therefore
     be carried separately and never aggregated.
   - *Aggregate interests* (finding 8) summarise subscribers behind a router for its
     clients. That is the place where Lemma 1's OR-bit summary applies.
4. **Regions compose** (finding 5): each region carries a tree segment rooted at its
   entry router; admission composes per region, matching the per-region bitstring
   re-encoding planned for BIER-TE.
5. **Peer regions have no in-network replication** (finding 6). A publishing peer sends
   one copy per subscribing peer, so its access link carries rₚ times its peer
   fan-out. For a mobile publisher with several subscribers, an edge router is needed
   to get one copy on the uplink; the admission model must use star delivery for
   peer regions.
6. **Transient over-delivery** (finding 9): before interests are finalized, data can
   reach faces without matching subscribers. Admission must either count this window
   or hold deterministic flows until declarations settle.
