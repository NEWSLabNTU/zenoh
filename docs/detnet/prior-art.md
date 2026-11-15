# Prior art and contribution claims

Literature check of 2026-11-15. Almost every mechanism in this design is prior art;
the defensible contributions are four narrower claims, listed first. Sources marked
"abstract" or "snippet" were not read in full and need checking before citation.

## Contribution claims

| # | Claim | Status | Type |
| --- | --- | --- | --- |
| C1 | Per-key-expression stream identity visible to a DetNet/TSN network, with reservations created and removed by Zenoh declarations | Open for Zenoh; solved for DDS by DDS-TSN | Engineering (port of a known approach) |
| C2 | Admission when both publisher and subscriber declarations are wildcard key expressions, so the receiver set is an intersection that any new declaration can enlarge, over a router mesh that aggregates declarations | Open; dynamic membership itself is solved | Research |
| C3 | DetNet-grade (firm, composable) bounds for flows whose transit node is itself mobile, including during repair | Open; only soft QoS exists | Research (hard) |
| C4 | Measured tail and worst-case latency of a tokio-based Zenoh router, with and without real-time tuning; software stateless queuing (C-SCORE/TCQF) evaluated in it | Open: no tail data for Zenoh, no measured software C-SCORE/TCQF | Empirical |

Claims dropped after the search:

- "Mapping pub/sub topics to TSN/DetNet flows is new" — OMG DDS-TSN does this for DDS.
- "A Zenoh session is one undifferentiated connection" — Zenoh ≥ 1.0 multilink already
  splits a session by priority class (`?prio=`), with `#dscp=` per endpoint.
- "Admission for a dynamic subscriber set is open" — solved since RSVP for exact group
  or stream identifiers.
- "Nobody handles reservations when transit nodes move" — MANET QoS did, with soft
  guarantees only.
- "Whether user-space software can hold bounded latency is open" — tuned DPDK and
  PREEMPT_RT stacks show tens of µs tails empirically; only the tokio case is open.

## C1: pub/sub to TSN/DetNet mapping

