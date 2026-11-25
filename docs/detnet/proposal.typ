// Deterministic Networking in Zenoh — research proposal
// Build: typst compile docs/detnet/proposal.typ
#import "@preview/fletcher:0.5.8" as fletcher: diagram, node, edge
#import "@preview/cetz:0.3.4"

#set document(title: "Deterministic Networking in Zenoh: Research Proposal")
#set page(paper: "a4", margin: (x: 2cm, y: 2cm), numbering: "1")
#set text(size: 10.5pt)
#set heading(numbering: "1.")
#set par(justify: true)
#show link: set text(fill: rgb("#1f6feb"))
#show figure.caption: set text(size: 9pt)

#let accent = rgb("#1f6feb")
#let warn = rgb("#d97706")
#let good = rgb("#16a34a")
#let bad = rgb("#dc2626")
#let muted = luma(120)
#let ctrl = (paint: luma(90), dash: "dashed", thickness: 0.7pt)
#let bx(pos, body, name, ..args) = node(pos, body, name: name, stroke: 0.6pt, corner-radius: 3pt, inset: 6pt, ..args)
#let small(body) = text(size: 8pt, body)

#align(center)[
  #text(size: 18pt, weight: "bold")[Deterministic Networking in Zenoh]\
  #v(1pt)
  #text(size: 12pt)[Research proposal and progress report]\
  #v(3pt)
  #text(size: 9pt, fill: muted)[Branch `detnet` of NEWSLabNTU/zenoh · 2026-11-25]
]

#v(6pt)

#block(fill: luma(246), inset: 9pt, radius: 3pt, width: 100%)[
  *Summary.* Zenoh routes publications by key expression but gives no bandwidth,
  latency or loss guarantee. We bring existing deterministic-networking standards
  (IETF DetNet, IEEE TSN, OMG DDS-TSN) to Zenoh instead of inventing new mechanisms.
  A literature check showed that almost every mechanism is prior art, so the
  contribution is narrowed to four claims: per-key-expression stream identity (C1,
  engineering), admission control when both publishers and subscribers use wildcard
  key expressions (C2, main research claim), firm bounds with mobile transit nodes
  (C3, stretch) and the first tail-latency characterisation of a Zenoh router (C4,
  empirical). First results: a configuration-only "deterministic lane" keeps the
  probe's p99 at 4--6 ms under 120 % overload, where stock Zenoh reaches 0.5 s; and
  wildcard admission admits 70 % of eligible flows against 2.6 % for exact-identifier
  admission, but only 1--2 points more than an application that re-expands its
  wildcards often.
]

= Problem

Zenoh is a pub/sub, query and storage protocol used in robotics (ROS 2 `rmw_zenoh`),
vehicles and edge systems. Routers forward each publication along per-source trees to
every router holding a matching subscription. Its quality-of-service knobs are
priorities, express sending, congestion control (block or drop) and, since 1.0,
multiple links per session split by priority range with a DSCP per endpoint. None of
these give a *guarantee*: nothing limits how much traffic enters, and nothing checks
that a link can carry what has been promised to the subscribers behind it.

Deterministic networking standards solve this for IP and Ethernet: DetNet (RFC 8655,
9016, 9320) defines flows, contracts and bounded-latency calculus; TSN defines
reservations (802.1Qat/Qcc), shapers (Qav) and time-aware gates (Qbv); OMG DDS-TSN
maps DDS topics onto TSN streams. The goal is to give Zenoh the same three properties
--- reserved bandwidth, bounded latency, bounded loss --- for flows that declare a
contract, while best-effort traffic keeps working unchanged.

= Positioning and claims

