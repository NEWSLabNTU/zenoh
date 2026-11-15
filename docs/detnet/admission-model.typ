// C2: admission for wildcard-defined receiver sets — formal model (draft)
// Build: typst compile docs/detnet/admission-model.typ
#set document(title: "Admission for Wildcard-Defined Receiver Sets")
#set page(paper: "a4", margin: (x: 2.2cm, y: 2cm), numbering: "1")
#set text(size: 10.5pt)
#set heading(numbering: "1.")
#set par(justify: true)
#set math.equation(numbering: "(1)")

#let ke(x) = raw(x)
#let sem(x) = $⟦ #x ⟧$
#let intersects = math.op("intersects")
#let includes = math.op("includes")

#align(center)[
  #text(size: 16pt, weight: "bold")[Admission for Wildcard-Defined Receiver Sets]\
  #text(size: 11pt)[Formal model for contribution C2 (draft)]\
  #v(2pt)
  #text(size: 9pt, fill: luma(100))[Branch `detnet` · companion to `prior-art.md` · 2026-11-15]
]

#v(4pt)

*Goal.* State precisely what is new in admission control for Zenoh, where the receivers
of a stream are whatever subscriptions currently intersect its key expression. Prior
art (RSVP, MSRP/RAP, IndiQoS, SOME/IP-over-TSN) admits receivers of *exact*
identifiers; RFC 9320 recomputes bounds per flow over *fixed* paths. This note defines
the model, proves two locality properties that make admission compatible with Zenoh's
declaration aggregation, and lists the open problems that form the research claim.

= Key expressions

Let $Sigma$ be the set of chunks (non-empty strings without `/` or wildcards) and
$cal(U) = Sigma^+$ the set of concrete keys (`/`-separated chunk sequences). A key
expression $k$ is a chunk-sequence pattern where `*` matches exactly one chunk, `**`
matches zero or more chunks, and `$*` matches any substring inside a chunk. Its
semantics is the key set it denotes:

$ sem(k) subset.eq cal(U). $

Zenoh implements the two relations the model needs (`keyexpr::intersects` and
`keyexpr::includes` in `commons/zenoh-keyexpr`):

$ a intersects b <==> sem(a) inter sem(b) != emptyset, quad
  a includes b <==> sem(b) subset.eq sem(a). $

Both are decidable for this pattern language (it is a restricted regular language);
Zenoh's implementation is the reference. Verbatim chunks (`@`) only restrict matching
and do not change the model.

= Network, declarations and contracts

*Network.* A directed graph $G = (V, E)$ of Zenoh routers and peers with link
capacity $c_e$ (b/s) and, for each link, a per-hop delay bound function
$delta_e (alpha)$ that maps an arrival curve $alpha$ to a delay bound, as in RFC 9320.

*Publisher declarations.* $p = (k_p, v_p, sigma_p)$: key expression, attachment node,
and contract $sigma_p = (r_p, b_p, D_p, epsilon_p, pi_p)$ — rate, burst, deadline, loss
budget, priority (RFC 9016 fields). $k_p$ may contain wildcards: Zenoh delivers a
publication on #ke("test/*") to a subscriber on #ke("test/cache")
(`routing-semantics.md`, finding 1).

*Subscriber declarations.* $s = (k_s, d_s, v_s, beta_s)$: subscription key expression
$k_s$; deterministic selector $d_s$ with $k_s includes d_s$ (which part of the
subscription asks for guaranteed delivery; $d_s = emptyset$ means best effort only);
attachment node $v_s$; receive budget $beta_s$ (b/s). The network may see a wider
declaration $hat(k)_s supset.eq k_s$ when the session aggregates subscriptions
(finding 7); delivery follows $hat(k)_s$, so $d_s$ must be signalled separately and
never aggregated, or admission would reserve for traffic the application drops.

*Matching.* With publisher set $P$ and subscriber set $S$, delivery and eligibility
are

$ M = {(p, s) in P times S : k_p intersects k_s}, quad
  M^det = {(p, s) in M : k_p intersects d_s}. $ <eq-match>

$M$ is what Zenoh delivers today (best effort). $M^det$ is the set of pairs that *may*
be admitted. The difference from prior art is that $M^det$ is not a list of
identifiers chosen by anyone: it is induced by two families of wildcard patterns and
changes whenever any declaration appears or disappears.

