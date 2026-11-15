//! Publishers, subscribers and the declaration trace.

use std::collections::HashMap;

use rand::{rngs::StdRng, Rng};
use zenoh_keyexpr::OwnedKeyExpr;

use crate::params::Params;

#[derive(Clone)]
pub struct Pub {
    pub ke: OwnedKeyExpr,
    pub router: usize,
    pub rate: f64,
    pub deadline: f64,
    pub prio: u8, // higher = more important
}

#[derive(Clone)]
pub struct Sub {
    pub sel: OwnedKeyExpr, // deterministic selector d_s
    pub router: usize,
    pub budget: f64,
    /// Session face at `router`. Several subscriptions may share a face; the router
    /// then delivers one copy per publisher to that face.
    pub face: usize,
    /// The face is a peer in the peer region hanging off `router`.
    pub peer: bool,
    r: usize,
    v: usize,
}

pub fn is_wild(k: &OwnedKeyExpr) -> bool {
    k.as_str().contains('*')
}

fn ke(s: String) -> OwnedKeyExpr {
    OwnedKeyExpr::new(s).expect("valid key expression")
}

pub fn random_pub(p: &Params, rng: &mut StdRng, routers: usize) -> Pub {
    let r = rng.gen_range(0..p.regions);
    let v = rng.gen_range(0..p.vehicles);
    let router = rng.gen_range(0..routers);
    if rng.gen_bool(0.05) {
        // Wildcard publisher: one stream covering all layers.
        return Pub { ke: ke(format!("fleet/r{r}/v{v}/*")), router, rate: 8e6, deadline: 0.05, prio: 1 };
    }
    if rng.gen_bool(0.5) {
        Pub { ke: ke(format!("fleet/r{r}/v{v}/base")), router, rate: 2e6, deadline: 0.05, prio: 2 }
    } else {
        Pub { ke: ke(format!("fleet/r{r}/v{v}/enh")), router, rate: 6e6, deadline: 0.1, prio: 1 }
    }
}

/// Subscriber class from a uniform draw `x`, using the weights in `p.mix`.
fn sub_class(p: &Params, x: f64, r: usize, v: usize) -> (OwnedKeyExpr, f64) {
    let mut acc = 0.0;
    let mut class = 4;
    for (i, w) in p.mix.iter().enumerate() {
        acc += w;
        if x < acc {
            class = i;
            break;
        }
    }
    let (sel, budget) = match class {
        0 => ("fleet/*/*/base".to_string(), 400e6), // operations centre
        1 => (format!("fleet/r{r}/**"), 100e6),     // region monitor
        2 => (format!("fleet/r{r}/v{v}/**"), 10e6), // nearby peer watching one vehicle
        3 => (format!("fleet/*/v{v}/base"), 5e6),   // vehicle id across regions
        _ => (format!("fleet/r{r}/v{v}/base"), 5e6), // exact stream
    };
    (ke(sel), budget)
}

fn random_sub(p: &Params, rng: &mut StdRng, routers: usize) -> Sub {
    let r = rng.gen_range(0..p.regions);
    let v = rng.gen_range(0..p.vehicles);
    let router = rng.gen_range(0..routers);
    let (sel, budget) = sub_class(p, rng.gen(), r, v);
    Sub { sel, router, budget, face: 0, peer: false, r, v }
}

#[derive(Clone)]
pub enum Event {
    DeclPub(usize, Pub),
    DeclSub(usize, Sub),
    UndeclPub(usize),
    UndeclSub(usize),
}

/// Declarations churning around the target populations: below target a declaration
/// is three times as likely as an undeclaration, above target the reverse.
pub fn trace(p: &Params, routers: usize, rng: &mut StdRng) -> Vec<Event> {
    let (mut pubs, mut subs) = (Vec::<usize>::new(), Vec::<usize>::new());
    let mut live: HashMap<usize, Sub> = HashMap::new();
    let mut next = 0usize;
    let mut out = Vec::with_capacity(p.events);
    while out.len() < p.events {
        let p_decl = |n: usize, target: usize| if n < target { 0.75 } else { 0.25 };
        if rng.gen_bool(0.5) {
            if pubs.is_empty() || rng.gen_bool(p_decl(pubs.len(), p.target_pubs)) {
                next += 1;
                pubs.push(next);
                out.push(Event::DeclPub(next, random_pub(p, rng, routers)));
            } else {
                let i = rng.gen_range(0..pubs.len());
                out.push(Event::UndeclPub(pubs.swap_remove(i)));
            }
        } else if subs.is_empty() || rng.gen_bool(p_decl(subs.len(), p.target_subs)) {
            next += 1;
            let mut s = random_sub(p, rng, routers);
            s.face = next;
            // Draws below only when the feature is on, so defaults keep the old trace.
            if p.shared_face > 0.0 && !subs.is_empty() && rng.gen_bool(p.shared_face) {
                // Same session declares another subscription about the same vehicle,
                // e.g. `fleet/r1/v3/**` next to `fleet/r1/v3/base`.
                let host = &live[&subs[rng.gen_range(0..subs.len())]];
                let (sel, budget) = sub_class(p, rng.gen(), host.r, host.v);
                s = Sub { sel, budget, ..host.clone() };
            } else if p.peer_share > 0.0 && rng.gen_bool(p.peer_share) {
                s.peer = true;
                s.router = rng.gen_range(0..p.peer_routers.min(routers));
            }
            subs.push(next);
            live.insert(next, s.clone());
            out.push(Event::DeclSub(next, s));
        } else {
            let i = rng.gen_range(0..subs.len());
            let id = subs.swap_remove(i);
            live.remove(&id);
            out.push(Event::UndeclSub(id));
        }
    }
    out
}