#figure(
  table(
    columns: (auto, 2.2fr, 1.3fr, auto),
    align: (center, left, left, left),
    stroke: 0.4pt + luma(200),
    inset: 5pt,
    table.header([*\#*], [*Claim*], [*Prior art*], [*Type*]),
    [C1], [Per-key-expression stream identity visible to DetNet/TSN, created and removed by Zenoh declarations], [Solved for DDS by DDS-TSN §8.3.1], [Engineering],
    [C2], [Admission when publisher *and* subscriber declarations are wildcard key expressions, so the receiver set is an intersection any new declaration can enlarge; expanded exactly and immediately inside the router mesh], [RSVP, SRP/RAP handle dynamic membership for exact identifiers only], [Research (main)],
    [C3], [Firm, composable bounds when a transit node moves, including during repair], [MANET QoS: soft only; RAW: static], [Research (stretch)],
    [C4], [Measured tail and worst-case latency of a tokio-based Zenoh router, with and without tuning], [Zenoh papers report mean/median only], [Empirical],
  ),
  caption: [Contribution claims after the literature check (`prior-art.md`). Dropped:
    "topic-to-stream mapping is new", "a Zenoh session is one undifferentiated
    connection", "dynamic-membership admission is open".],
)

= Architecture

Work stays out of tree: a `zenohd` admission plugin, an ingress interceptor and
configuration, with only small generic changes proposed upstream as pull requests.
Decisions are split by timescale so that fast ones use exact local state.

#figure(
  diagram(
    spacing: (13mm, 11mm),
    node-stroke: 0.6pt,
    node-corner-radius: 3pt,
    node(enclose: ((1, 0.6), (3, 1.4)), fill: luma(248), stroke: (paint: luma(190), dash: "dotted"), snap: -1),
    bx((0, 1), [*Publisher*\ #small[RFC 9016 contract\ on key expression]], <pub>),
    bx((1, 1), [*Ingress router*\ #small[token-bucket policer\ (interceptor)]], <ing>, fill: accent.lighten(85%), stroke: accent),
    bx((2, 1), [*Transit*\ #small[normal Zenoh trees\ 1 load counter / link]], <tr>),
    bx((3, 1), [*Egress router*], <eg>),
    bx((4, 0.45), [*Subscriber*\ #small[receive contract]], <s1>),
    bx((4, 1.55), [*Subscriber*\ #small[best effort]], <s2>),
    bx((2, -0.35), [*Admission plugin* (zenohd)\ #small[declarations #sym.arrow eligible pairs by\ key-expression intersection; admit\ against capacity, deadline, budget]], <adm>, fill: luma(238)),
    edge(<pub>, <ing>, "-|>", stroke: 1.4pt),
    edge(<ing>, <tr>, "-|>", stroke: 1.4pt),
    edge(<tr>, <eg>, "-|>", stroke: 1.4pt),
    edge(<eg>, <s1>, "-|>", stroke: 1.4pt),
    edge(<eg>, <s2>, "-|>", stroke: 1.4pt),
    edge(<pub>, <adm>, "-|>", stroke: ctrl, [contract token], label-side: left, bend: 25deg),
    edge(<s1>, <adm>, "-|>", stroke: ctrl, [receive contract], label-side: right, bend: -25deg),
    edge(<adm>, <ing>, "-|>", stroke: ctrl, [admit / police], label-side: right),
    edge(<tr>, <adm>, "-|>", stroke: ctrl, [link stats], label-side: right),
  ),
  caption: [Roles. Solid: data; dashed: control, carried as liveliness tokens and
    admin-space keys (`@detnet/<zid>/...`), so no wire change is needed.],
) <fig-arch>

#figure(
  table(
    columns: (auto, 1fr, auto),
    stroke: 0.4pt + luma(200),
    inset: 5pt,
    table.header([*Timescale*], [*Mechanism*], [*State held by*]),
    [Minutes], [Link capacities and statistics, guard bands], [Admission plugin],
    [Seconds], [Admission per declaration event, handover re-admission], [Admission plugin, ingress],
    [Milliseconds], [Policing, scheduling, congestion control], [Ingress router, endpoints],
  ),
  caption: [Timescale split.],
)

Three guarantee tiers follow from who owns the links: *hard* (owned TSN or DetNet
underlay, Zenoh maps flows onto it --- C1), *firm* (owned Zenoh routers over measured
links, admission plus router scheduling --- needs gate G1) and *statistical* (shared
or wireless links, admission plus loss budget only).

= C2: admission for wildcard receiver sets

A publisher on `fleet/car1/video` and a subscriber on `fleet/*/video` form an
_eligible pair_ because their key expressions intersect. Both sides may be wildcards,
and a router mesh aggregates subscriptions, so the set of receivers behind a link is
neither declared nor fixed: one new publisher declaration can enlarge many
reservations at once (@fig-ke).

#figure(
  diagram(
    spacing: (30mm, 7mm),
    node-stroke: 0.6pt,
    node-corner-radius: 3pt,
    bx((0, 0), [`fleet/car1/video`], <p1>),
    bx((0, 1), [`fleet/car2/video`], <p2>),
    bx((0, 2), [`fleet/car1/lidar`], <p3>),
    bx((0, 3), [`fleet/car3/video` #text(fill: warn)[(new)]], <p4>, stroke: warn),
    bx((1, 0.5), [`fleet/*/video`\ #small[operator console]], <s1>, fill: accent.lighten(88%)),
    bx((1, 2), [`fleet/car1/**`\ #small[car1 digital twin]], <s2>, fill: accent.lighten(88%)),
    edge(<p1>, <s1>, "-|>"),
    edge(<p2>, <s1>, "-|>"),
    edge(<p1>, <s2>, "-|>"),
    edge(<p3>, <s2>, "-|>"),
    edge(<p4>, <s1>, "-|>", stroke: (paint: warn, dash: "dashed", thickness: 1.2pt)),
  ),
  caption: [Eligible pairs are intersections of wildcard declarations. The new
    publisher on the left joins the console's receiver set without any subscriber
    action; admission must charge its stream on every link of its tree.],
) <fig-ke>

*Model* (`admission-model.typ`). Each admitted publisher stream is charged once per
edge of its distribution tree (not once per subscriber), subject to capacity with a
guard fraction, a per-path deadline and a per-subscriber receive budget. Admitting a
new publisher is polynomial; admitting a new subscriber (choose which of its matches
to accept under a budget) is a knapsack and NP-hard; preemption is a
multi-dimensional knapsack. Two aggregation lemmas say when a router can summarise
its subtree with a bit (bandwidth) and a minimum slack (deadline).

*Simulator* (`tools/detnet-sim`). One router region, 200 publishers and 300
subscribers churning around those targets, five seeds per point.

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *
    let data = (
      ([no admission], 100, bad, [91 % of admitted violated]),
      ([exact identifiers], 2.6, muted, []),
      ([snapshot, never refreshed], 35.9, warn, []),
      ([snapshot, refresh / 1000 ev.], 53.8, warn, []),
      ([snapshot, refresh / 100 ev.], 68.6, warn, []),
      ([wildcard (C2)], 70.0, accent, []),
    )
    let w = 0.09
    for (i, (label, v, c, note)) in data.enumerate() {
      let y = -i * 0.62
      content((-0.2, y), anchor: "east", text(size: 8.5pt, label))
      rect((0, y - 0.22), (v * w, y + 0.22), fill: c.lighten(30%), stroke: none)
      content((v * w + 0.12, y), anchor: "west", text(size: 8.5pt)[#v %  #text(fill: bad, note)])
    }
    line((0, 0.4), (0, -5 * 0.62 - 0.4), stroke: 0.5pt)
  }),
  caption: [Share of eligible pairs admitted (no violations except "no admission").
    _Snapshot_ is the strongest baseline: the application expands its wildcards into
    exact reservations and refreshes them every N events.],
) <fig-adm>

#figure(
  table(
    columns: (auto, 1fr),
    stroke: 0.4pt + luma(200),
    inset: 5pt,
    table.header([*Experiment*], [*Result (5 seeds)*]),
    [Mix and topology sweeps], [Wildcard 59--76 % vs exact identifiers 0.4--24 % everywhere],
    [Charging rules for overlapping subscriptions], [Shared rules admit $<=$ 1 point more than per-pair but force \~500 evictions per run; per-pair never evicts],
    [Preemption (RFC 4829 style)], [High priority 91 #sym.arrow 96 %, low priority 35 #sym.arrow 33 %; 80 % of evicted pairs re-admitted],
    [Peer regions, one copy per face vs relaying], [Peer subscribers 20 % vs 32 % admitted --- the largest single loss found],
    [Shortest-path vs capacity-aware routing], [+2 to +5 points for 4× event cost],
  ),
  caption: [Simulator extensions (track C, step C2).],
)

*Gates.* *G2* passes in a narrowed form: wildcard admission beats exact identifiers
everywhere, but beats a frequently refreshed snapshot by only 1--2 points. C2 is
therefore claimed as _exact, immediate expansion inside the network_ (no refresh
logic, correct under declaration aggregation), and its value now rests on signalling
and reaction-time cost, which is the next experiment. *G3* does not trigger:
capacity-aware routing does not justify explicit paths or BIER-TE trees.

= C4: tail latency of a Zenoh router

*Testbed* (`tools/detnet-lab`). containerlab on a 4-core host with a stock kernel;
results compare configurations on the same host, not absolute bounds.

#figure(
  diagram(
    spacing: (16mm, 8mm),
    node-stroke: 0.6pt,
    node-corner-radius: 3pt,
    bx((0, 0), [`pub`\ #small[probe 8.2 Mb/s]], <pub>, fill: accent.lighten(85%)),
    bx((0, 1), [`xpub`\ #small[cross 0--120 %]], <xpub>),
    bx((1, 0.5), [`r1`], <r1>),
    bx((2, 0.5), [`r2`], <r2>),
    bx((3.6, 0.5), [`r3`], <r3>),
    bx((4.6, 0), [`sub`\ #small[one-way latency]], <sub>, fill: accent.lighten(85%)),
    bx((4.6, 1), [`xsub`], <xsub>),
    edge(<pub>, <r1>, "-"),
    edge(<xpub>, <r1>, "-"),
    edge(<r1>, <r2>, "-", stroke: 1.2pt),
    edge(<r2>, <r3>, "-|>", stroke: 1.6pt + warn, text(size: 8pt)[netem + tbf\ 100 Mb/s], label-side: left, label-sep: 6pt),
    edge(<r3>, <sub>, "-"),
    edge(<r3>, <xsub>, "-"),
  ),
  caption: [Three `zenohd` routers in a line; endpoints are clients of their edge
    router. The probe carries `[seq][send time]`; all containers share the host
    clock, so arrival minus send time is one-way latency.],
)

*Profiles.* `stock`: probe and cross traffic share priority Data, blocking.
`prio`: probe on RealTime, express, drop. `lane`: as `prio`, plus a dedicated TCP
link per hop for priority 1 marked DSCP EF and a priority qdisc on the bottleneck
(@fig-lane). The lane is configuration only:
`tcp/<next>:7447?prio=1-1#dscp=0xb8` next to `tcp/<next>:7447?prio=2-7`.

#figure(
  diagram(
    spacing: (14mm, 7mm),
    node-stroke: 0.6pt,
    node-corner-radius: 3pt,
    bx((0, 0.5), [`r2`\ #small[Zenoh tx]], <r2>),
    bx((1.4, 0), [TCP link `prio=1-1`\ #small[TOS 0xb8 (EF)]], <l1>, fill: accent.lighten(85%), stroke: accent),
    bx((1.4, 1), [TCP link `prio=2-7`], <l2>),
    bx((3, 0.5), [`netem` #sym.arrow `tbf` #sym.arrow `prio`\ #small[band 1: EF · band 2: rest]], <q>, fill: luma(240)),
    bx((4.4, 0.5), [`r3`], <r3>),
    edge(<r2>, <l1>, "-|>", stroke: accent + 1.4pt),
    edge(<r2>, <l2>, "-|>"),
    edge(<l1>, <q>, "-|>", stroke: accent + 1.4pt),
    edge(<l2>, <q>, "-|>"),
    edge(<q>, <r3>, "-|>", stroke: 1.4pt),
  ),
  caption: [Deterministic lane: separate socket buffers per class, and the kernel
    queue serves EF first. Follows DDS-TSN §8.3.1 (distinct 5-tuple per stream).],
) <fig-lane>

#figure(
  cetz.canvas(length: 1cm, {
    import cetz.draw: *
    let loads = (0, 50, 75, 85, 90, 100, 120)
    let series = (
      ("stock", muted, (7.01, 1.42, 1.34, 3.28, 477, 479, 618)),
      ("prio", warn, (0.57, 0.98, 3.41, 1.20, 255, 1025, 364)),
      ("lane", accent, (5.50, 7.85, 6.91, 4.44, 3.83, 5.06, 6.14)),
    )
    let X(l) = l / 120 * 11
    let Y(v) = (calc.log(v, base: 10) + 1) * 1.3
    // axes and grid
    for e in (-1, 0, 1, 2, 3) {
      let y = (e + 1) * 1.3
      line((0, y), (11, y), stroke: 0.3pt + luma(215))
      content((-0.15, y), anchor: "east", text(size: 8pt)[#calc.pow(10.0, e) ms])
    }
    for l in loads {
      content((X(l), -0.3), text(size: 8pt)[#l])
    }
    content((5.5, -0.8), text(size: 8.5pt)[cross-traffic load, % of bottleneck])
    line((0, 0), (11, 0), stroke: 0.5pt)
    line((0, 0), (0, 5.4), stroke: 0.5pt)
    for (i, (name, c, vals)) in series.enumerate() {
      let pts = loads.zip(vals).map(((l, v)) => (X(l), Y(v)))
      line(..pts, stroke: 1.4pt + c)
      for p in pts { circle(p, radius: 0.07, fill: c, stroke: none) }
      content((0.4, 5.1 - i * 0.4), anchor: "west", text(size: 8.5pt, fill: c, weight: "bold", name))
    }
    content((X(105), Y(1025) + 0.25), text(size: 8pt, fill: warn)[60 % loss])
    content((X(117), Y(364) - 0.3), text(size: 8pt, fill: warn)[90 % loss])
  }),
  caption: [Probe p99 one-way latency, median of 3 runs, log scale. Above 85 % load
    the bottleneck saturates: stock and prio queue for hundreds of ms (prio also
    drops), the lane stays at 4--6 ms. Below saturation the run was disturbed by an
    unrelated build on the host (load average \~10), so the low-load tails and the
    lane's 1--8 % loss are to be re-measured.],
) <fig-a1>

*Preliminary gate G1.* Under load, the tail is queueing --- but in the shared TCP
socket buffer and kernel queue, below Zenoh's priority queues, where Zenoh priority
cannot help. Splitting links per class fixes it without a scheduler change inside
Zenoh, which argues against a software C-SCORE/TCQF (A3, E3) for this case. The
lane's residual loss is attributed to drop congestion control (`wait_before_drop` =
1 ms turns CPU stalls into drops); a lane with blocking congestion control will test
this.

= Plan

#figure(
  diagram(
    spacing: (9mm, 9mm),
    node-stroke: 0.6pt,
    node-corner-radius: 3pt,
    bx((0, 0), [*A* Measure (C4)\ #small[A0 done · A1 partial]], <a>, fill: good.lighten(80%)),
    bx((0, 1.2), [*C* Admit (C2)\ #small[model, sim done]], <c>, fill: good.lighten(80%)),
    bx((0, 2.4), [*B* Map (C1)\ #small[not started]], <b>),
    node((1.6, 0), [*G1*\ #small[queueing?]], shape: fletcher.shapes.diamond, stroke: 0.6pt, name: <g1>, fill: warn.lighten(75%)),
    node((1.6, 1.2), [*G2* #text(fill: good)[✓]\ #small[narrowed]], shape: fletcher.shapes.diamond, stroke: 0.6pt, name: <g2>),
    node((1.6, 2.4), [*G3* #text(fill: bad)[✗]], shape: fletcher.shapes.diamond, stroke: 0.6pt, name: <g3>),
    bx((3.2, 0), [A2 tuning\ #small[cpuset, workers]], <a2>),
    bx((3.2, 1.2), [C4 admission plugin\ #small[in testbed, with B2]], <plug>, fill: accent.lighten(85%), stroke: accent),
    bx((3.2, 2.4), [E1/E2 explicit paths\ #small[parked]], <e12>, stroke: (paint: muted, dash: "dashed")),
    bx((4.8, 0), [E3 scheduling tags\ #small[likely not needed]], <e3>, stroke: (paint: muted, dash: "dashed")),
    bx((4.8, 1.2), [D mobility (C3)\ #small[candidate to drop]], <d>, stroke: (paint: muted, dash: "dashed")),
    bx((4.8, 2.4), [E4 replication\ #small[gate G4]], <e4>, stroke: (paint: muted, dash: "dashed")),
    edge(<a>, <g1>, "-|>"),
    edge(<c>, <g2>, "-|>"),
    edge(<c>, <g3>, "-|>"),
    edge(<g1>, <a2>, "-|>"),
    edge(<g1>, <e3>, "-|>", bend: 30deg, stroke: (dash: "dashed")),
    edge(<g2>, <plug>, "-|>"),
    edge(<b>, <plug>, "-|>"),
    edge(<g3>, <e12>, "-|>", stroke: (dash: "dashed")),
    edge(<plug>, <d>, "-|>", stroke: (dash: "dashed")),
  ),
  caption: [Tracks and gates. Green: done or under way; dashed: gated or likely dropped.],
)

#figure(
  table(
    columns: (auto, 1fr, auto),
    stroke: 0.4pt + luma(200),
    inset: 5pt,
    table.header([*Order*], [*Work*], [*Decides*]),
    [1], [Re-run A1 on a quiet host (stock, prio, lane, lane with blocking); A2 cpuset pinning and tokio worker settings], [G1; C4 report],
    [2], [Simulator: signalling and reaction-time cost of snapshot refresh vs in-network wildcard expansion], [Whether C2 holds as a systems claim],
    [3], [B1 flow and receive contracts (liveliness tokens, admin space); B2 key expression #sym.arrow lane #sym.arrow DSCP], [C1],
    [4], [C4 admission plugin with interceptor policing, validated in the testbed against the simulator], [C2 end to end],
    [5], [Small upstream PRs: link statistics in admin space, per-link `SO_PRIORITY` (\#2881)], [---],
  ),
  caption: [Next steps. Free research, no deadline; ordered by risk and value.],
)

*Alternative if C2 weakens further.* The peer-region result (one copy per peer face
costs 40 % of peer admissions) points to replication inside peer regions as a
research topic of its own; it is Zenoh architecture work rather than a standard port.

= Risks

- *Host noise.* Four shared cores and a non-real-time kernel limit what latency
  results can claim; only relative comparisons are reported, and each run records
  the host load average.
- *C2 novelty.* Rests on signalling and correctness arguments now that the admitted
  share barely beats a refreshed snapshot.
- *Upstream.* No contact with ZettaScale; everything must work out of tree, and PRs
  must be small and generic.
- *Simulator fidelity.* Greedy admission, one region, no aggregated client
  subscriptions or peer publishers yet.

= Key references

RFC 8655 DetNet architecture · RFC 9016 flow information model · RFC 9320 bounded
latency · RFC 8939 DetNet IP data plane · RFC 2205 RSVP · RFC 3754 neglected
reservation subtree · RFC 4829 preemption · RFC 9912 RAW · IEEE 802.1Qat/Qcc/Qbv ·
OMG DDS-TSN 1.0 · Škoudlil et al., ECRTS 2025 (async Rust real-time) ·
Gallenmüller et al., NOMS 2020 (software forwarding tails). Full list with read
status: `docs/detnet/prior-art.md`.