= Admission state and constraints

The admission state is a set $A subset.eq M^det$ of admitted pairs. For each publisher,
routing gives a path $"path"(p, s)$ from $v_p$ to $v_s$; the delivery tree is

$ T_p = union.big_({s : (p, s) in A}) "path"(p, s). $

In a Zenoh router region the paths are the source router's shortest-path tree, so
$T_p$ is a tree and $"path"(p,s)$ is fixed by link weights (findings 3–4); across
regions the tree is re-rooted at each region's entry router (finding 5). In a peer
region there is no relaying: the publisher sends one copy per subscribing peer
(finding 6), so $T_p$ is a star and its access link carries $r_p$ once per peer.

A state $A$ is *feasible* when three constraints hold.

/ Link capacity: each publisher is carried once per tree edge, whatever its fan-out
  below that edge:
  $ L_e = sum_(p in P) r_p dot bb(1)[e in T_p] <= (1 - g_e) c_e quad forall e in E, $ <eq-cap>
  where $g_e$ is a guard fraction (handover reserve, use case 5).
/ Deadline: each admitted pair meets its publisher's deadline along its own path,
  $ sum_(e in "path"(p, s)) delta_e (alpha_(p, e)) <= D_p quad forall (p, s) in A, $ <eq-lat>
  where $alpha_(p,e)$ is $p$'s arrival curve at $e$ (equal to the source curve under
  stateless fair queuing such as C-SCORE; growing with upstream burst under FIFO).
/ Receive budget: a subscriber is not sent more guaranteed traffic than it accepts,
  $ sum_({p : (p, s) in A}) r_p <= beta_s quad forall s in S. $ <eq-budget>

= Events

Every declaration event changes $M^det$ and asks for a new feasible $A'$:

#table(
  columns: (auto, 1fr),
  stroke: 0.4pt + luma(200),
  table.header([*Event*], [*New candidate pairs*]),
  [declare subscriber $s$], [${(p, s) : k_p intersects d_s}$ — all intersecting publishers at once],
  [declare publisher $p$], [${(p, s) : k_p intersects d_s}$ — *all intersecting subscribers at once*],
  [undeclare $p$ or $s$], [none; pairs leave $A$, capacity is released],
  [subscriber widens $d_s$], [pairs for publishers newly covered by $d_s$],
)

The second row is the case prior art does not have: in RSVP or MSRP a new talker or
sender creates one stream that receivers then join one by one. Here a single publisher
declaration on #ke("video/cam7") instantly makes every subscriber with
$d_s in {#ke("video/**"), #ke("*/cam7"), #ke("**")}$ a candidate, each with its own
path, deadline and budget.

*Admission policy for one event.* Keep $A$ (no disruption) or allow preemption by
priority, and choose $A' supset.eq A$ among the new candidates to maximise admitted
priority weight subject to @eq-cap–@eq-budget. With routing fixed and no preemption:

- *New publisher.* Each edge either has $r_p$ spare or not, deadlines are per path and
  budgets per subscriber, so the constraints decouple: admitting every candidate whose
  path fits, whose deadline holds and whose budget has room is optimal and polynomial
  (one pass over the paths, edges shared by earlier admitted paths cost nothing more).
- *New subscriber.* Many publishers compete for one budget $beta_s$ and for shared
  edges toward $v_s$: choosing the best subset is a knapsack problem, NP-hard; greedy
  by priority then rate is the baseline.
- *Preemption or simultaneous events.* Several publishers compete for the same edges:
  multi-dimensional knapsack, NP-hard; greedy with RFC 4829-style victim selection.

= Locality under declaration aggregation

Inside a router region nothing is hidden at router granularity: every router learns
which routers hold matching subscribers (finding 4). Fan-out is hidden only at the
edge: a session may aggregate its subscriptions into one wider declaration
(finding 7), and a router may answer a client's interest with aggregated
declarations (finding 8). The question is what admission needs to see through that.

*Lemma 1 (bandwidth needs one bit per face).* For edge $e = (u, w)$ and publisher $p$,
the load term in @eq-cap is $r_p dot bb(1)[exists s : (p, s) in A, "path"(p,s) in.rev e]$.
So the upstream router $u$ needs only the boolean "at least one admitted subscriber of
$p$ lies behind $w$". Booleans aggregate by OR, so they survive aggregation of
declarations.

_Proof sketch._ Tree delivery sends one copy of each message over each tree edge;
$e in T_p$ iff some admitted path uses $e$. $square$

*Lemma 2 (deadlines need one number per face, if per-hop bounds do not depend on the
path).* If $delta_e$ depends only on $p$'s source contract (true for C-SCORE and other
rate-based stateless schedulers, whose per-hop bound uses the reserved rate, not the
upstream burst), let the residual slack behind face $w$ be

