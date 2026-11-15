# Track D: Mobility (claim C3, stretch)

**Status:** not started · **Depends on:** tracks A and C

Keep guarantees across handover. DetNet and RAW are limited to static scenarios; MANET
QoS (INSIGNIA, SWAN, CEDAR) handles moving nodes with soft guarantees only
([prior-art.md](prior-art.md), C3).

## Scope

Upstream `main` has no link-state peer mode, so mobile *transit* nodes arise only if
peer relaying returns. D therefore covers **edge handover inside a router region**,
with peers and clients as leaves:

- a publisher or subscriber container moves its link from one edge router to another
  (emulated in containerlab);
- re-admission at the new edge before the old path is released (make-before-break);
- guard bandwidth at edges for incoming handovers;
- base-layer-only duplication during the overlap, if gate G4 shows loss.

Design background: use cases 4 and 5 in [architecture.typ](architecture.typ) (anchored
trees, guard bandwidth, layers as separate flows).

## Questions

- Does re-admission at the new edge fit inside the handover gap at emulated rates?
- How large a guard band keeps handover rejection below a target under correlated
  mobility?
- Is the deadline bound kept during the overlap?