| Source | Year | Finding | Read |
| --- | --- | --- | --- |
| [OMG DDS-TSN 1.0](https://www.omg.org/spec/DDS-TSN/1.0) | 2023 beta, formal 2025–26 | DataWriter+Topic modelled as an 802.1Qcc Talker, readers as Listeners. §8.3.1 faces the same multiplexing problem (RTPS shares sockets) and requires a port, address or DSCP per DataWriter so the 6-tuple identifies the stream | Beta 1 full text |
| [zenoh-pico raweth transport](https://github.com/eclipse-zenoh/zenoh-pico/tree/main/src/transport/raweth) | 2023 (0.11.0) | Static `mapping` config: key expression → destination MAC + optional VLAN, i.e. a TSN stream identity; multicast only, no reservation | source |
| [Zenoh 1.0 "Firesong"](https://zenoh.io/blog/2024-10-21-zenoh-firesong/) | 2024 | Multilink: links tagged with priority range and reliability; transport picks a link per message | yes |
| [zenoh PR #2881](https://github.com/eclipse-zenoh/zenoh/pull/2881) (open) | 2026 | Per-link `SO_PRIORITY`; real-time link p99 15 ms → 0.9 ms on its own NIC queue. No TSN mapping | yes |
| [RFC 9016](https://www.rfc-editor.org/rfc/rfc9016.html) §3.3 | 2021 | A DetNet flow may aggregate N app-flows, so one Zenoh link can be one DetNet flow | yes |
| [RFC 8939](https://www.rfc-editor.org/rfc/rfc8939.html) §4.1 | 2020 | End systems should not mix DetNet and non-DetNet traffic in one 5-tuple | yes |
| [Luo et al., DDS over TSN in-vehicle (arXiv 2409.05904)](https://arxiv.org/abs/2409.05904) | 2024 | DDS discovery drives TSN configuration and flow identification | abstract |
| [NXP dds-tsn](https://github.com/NXP/dds-tsn) | — | ROS 2 topic on a VLAN with PCP 5; priority only, no reservation | yes |
| OPC UA PubSub over TSN ([Ekrad et al. 2026](https://arxiv.org/abs/2602.19603); Rezabek et al. 2024; Denzler et al.) | 2024–26 | Second mature line of pub/sub-over-TSN work | abstract / snippet |

**What is left for Zenoh:** identity per key expression rather than per priority class,
in the router (not only zenoh-pico), with reservations driven by declarations.

## C2: admission for wildcard-defined receiver sets

| Source | Year | Finding | Read |
| --- | --- | --- | --- |
| [RFC 2205 RSVP](https://www.rfc-editor.org/rfc/rfc2205) | 1997 | Receiver-initiated reservations merged at branch points; soft state handles changing membership; Wildcard-Filter style extends to new senders | yes (~40%) |
| [RFC 3754](https://www.rfc-editor.org/rfc/rfc3754) | 2004 | "Neglected Reservation Subtree": a grafted branch uses unreserved resources; new branches get Limited Effort until admitted | yes |
| [Stream Reservation Protocol (802.1Qat)](https://en.wikipedia.org/wiki/Stream_Reservation_Protocol), P802.1DD RAP | 2010, 2019–23 | Talker Advertise / Listener Ready, per-hop admission, dynamic Listener Attach per StreamID | summary / snippet |
| [IndiQoS (Carvalho et al.)](https://www.dpss.inesc-id.pt/~ler/reports/nca05.pdf) | 2005 | Per-link reservations for type-based pub/sub via DHT rendezvous on hash(event type); no wildcards | yes |
| [Salomon et al., SOME/IP over TSN (arXiv 2504.05793)](https://arxiv.org/html/2504.05793v3) | 2025 | SDN controller re-verifies all CBS bounds on each subscribe and rejects if violated; notes QoS signalling for dynamic services is unaddressed in TSN | yes |
| [Guo et al., IEEJ Trans. C](https://www.jstage.jst.go.jp/article/ieejeiss/128/3/128_3_499/_article/-char/ja) | 2008 | Content-based subscription admission using covering relations — broker load, not network guarantees | abstract |
| [RFC 9064](https://www.rfc-editor.org/rfc/rfc9064) | 2021 | ICN's multi-destination forwarding makes admission "particularly tricky"; no mechanism | yes |
| [Ambalavanan et al., ICN '22](https://www.nitindermohan.com/documents/2022/pubs/reservation_icn_poster.pdf) | 2022 | Per-prefix/per-consumer admission listed as open in NDN | yes |
| [RFC 9320](https://www.rfc-editor.org/rfc/rfc9320.html) | 2022 | Bounds recomputed when a flow is added or removed; paths "are fixed" | yes |

**What is left for Zenoh:** receiver-triggered admission (RSVP/MSRP style) generalised
from exact identifiers to intersecting wildcard key expressions on both sides, where a
new publisher declaration can enlarge many reservations at once; aggregated
declarations in a router mesh hide true fan-out (RFC 3754's problem, defined by the
namespace instead of a multicast tree); reservations that aggregate by key-expression
covering (`video/**` subsuming `video/cam1`).

## C3: reservations when transit nodes move

| Source | Year | Finding | Read |
| --- | --- | --- | --- |
| [RFC 8655](https://www.rfc-editor.org/rfc/rfc8655.html) | 2019 | Explicit routes that do not change on topology events; no mobility | yes |
| [RFC 9912 RAW architecture](https://www.rfc-editor.org/rfc/rfc9912.html) | 2026 | Motion treated as link degradation; recovery-graph computation out of scope | yes |
| [draft-bernardos-detnet-raw-mobility-05](https://datatracker.ietf.org/doc/html/draft-bernardos-detnet-raw-mobility) | 2026 | States DetNet/RAW are "limited to static scenarios"; covers only a moving end host | yes |
| [INSIGNIA](https://datatracker.ietf.org/doc/html/draft-ietf-manet-insignia), [SWAN](https://pdos.csail.mit.edu/archive/decouto/papers/ahn02.pdf), [CEDAR](https://datatracker.ietf.org/doc/html/draft-ietf-manet-cedar-spec) | 1998–2002 | Reservation restore / re-admission / local repair after mobility; all explicitly soft (CEDAR "cannot provide bandwidth guarantees") | yes |
| [3GPP 5GS as TSN bridge](https://www.3gpp.org/news-events/2122-tsn_v_lan), TS 23.501 §5.27.5 | 2020+ | Transit (gNB, UPF) fixed; bridge delay set at PDU-session setup only | yes |
| [Wireless TSN survey (arXiv 2312.01204)](https://arxiv.org/html/2312.01204v3) | 2023–24 | Handover and highly mobile end devices listed as open | yes |

**What is left:** firm, composable per-hop bounds for a flow whose relay moves, and how
the bound holds during repair (make-before-break through old and new relay, PREOF
across them).

## C4: latency of a tokio-based router

| Source | Year | Finding | Read |
| --- | --- | --- | --- |
| [Gallenmüller et al., NOMS](https://www.net.in.tum.de/fileadmin/bibtex/publications/papers/gallenmueller_noms2020.pdf) | 2020 | Tuned DPDK forwarder: 16 µs tail; untuned: ms spikes; ~1000× under overload | yes |
| [Kirdan et al., OPC UA real-time (arXiv 2310.17052)](https://arxiv.org/abs/2310.17052) | 2023 | User-space OPC UA on PREEMPT_RT with taprio/ETF meets many industrial deadlines | yes |
| [Škoudlil et al., async Rust in ROS 2, ECRTS (arXiv 2505.21323)](https://arxiv.org/html/2505.21323v2) | 2025 | Tokio's work stealing "clearly unsuitable for real-time"; bounds only with single-threaded executors on SCHED_FIFO; network I/O not modelled | yes |
| [Corsaro et al., Zenoh (DSD)](https://zenodo.org/records/10635550); [Liang et al. (arXiv 2303.09419)](https://arxiv.org/pdf/2303.09419); [Zhang et al. (arXiv 2309.07496)](https://arxiv.org/html/2309.07496v4) | 2023–24 | Zenoh latency reported as mean/median only | yes |
| [Klüner et al. (arXiv 2505.02734)](https://arxiv.org/html/2505.02734v2) | 2025 | Zenoh max ≈ 3× median, unexplained | yes |
| [Joung et al., C-SCORE, IEEE Access](https://doi.org/10.1109/ACCESS.2023.3318479) | 2023 | Bound equals stateful fair queuing; simulation only | metadata |

**What is left:** the first tail/worst-case characterisation of a Zenoh router, the
effect of real-time tuning on it, and a measured software C-SCORE or TCQF.

## Consequences for the plan

- Phase 1 delivers C4 and the groundwork for C1; the result is publishable either way.
- C1 follows DDS-TSN's design; cite it as the model rather than claiming novelty.
- C2 is the main research contribution; it needs a formal model of key-expression
  intersection and covering before any code.
- C3 is hard and depends on the peer-mesh mode; treat as a stretch goal.
- Segment lists, BIER-TE, anchors, guard bands and layered streams are prior art;
  add each only when a measurement shows the simpler option (Zenoh `transport_weights`,
  one link per deterministic flow, underlay or 5G QoS) is insufficient.
