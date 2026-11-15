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

= Open questions

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
- draft-ietf-detnet-stateless-fair-queuing (C-SCORE); draft-ietf-detnet-tcqf (TCQF).
- Zenoh Regions and gateways, eclipse-zenoh/zenoh \#2096.

#text(size: 8.5pt, fill: luma(100))[RFC numbers and titles are from memory and earlier
notes and still need checking against the RFC Editor.]
