// Deterministic Networking in Zenoh — architecture and use cases
// Build: typst compile docs/detnet/architecture.typ
#import "@preview/fletcher:0.5.8" as fletcher: diagram, node, edge

#set document(title: "Deterministic Networking in Zenoh: Architecture and Use Cases")
#set page(paper: "a4", margin: (x: 2cm, y: 2cm), numbering: "1")
#set text(size: 10.5pt)
#set heading(numbering: "1.")
#set par(justify: true)
#show link: set text(fill: rgb("#1f6feb"))

#let accent = rgb("#1f6feb")
#let victim = rgb("#d97706")
#let muted = luma(150)
#let ctrl = (paint: luma(90), dash: "dashed", thickness: 0.7pt)
#let link-style = (paint: luma(170), thickness: 1.2pt)
#let hop(a, b, st, last: false, ..rest) = edge(a, b, if last { "-|>" } else { "-" }, stroke: st, bend: 18deg, ..rest)
#let fstroke = accent + 1.6pt
#let vstroke = (paint: victim, thickness: 1.4pt, dash: "dashed")
#let box-node(pos, body, name, ..args) = node(pos, body, name: name, stroke: 0.6pt, corner-radius: 3pt, inset: 6pt, ..args)

#align(center)[
  #text(size: 17pt, weight: "bold")[Deterministic Networking in Zenoh]\
  #text(size: 12pt)[Architecture and use cases]\
  #v(2pt)
  #text(size: 9pt, fill: luma(100))[Branch `detnet` · companion to `design.md` · draft 2026-11-15]
]

#v(6pt)

*Summary.* Admission and path choice happen at the edge and in a per-region PCE; the
path and scheduling state travel in each message as DetNet-style extensions; transit
routers keep one load counter per link and no per-flow state. Rerouting a flow is
therefore a change at its ingress router only, which is what makes bandwidth
preemption, multi-subscriber trees and mobility cheap.

= Architecture