$ rho_(p,w) = min_({s : (p,s) in A, "path"(p,s) in.rev w}) (D_p - sum_(e in "path"(p,s), e "after" w) delta_e). $

Then the deadline constraint for all subscribers behind $w$ holds iff the delay
accumulated up to $w$ is at most $rho_(p, w)$. Slack aggregates by $min$.

_Proof sketch._ Path delay splits into the prefix up to $w$ and the suffix after it;
the prefix is shared by all subscribers behind $w$. $square$

*Consequence.* Under rate-based scheduling, admission composes hop by hop with a
summary of two values per (publisher, face) — an OR-bit and a min-slack — exactly the
shape of information that aggregated declarations can carry upstream. Hidden fan-out
then matters only for (i) receive budgets, which are local to the subscriber's
attachment router; (ii) replication cost at branch routers; (iii) choosing preemption
victims, which needs per-pair priorities. Under FIFO scheduling Lemma 2 fails because
$alpha_(p,e)$ grows with the upstream path, which is one formal reason to prefer
stateless fair queuing for C2.

= Envelope reservations by key-expression covering

RSVP's wildcard-filter style reserves for senders not yet known. The Zenoh analogue is
an *envelope*: a reservation $R(d, e)$ of bandwidth on edge $e$ for all publishers
matching a selector $d$. A new publisher $p$ with $k_p intersects d$ is admitted against
the envelope locally, without a PCE round trip — the key-expression form of the
edge-to-cloud trunks in the architecture.

Envelopes raise the problem that makes covering non-trivial: they overlap. For
#ke("video/**") and #ke("*/cam7"), publisher #ke("video/cam7") matches both.

- *Charging rule.* Each publisher must be charged to exactly one envelope per edge, or
  capacity is double-counted (over-reservation) or under-counted (over-admission).
  Candidates: most specific envelope (smallest $sem(d)$ that includes $k_p$, which
  needs a total order the `includes` relation does not give when neither includes the
  other); first declared; or split.
- *Partial overlap.* If $k_p$ intersects $d$ without $d includes k_p$ (a wildcard
  publisher only partly inside the envelope), the fraction of $p$'s traffic that
  falls in the envelope is unknown from declarations alone.
- *Monotonicity.* Widening $d_s$ or adding an envelope must never silently invalidate
  admitted pairs; a charging rule should be stable under such changes.

= Open problems (the research claim)

+ *Incremental admission per declaration event*, especially publisher declarations
  that create many candidate pairs at once: algorithm, approximation ratio of the
  greedy baseline, and cost per event at Zenoh's declaration rates.
+ *Aggregation-compatible summaries*: prove Lemmas 1–2 formally for Zenoh's actual
  declaration propagation (including interest-based declaration filtering), and find
  the weakest scheduling assumption under which per-face summaries suffice.
+ *Envelope charging*: a sound, stable charging rule for overlapping key-expression
  envelopes, and the admission loss it causes compared with exact per-pair admission.
+ *Wildcard publishers*: semantics and accounting when $k_p$ contains wildcards
  (now confirmed to exist; see Section 2).
+ *Churn*: soft-state refresh versus explicit undeclare, and stability under
  subscriber flapping (hold-down, hysteresis), extending RSVP soft state to
  declarations.

= Next steps

+ Write a simulator over Zenoh's own `keyexpr` crate: random topologies, declaration
  traces, greedy admission, envelope charging rules; measure admitted fraction and
  per-event cost.
+ Formalise Lemmas 1–2; then decide whether the admission plugin (Phase 1) keeps exact
  per-pair state or per-face summaries.

#text(size: 8.5pt, fill: luma(100))[Draft model. Prior art and sources: `prior-art.md`.
Lemma proofs are sketches.]