#figure(
  diagram(
    spacing: (14mm, 12mm),
    node-stroke: 0.6pt,
    node-corner-radius: 3pt,
    node(enclose: ((1, -1), (3, 1)), fill: luma(248), stroke: (paint: luma(200), dash: "dotted"), snap: -1, name: <region>),
    box-node((0, 1), [*Publisher*\ #text(size: 8pt)[declares contract]], <pub>),
    box-node((1, 1), [*Ingress edge*\ #text(size: 8pt)[admit · police · stamp]], <ing>, fill: accent.lighten(85%), stroke: accent),
    box-node((2, 1), [*Transit*\ #text(size: 8pt)[forward by header\ 1 counter / link]], <tr>),
    box-node((3, 1), [*Egress edge*\ #text(size: 8pt)[replicate · eliminate]], <eg>),
    box-node((4, 0.4), [*Subscriber 1*], <s1>),
    box-node((4, 1.6), [*Subscriber 2*], <s2>),
    box-node((2, -0.6), [*PCE* (zenohd plugin)\ #text(size: 8pt)[topology · reservations\ CSPF · preemption]], <pce>, fill: luma(240)),
    edge(<pub>, <ing>, "-|>", stroke: 1.4pt),
    edge(<ing>, <tr>, "-|>", stroke: 1.4pt, [data + ext], label-side: right),
    edge(<tr>, <eg>, "-|>", stroke: 1.4pt),
    edge(<eg>, <s1>, "-|>", stroke: 1.4pt),
    edge(<eg>, <s2>, "-|>", stroke: 1.4pt),
    edge(<ing>, <pce>, "<|-|>", stroke: ctrl, [admit / path], label-side: left, bend: 20deg),
    edge(<tr>, <pce>, "-|>", stroke: ctrl, [usage bands], label-side: right),
    edge(<eg>, <pce>, "-|>", stroke: ctrl, [subscriber set], label-side: right, bend: -20deg),
    edge(<pub>, <pce>, "-|>", stroke: ctrl, [contract token], label-side: left, bend: 30deg),
  ),
  caption: [Roles in one Zenoh region. Solid: data path. Dashed: control. Only the
  ingress edge and the PCE hold per-flow state.],
)

== Roles

#table(
  columns: (auto, 1fr, auto),
  stroke: 0.4pt + luma(200),
  table.header([*Role*], [*Does*], [*Per-flow state*]),
  [Publisher], [Declares an RFC 9016 contract (rate, burst, max packet, deadline, loss, setup/hold priority) as a liveliness token. Adapts encoder bitrate.], [own flows],
  [Ingress edge], [Asks the PCE to admit; polices with a token bucket; stamps path (segment list or BIER-TE bitstring), C-SCORE tag and PREOF sequence number.], [yes, its own flows],
  [Transit], [Forwards by header; schedules by tag; counts bytes per link; floods a usage band only when it crosses a threshold.], [*none*],
  [Egress edge], [Replicates to local subscribers; eliminates duplicates (PREOF).], [sequence window per replicated flow],
  [PCE], [Holds topology, capacities and reservations for its region; computes paths and trees; chooses preemption victims.], [reservation table],
)

== Timescales

#table(
  columns: (auto, 1fr, auto),
  stroke: 0.4pt + luma(200),
  table.header([*Timescale*], [*Mechanism*], [*State holder*]),
  [minutes], [Capacity and usage bands flooded in link-state; trunks resized], [link-state, PCE],
  [seconds], [Admission, preemption, handover re-admission], [PCE, ingress edge],
  [milliseconds], [Policing, scheduling, congestion control, SVC layer dropping], [ingress, endpoints],
)

== Message extensions

All extensions are non-mandatory: a router that does not understand them ignores them
and falls back to declaration-driven Zenoh routing.

#table(
  columns: (auto, auto, auto, auto),
  stroke: 0.4pt + luma(200),
  table.header([*Extension*], [*Standard model*], [*Written by*], [*Phase*]),
  [Segment list (unicast path)], [SR, RFC 8402 / RFC 9256], [ingress], [3],
  [Tree bitstring (multi-subscriber)], [BIER-TE, RFC 9262], [ingress, region gateway], [3],
  [Finish time or cycle ID], [C-SCORE / TCQF drafts], [ingress, updated per hop], [4],
  [Flow sequence number], [DetNet PREOF, RFC 8655 / RFC 8938], [ingress], [5],
)

#pagebreak()

= Use case 1: video stream that needs bandwidth, with preemption

A 4K camera publishes flow *F*: 40 Mb/s, setup priority 2. Every link is
100 Mb/s except R1–R3 (50 Mb/s). Lower-priority flow *V* (30 Mb/s, holding priority 5)
already uses R2–E2, and a high-priority flow W reserves another 50 Mb/s there.
Another flow already holds 20 Mb/s of R1–R3.

#let topo(after: false) = diagram(
  spacing: (13mm, 10mm),
  node-shape: rect,
  node-stroke: 0.6pt,
  node-corner-radius: 3pt,
  node((0, 1), [E1], name: <e1>, fill: accent.lighten(85%)),
  node((1, 1), [R1], name: <r1>),
  node((2, 0), [R2], name: <r2>),
  node((2, 2), [R3], name: <r3>),
  node((3, 1), [E2], name: <e2>),
  node((1, -0.6), [E3], name: <e3>, fill: victim.lighten(80%)),
  edge(<e1>, <r1>, "-", stroke: link-style),
  edge(<r1>, <r2>, "-", stroke: link-style),
  edge(<r1>, <r3>, "-", stroke: link-style, if after [free 0] else [free 30], label-side: right, label-sep: 8pt),
  edge(<r2>, <e2>, "-", stroke: link-style, if after [free 10] else [free 20], label-side: right, label-sep: 8pt),
  edge(<r3>, <e2>, "-", stroke: link-style),
  edge(<e3>, <r2>, "-", stroke: link-style),
  ..if after {(
    hop(<e1>, <r1>, fstroke),
    hop(<r1>, <r2>, fstroke, [F 40], label-side: left),
    hop(<r2>, <e2>, fstroke, last: true),
    hop(<e3>, <r2>, vstroke),
    hop(<r2>, <r1>, vstroke),
    hop(<r1>, <r3>, vstroke, [V 30], label-side: left),
    hop(<r3>, <e2>, vstroke, last: true),
  )} else {(
    hop(<e3>, <r2>, vstroke, [V 30], label-side: left),
    hop(<r2>, <e2>, vstroke, last: true),
  )},
)

#figure(
  grid(columns: 2, gutter: 12mm, align: bottom,
    [#topo(after: false) \ #align(center)[*before:* R2–E2 has 20 free, F needs 40]],
    [#topo(after: true) \ #align(center)[*after:* V moved via R1–R3, F on R2–E2]],
  ),
  caption: [Preemption on R2–E2. V is moved by changing its segment list at E3; no
  transit router is signalled.],
)

*Sequence*

+ Publisher declares F's contract token; ingress E1 asks the PCE for admission.
+ PCE runs CSPF on free capacity (RFC 9320 latency check per candidate path). No
  feasible path: R2–E2 has 20 free, R1–R3–E2 has 30 free.
+ PCE runs preemption selection (RFC 4829 style): only flows whose holding priority is
  strictly lower than F's setup priority; prefer the fewest flows and the least
  bandwidth freed beyond need. Candidate: V on R2–E2.
+ *Make-before-break for the victim.* PCE computes a path for V first:
  E3–R2–R1–R3–E2 (R1–R3 has 30 free). It sends V's ingress E3 the new segment list
  `[R1, R3, E2]` and moves V's reservation. E3 confirms after it starts stamping.
+ Only then does the PCE commit F on E1–R1–R2–E2 and return the path to E1, which
  starts policing and forwarding F.
+ If V had no alternative path: V is degraded to its declared minimum rate (drop SVC
  layers) or demoted to best-effort with `qos_overwrite`, and its publisher is told
  through its contract token's admin-space status.

*Why it is cheap.* With RSVP-TE every router on V's old and new path would update
per-tunnel state. Here V's reroute is one update at E3; transit routers only see
different headers.

*Guards against churn.* Victims are chosen only with strictly lower holding priority, so
a victim cannot preempt back; rerouted flows keep the new path for a hold-down time;
publishers retry after random backoff; admission stops below a high-water band (e.g.
80%) while preemption starts only above 95%.

#pagebreak()

= Use case 2: multiple subscribers

Zenoh delivers a publication to every matching subscriber, so a flow's path is a tree,
not a line. Reservations count once per tree edge: subscribers that share a branch
share its bandwidth.

#figure(
  diagram(
    spacing: (20mm, 9mm),
    node-stroke: 0.6pt,
    node-corner-radius: 3pt,
    node-shape: rect,
    node((0, 1), [Pub], name: <p>),
    node((1, 1), [E1], name: <e1>, fill: accent.lighten(85%)),
    node((2, 1), [R1], name: <r1>),
    node((3, 0), [R2], name: <r2>),
    node((3, 2), [R3], name: <r3>),
    node((4, 0), [S1], name: <s1>),
    node((4, 1.4), [S2], name: <s2>),
    node((4, 2.6), [S3], name: <s3>),
    edge(<p>, <e1>, "-|>", stroke: accent + 1.6pt, [F 40 Mb/s], label-side: left),
    edge(<e1>, <r1>, "-|>", stroke: accent + 1.6pt),
    edge(<r1>, <r2>, "-|>", stroke: accent + 1.6pt, [b1], label-side: left),
    edge(<r1>, <r3>, "-|>", stroke: accent + 1.6pt, [b2], label-side: right),
    edge(<r2>, <s1>, "-|>", stroke: accent + 1.6pt, [b3], label-side: left),
    edge(<r3>, <s2>, "-|>", stroke: accent + 1.6pt, [b4], label-side: left),
    edge(<r3>, <s3>, "-|>", stroke: (paint: muted, dash: "dashed", thickness: 1pt), [b5 (join)], label-side: right),
  ),
  caption: [Tree for one flow; each link carries F once. The ingress stamps the bitstring {b1…b4}; R1 and R3
  replicate by reading it. S3 joining sets b5 — one update at E1.],
)

*Encoding: BIER-TE instead of per-subscriber segment lists.* Per-subscriber segment
lists would make the ingress send one copy per subscriber, which multiplies the load on
the mobile uplink and on E1–R1. BIER-TE (RFC 9262) is the existing standard for
stateless explicit trees: each bit names an adjacency, the ingress sets the bits of
the tree, and each router replicates onto the adjacencies whose bits are set and that
are its own. Transit still holds no per-flow state, only its bit-to-adjacency table,
which is topology, not flows.

*Sequence when a subscriber joins*

+ S3 declares its subscription as usual; the egress side reports the new subscriber
  to the PCE (the PCE already sees declarations through the admin space).
+ PCE extends the tree with the cheapest feasible branch (per-subscriber CSPF, merged
  into the existing tree; a Steiner heuristic, not optimal) and checks S3's RFC 9320
  bound on its own root-to-leaf path.
+ PCE reserves only the new edges (R3–S3) and sends E1 the new bitstring.
+ If no feasible branch exists, S3 is served best-effort by normal Zenoh routing and is
  told so; the deterministic tree is unchanged.

*Per-leaf guarantees.* The latency bound is checked per root-to-leaf path, so a far
subscriber can be refused while near ones are admitted. Subscribers with weaker needs
(e.g. a recorder) can opt out and take the best-effort copy.

*Scale.* Bitstrings have a fixed length (e.g. 256 bits), so trees are encoded per
region; a region gateway (Zenoh Regions, \#2096) re-encodes the bitstring for the next
region, which keeps headers short and matches the hierarchical-routing work upstream.

#pagebreak()

= Use case 3: no flow state in transit, explicit paths only when needed

#figure(
  diagram(
    spacing: (18mm, 9mm),
    node-stroke: 0.6pt,
    node-shape: rect,
    node-corner-radius: 3pt,
    node((0, 1), [E1], name: <e1>, fill: accent.lighten(85%)),
    node((1, 1), [R1], name: <r1>),
    node((2, 0), [R2], name: <r2>),
    node((2, 2), [R3], name: <r3>),
    node((3, 1), [E2], name: <e2>),
    edge(<e1>, <r1>, "-", stroke: link-style),
    edge(<r1>, <r2>, "-", stroke: link-style),
    edge(<r2>, <e2>, "-", stroke: link-style, [band >95%], label-side: right),
    edge(<r1>, <r3>, "-", stroke: link-style),
    edge(<r3>, <e2>, "-", stroke: link-style),
    hop(<r1>, <r2>, (paint: muted, dash: "dashed", thickness: 1pt), [shortest: no extension], label-side: left),
    hop(<r2>, <e2>, (paint: muted, dash: "dashed", thickness: 1pt), last: true),
    edge(<e1>, <r1>, "-", stroke: fstroke),
    hop(<r1>, <r3>, fstroke, [segments `[R3, E2]`], label-side: right, bend: -18deg),
    hop(<r3>, <e2>, fstroke, last: true, bend: -18deg),
  ),
  caption: [The ingress adds a segment list only when the admitted path differs from the
  shortest path. Loose segments name only waypoints.],
)

- *Default is plain Zenoh.* A flow whose admitted path equals the shortest path
  carries no path extension; routers forward as today.
- *Loose segments.* When the PCE's path differs, the ingress stamps only the waypoints
  (`[R3, E2]`): between waypoints each router uses its normal shortest path, as in SR
  with node segments. Strict hop lists are used only where shortest paths between
  waypoints would cross the overloaded link (use case 1's victim path).
- *What transit keeps.* One byte counter per outgoing link and its current band. No
  table of flows, no reservation per flow, nothing to time out or clean up when a flow
  ends.
- *Where per-flow state lives.* The PCE (reservations), the ingress edge (policer,
  path to stamp) and the publisher (contract token). When a flow ends, its liveliness
  token disappears and both release it.
- *Safety without transit state.* Admission keeps the sum of reservations under
  capacity; the ingress policer keeps each flow inside its contract; transit counters
  catch what the PCE cannot see (unmanaged traffic, hidden shared bottlenecks) and raise
  the band, which feeds back into admission.

#table(
  columns: (auto, 1fr, 1fr),
  stroke: 0.4pt + luma(200),
  table.header([], [*RSVP-TE style*], [*This design (SR / BIER-TE style)*]),
  [Transit state], [per tunnel, refreshed], [per link counter only],
  [Reroute], [signal every hop of old and new path], [update one ingress],
  [Failure of a transit router], [state must be rebuilt], [nothing to rebuild; PCE recomputes paths],
  [Cost], [—], [bigger headers; PCE must know capacity],
)

#pagebreak()

= Use case 4: mobility

== Publisher handover

#figure(
  diagram(
    spacing: (14mm, 9mm),
    node-stroke: 0.6pt,
    node-shape: rect,
    node-corner-radius: 3pt,
    node((0, 1), [*Mobile publisher*\ #text(size: 8pt)[Wi-Fi + cellular]], name: <mp>),
    node((1, 0), [Wi-Fi AP\ → E1], name: <e1>, fill: accent.lighten(85%)),
    node((1, 2), [Cell\ → E3], name: <e3>, fill: accent.lighten(85%)),
    node((2, 1), [R1], name: <r1>),
    node((3, 1), [*Egress E2*\ #text(size: 8pt)[eliminate duplicates]], name: <e2>),
    node((4, 1), [Sub], name: <s>),
    edge(<mp>, <e1>, "-|>", stroke: accent + 1.4pt, [copy A, seq n], label-side: left),
    edge(<mp>, <e3>, "-|>", stroke: (paint: accent, thickness: 1.4pt, dash: "dashed"), [copy B, seq n], label-side: right),
    edge(<e1>, <r1>, "-|>", stroke: accent + 1.4pt),
    edge(<e3>, <r1>, "-|>", stroke: (paint: accent, thickness: 1.4pt, dash: "dashed")),
    edge(<r1>, <e2>, "-|>", stroke: accent + 1.4pt),
    edge(<e2>, <s>, "-|>", stroke: accent + 1.4pt),
  ),
  caption: [Handover from Wi-Fi to cellular, make-before-break. During the overlap both
  copies carry the same sequence number (PREOF) and E2 forwards the first one.],
)

+ *Detect.* The publisher's link layer or signal strength predicts a handover; the
  publisher opens (or already holds) a session through the second access.
+ *Re-admit first.* The contract token is re-announced on the new session; the new
  ingress E3 asks the PCE to admit F from E3. Only the access hop and the path from
  E3 change; the rest of the tree is reused.
+ *Overlap.* The publisher sends both copies with the same PREOF sequence number; E2
  eliminates duplicates, so the subscriber sees no gap and no duplicate.
+ *Release.* When the old access is gone or idle for a hold-down time, E1 drops its
  policer and the PCE releases the reservation on the old access path.
+ *Session continuity.* QUIC connection migration (quinn) keeps the Zenoh session over
  an IP change, so the contract token does not flap when only the address changes.

== Subscriber moves

The egress side reports the new attachment; the PCE adds the new leaf before removing
the old one (both bits set during the overlap) and sends the ingress the new
bitstring. The publisher and the rest of the tree are not affected.

== Access capacity changes

A cellular uplink varies faster than any reservation can follow. The contract declares
a *minimum* and a *target* rate:

- admission reserves the minimum on every link, including the access hop as estimated
  by congestion control (GCC, SCReAM, or quinn's RTT/cwnd stats);
- between minimum and target the flow uses spare capacity on its priority lane;
- when the access estimate drops, the encoder drops SVC or simulcast layers first; a
  reroute or re-admission happens only after degradation lasts seconds, with random
  backoff.

== Why mobility stays local

Handovers happen every few seconds or minutes per device. Each one touches the PCE and
one or two ingress edges, never transit routers, and changes link-state only if a band
is crossed. Global flooding stays on the minutes timescale.

#pagebreak()

= Use case 5: many mobile publishers, many subscribers

The cases above combine. A fleet of mobile peers (vehicles, drones, body cameras)
each publishes live video, and each stream has several subscribers: a cloud operations
centre that watches every stream, a recorder, and peers nearby that watch a few. The
design must hold when all of this happens at once.

*Working scenario.* 200 publishers on 6 edge routers (cells or access points), one
core region, a cloud site. Each stream has a 2 Mb/s base layer and 6 Mb/s of
enhancement layers. Each publisher hands over about once a minute, so the system sees
about 3 handovers per second. The numbers only size the problems below; they are
assumptions.

== What breaks if the earlier cases are simply added up

#table(
  columns: (auto, 1fr, auto),
  stroke: 0.4pt + luma(200),
  table.header([*Problem*], [*Why*], [*Answer*]),
  [Tree churn], [Each handover moves the root of a source-rooted tree, so every branch is recomputed: 3 handovers/s across 200 trees], [anchored trees],
  [Handover storms], [Vehicles cross the same cell boundary together; the new edge sees a burst of re-admissions], [guard bandwidth],
  [Uplink cost of PREOF], [Duplicating 8 Mb/s over Wi-Fi and cellular doubles the scarcest link], [replicate base layer only],
  [Subscriber fan-in], [The operations centre's access link carries the sum of 200 streams: 400 Mb/s of base layers alone], [subscriber budget],
  [Heterogeneous subscribers], [A nearby peer on Wi-Fi and the recorder in the cloud need different rates], [layers as flows],
  [Hairpinning], [A nearby subscriber on the same edge would receive the stream through the anchor], [local breakout],
  [PCE load], [Every event going through one PCE], [edge-local admission],
)

== Anchored trees: separate mobility from fan-out

#figure(
  diagram(
    spacing: (20mm, 10mm),
    node-stroke: 0.6pt,
    node-corner-radius: 3pt,
    node-shape: rect,
    node((0, 0), [V1], name: <v1>),
    node((0, 1.2), [V2], name: <v2>),
    node((1, 0), [E1], name: <e1>, fill: accent.lighten(85%)),
    node((1, 1.2), [E2], name: <e2>, fill: accent.lighten(85%)),
    node((2.2, 0.6), [*Anchor A*\ #text(size: 8pt)[tree root for V1, V2]], name: <a>, fill: luma(240)),
    node((3.4, 0.6), [Core], name: <c>),
    node((4.6, 0), [Ops centre], name: <ops>),
    node((4.6, 1.2), [Recorder], name: <rec>),
    node((1, 2.4), [Peer P], name: <p>),
    edge(<v1>, <e1>, "-|>", stroke: fstroke),
    edge(<v2>, <e2>, "-|>", stroke: fstroke),
    edge(<v2>, <e1>, "-|>", stroke: (paint: accent, thickness: 1.2pt, dash: "dashed"), [handover], label-side: left),
    edge(<e1>, <a>, "-|>", stroke: fstroke),
    edge(<e2>, <a>, "-|>", stroke: fstroke, [unicast segments], label-side: right),
    edge(<a>, <c>, "-|>", stroke: fstroke, [BIER-TE], label-side: right),
    edge(<c>, <ops>, "-|>", stroke: fstroke),
    edge(<c>, <rec>, "-|>", stroke: fstroke),
    edge(<e2>, <p>, "-|>", stroke: (paint: victim, thickness: 1.4pt), [local breakout], label-side: right),
  ),
  caption: [Mobility changes only the unicast segment from the edge to the anchor. The
  tree from the anchor to the subscribers is stable. A subscriber on the publisher's own
  edge is served there directly.],
)

- *Anchor.* Each publisher's tree is rooted at an anchor router in the core (or the
  region gateway), not at the publisher. The ingress edge sends one copy to the anchor
  over a unicast segment list; the anchor stamps the BIER-TE bitstring. This is the
  rendezvous point of PIM-SM and the mobility anchor of Proxy Mobile IPv6 and 3GPP
  user-plane functions, applied to Zenoh trees.
- *Handover is cheap.* A handover changes only the edge-to-anchor segment: the new edge
  is admitted for one unicast path and the anchor's tree is untouched. 3 handovers/s
  become 3 unicast admissions/s, not 3 tree recomputations.
- *Anchor relocation.* If a publisher moves far from its anchor and the stretch passes a
  threshold, the PCE builds a tree at a closer anchor, runs both with PREOF during the
  overlap, then removes the old one, like PIM's switch from the shared tree to a
  shortest-path tree.
- *Local breakout.* The ingress edge already holds the publisher's flow state, so it
  also replicates to subscribers on the same edge. They skip the anchor round trip,
  which matters most for nearby peers with tight deadlines.
- *State.* Anchors hold per-publisher tree state (one bitstring per stream); they act
  as edges of the core. Transit routers still hold none.

== Handover storms: guard bandwidth

Cellular networks solved this with guard channels: part of each cell's capacity
accepts only handovers, never new calls, because dropping a moving call is worse than
refusing a new one.

- Each edge keeps a guard band G of its uplink and edge-to-anchor capacity. New streams
  are admitted only while reservations stay below C − G; handovers may use all of C.
- G is sized from the expected handover rate into that edge and the per-stream base
  rate; the PCE resizes it on the minutes timescale from observed mobility.
- *Predicted handover.* When a publisher's signal to its edge degrades, the neighbour
  edge makes a soft, time-limited reservation for it, so the re-admission at handover
  cannot fail for lack of capacity.
- When a burst still exceeds G, lower-priority streams arrive at the new edge with
  their base layer only before any stream is refused.

== Layers as separate flows

Each stream is published as layers on separate key expressions (e.g.
`video/<id>/base`, `video/<id>/enh1`), each with its own contract.

#table(
  columns: (auto, auto, auto, 1fr),
  stroke: 0.4pt + luma(200),
  table.header([*Layer*], [*Contract*], [*Priority*], [*Treatment*]),
  [Base, 2 Mb/s], [deterministic: rate, deadline, loss], [`RealTime`], [Reserved end to end; PREOF over both accesses during handover; never preempted by enhancement traffic],
  [Enhancement, 6 Mb/s], [elastic: target rate], [`DataHigh`], [Uses spare capacity; first to be demoted or dropped under preemption, a storm or a weak uplink],
)

- *Uplink.* PREOF duplicates only the base layer during handover: 2 extra Mb/s on the
  second access instead of 8.
- *Heterogeneous subscribers.* Each subscriber takes the layers it can carry: the
  operations centre takes the base layers of all 200 streams and the enhancement layers
  of the few being watched; a nearby peer on Wi-Fi takes base plus one enhancement
  layer. Trees differ per layer: receiver-driven layered multicast (McCanne, Jacobson
  and Vetterli, 1996) on top of BIER-TE.
- *Preemption order.* Enhancement layers go before any base layer of equal or higher
  priority, so preemption lowers quality before it cuts video.

== Subscriber fan-in

A subscriber with a wildcard key expression (`video/**`) matches every stream, and its
own access link becomes the bottleneck.

- *Deterministic service is opt-in per subscriber and stream.* A wildcard subscription
  gets best-effort delivery by default. The subscriber declares a receive contract: a
  budget (e.g. 500 Mb/s) and which streams or layers get deterministic service.
- *Admission at both ends.* A branch to a subscriber is admitted only if the stream's
  path and the subscriber's budget both have room; the subscriber's access link is
  checked like any other link.
- *Changing what is watched.* When an operator opens a stream, the operations centre
  adds that stream's enhancement layers to its receive contract and the PCE adds
  branches for those layers only.

== Who handles which event

#table(
  columns: (auto, auto, auto, 1fr),
  stroke: 0.4pt + luma(200),
  table.header([*Event*], [*Rate*], [*Handled by*], [*Work*]),
  [Handover], [≈3/s], [new edge, PCE], [one edge-to-anchor admission against the guard band; no tree change],
  [Layer up/down at a subscriber], [≈1/s], [PCE], [add or remove one branch of one layer tree],
  [New stream], [rare], [PCE], [pick an anchor, build trees for subscribed layers],
  [Anchor relocation], [rare], [PCE], [new tree in parallel, PREOF overlap, remove old],
  [Uplink fluctuation], [continuous], [publisher, edge], [drop enhancement layers; no PCE work],
  [Guard band resize], [minutes], [PCE], [from observed handover rates per edge],
)

At these rates (assumed, not measured) one PCE per region is enough; a region gateway
forwards only inter-region tree changes.

= What is new

Almost every mechanism above is prior art: segment routing, BIER-TE, PCE, RSVP-TE
preemption, PIM-SM anchors, guard channels, layered multicast and PREOF. The
contributions are narrower; `prior-art.md` lists the sources.

#table(
  columns: (auto, 1fr, auto),
  stroke: 0.4pt + luma(200),
  table.header([*Claim*], [*Contribution*], [*Nearest prior art*]),
  [C1], [Per-key-expression stream identity visible to DetNet/TSN, reservations driven by declarations (engineering)], [OMG DDS-TSN; zenoh-pico raweth key→MAC/VLAN],
  [C2], [Admission when publisher and subscriber declarations are both wildcard key expressions, over a mesh that aggregates declarations (research)], [RSVP, MSRP/RAP, IndiQoS, SOME/IP-TSN (exact IDs only)],
  [C3], [Firm bounds for flows whose transit node is mobile, including during repair (research, stretch)], [INSIGNIA, SWAN, CEDAR (soft only); DetNet/RAW (static)],
  [C4], [Tail latency of a tokio-based Zenoh router; software C-SCORE/TCQF measured (empirical)], [DPDK and PREEMPT_RT studies; no Zenoh tail data],
)

The protocol extensions in use cases 1–5 are optional: each is added only when a
measurement shows that Zenoh `transport_weights`, one link per deterministic flow, or
underlay/5G QoS is not enough.

= Open questions

- *Anchor placement:* one anchor per publisher chosen by load, or a fixed anchor per
  edge group? How large a stretch triggers relocation?
- *Guard band sizing:* from which mobility model, and per edge or per edge pair?
- *Receive contracts:* how a subscriber with a wildcard key expression names the
  streams it wants deterministic service for, without a wire change.
- *PCE placement:* one per region, with gateways asking the next region's PCE, or one
  per deployment for the many-to-one cloud-upload case?
- *Tree encoding:* BIER-TE bitstring vs per-subscriber segment lists for small fan-out
  (2–3 subscribers), where a list may be shorter than a bitstring.
- *Victim notification:* contract-token status in the admin space vs a dedicated
  control message to the victim's ingress.
- *Handover prediction:* who triggers re-admission — the publisher, its ingress, or the
  PCE seeing the second access?
- *Hidden shared bottlenecks:* two overlay links over the same physical link; how the
  PCE learns shared-risk groups.

= References

- RFC 8655 DetNet architecture; RFC 9016 flow and service information model; RFC 9320
  bounded latency; RFC 8938 data plane framework.
- RFC 8402 segment routing; RFC 9256 SR Policy; RFC 4829 TE LSP preemption policies.
- RFC 8279 BIER; RFC 9262 BIER-TE.
- RFC 7761 PIM-SM (rendezvous point, shortest-path-tree switchover); RFC 5213 Proxy
  Mobile IPv6 (mobility anchor).
- S. McCanne, V. Jacobson, M. Vetterli, "Receiver-driven layered multicast", SIGCOMM 1996.
- Guard-channel handover admission in cellular networks (Hong and Rappaport, 1986).
- draft-ietf-detnet-stateless-fair-queuing (C-SCORE); draft-ietf-detnet-tcqf (TCQF).
- Zenoh Regions and gateways, eclipse-zenoh/zenoh \#2096.

#text(size: 8.5pt, fill: luma(100))[RFC numbers and titles are from memory and earlier
notes and still need checking against the RFC Editor.]
