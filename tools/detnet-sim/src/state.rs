//! Admission state: reservations, charging, routing, preemption.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::time::Instant;

use zenoh_keyexpr::OwnedKeyExpr;

use crate::params::{Charging, Params, PeerMode, Routing};
use crate::topology::{Topology, PKT_BITS};
use crate::workload::{is_wild, Event, Pub, Sub};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Policy {
    /// No admission; every eligible pair is delivered, violations counted.
    None,
    /// Prior art (RSVP/MSRP): only a concrete selector equal to the publisher's key.
    ExactId,
    /// Prior art with application-side expansion: a wildcard subscription is expanded
    /// into exact reservations for the publishers that exist when it is declared and
    /// at every refresh (`--refresh`), like RSVP soft-state refresh.
    Snapshot,
    /// C2: any pair with `k_p intersects d_s`, admitted on every declaration event.
    Wildcard,
}

impl Policy {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "none" => Policy::None,
            "exactid" => Policy::ExactId,
            "snapshot" => Policy::Snapshot,
            "wildcard" => Policy::Wildcard,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Policy::None => "none",
            Policy::ExactId => "exactid",
            Policy::Snapshot => "snapshot",
            Policy::Wildcard => "wildcard",
        }
    }
}

pub type Pair = (usize, usize); // (publisher id, subscriber id)

/// Why a pair does not fit; preemption uses it to pick victims.
enum Block {
    Delay,
    NoPath,
    Link(usize),
    Access(usize), // peer-region access segment at this router
    Budget(Vec<usize>),
}

const EPS: f64 = 1e-6;
const ACCESS_PROP: f64 = 0.0005;

pub struct State<'a> {
    pub policy: Policy,
    p: &'a Params,
    pub topo: &'a Topology,
    pub pubs: HashMap<usize, Pub>,
    pub subs: HashMap<usize, Sub>,
    /// Admitted pairs and the link path each one uses.
    pub admitted: BTreeMap<Pair, Vec<usize>>,
    pending: BTreeSet<Pair>, // eligible, admissible, rejected; retried after releases
    expanded: HashSet<Pair>, // snapshot policy: pairs known to the subscriber
    refc: HashMap<(usize, usize), u32>, // (publisher, link) -> admitted paths using it
    tree_in: HashMap<(usize, usize), usize>, // (publisher, node) -> incoming tree link
    pub load: Vec<f64>,
    acc_refc: HashMap<(usize, usize, usize), u32>, // (publisher, router, face or MAX)
    pub acc_load: Vec<f64>,                        // per router's peer region
    group: HashMap<(usize, usize), Vec<usize>>, // (publisher, face) -> admitted subscribers
    pub used: HashMap<usize, f64>,              // subscriber -> charged rate
    evicted: HashSet<Pair>,
    // metrics
    pub fanout: Vec<usize>,
    pub event_ns: Vec<u128>,
    pub preempted: usize,
    pub readmitted: usize,
    pub forced: usize,
}

fn specificity(k: &OwnedKeyExpr, id: usize) -> (usize, usize, Reverse<usize>, usize) {
    let chunks: Vec<&str> = k.as_str().split('/').collect();
    let dsw = chunks.iter().filter(|c| **c == "**").count();
    let sw = chunks.iter().filter(|c| c.contains('*') && **c != "**").count();
    (dsw, sw, Reverse(chunks.len()), id)
}

impl<'a> State<'a> {
    pub fn new(policy: Policy, p: &'a Params, topo: &'a Topology) -> Self {
        State {
            policy,
            p,
            topo,
            pubs: HashMap::new(),
            subs: HashMap::new(),
            admitted: BTreeMap::new(),
            pending: BTreeSet::new(),
            expanded: HashSet::new(),
            refc: HashMap::new(),
            tree_in: HashMap::new(),
            load: vec![0.0; topo.links.len()],
            acc_refc: HashMap::new(),
            acc_load: vec![0.0; topo.n],
            group: HashMap::new(),
            used: HashMap::new(),
            evicted: HashSet::new(),
            fanout: vec![],
            event_ns: vec![],
            preempted: 0,
            readmitted: 0,
            forced: 0,
        }
    }

    // ---------------------------------------------------------------- helpers

    pub fn limit(&self, l: usize) -> f64 {
        (1.0 - self.p.guard) * self.topo.links[l].cap
    }

    pub fn acc_limit(&self) -> f64 {
        (1.0 - self.p.guard) * self.p.peer_cap
    }

    fn acc_key(&self, pi: usize, s: &Sub) -> (usize, usize, usize) {
        match self.p.peer_mode {
            PeerMode::Star => (pi, s.router, s.face),
            PeerMode::Tree => (pi, s.router, usize::MAX),
        }
    }

    fn refc_of(&self, pi: usize, l: usize) -> u32 {
        self.refc.get(&(pi, l)).copied().unwrap_or(0)
    }

    fn admissible(&self, (pi, si): Pair) -> bool {
        let (p, s) = (&self.pubs[&pi], &self.subs[&si]);
        let exact = !is_wild(&s.sel) && !is_wild(&p.ke) && p.ke == s.sel;
        match self.policy {
            Policy::None | Policy::Wildcard => true,
            Policy::ExactId => exact,
            Policy::Snapshot => exact || self.expanded.contains(&(pi, si)),
        }
    }

    pub fn delay(&self, p: &Pub, s: &Sub, path: &[usize]) -> f64 {
        let mut d: f64 = path.iter().map(|&l| self.topo.hop_delay(l, p.rate)).sum();
        if s.peer {
            d += PKT_BITS / p.rate + PKT_BITS / self.p.peer_cap + ACCESS_PROP;
        }
        d
    }

    // --------------------------------------------------------------- charging

    /// Charge per member when publisher `pi` is delivered once to a face whose
    /// admitted subscriptions are `members`.
    fn charges(&self, pi: usize, members: &[usize]) -> Vec<(usize, f64)> {
        if members.is_empty() {
            return vec![];
        }
        let rate = self.pubs[&pi].rate;
        let one_payer = |payer: usize| members.iter().map(|&m| (m, if m == payer { rate } else { 0.0 })).collect();
        match self.p.charging {
            Charging::PerPair => members.iter().map(|&m| (m, rate)).collect(),
            Charging::Split => {
                let share = rate / members.len() as f64;
                members.iter().map(|&m| (m, share)).collect()
            }
            Charging::FirstDeclared => one_payer(*members.iter().min().unwrap()),
            Charging::MostSpecific => {
                one_payer(*members.iter().min_by_key(|&&m| specificity(&self.subs[&m].sel, m)).unwrap())
            }
        }
    }

    fn set_group(&mut self, key: (usize, usize), new: Vec<usize>) {
        let old = self.group.get(&key).cloned().unwrap_or_default();
        for (m, c) in self.charges(key.0, &old) {
            *self.used.entry(m).or_insert(0.0) -= c;
        }
        for (m, c) in self.charges(key.0, &new) {
            *self.used.entry(m).or_insert(0.0) += c;
        }
        if new.is_empty() {
            self.group.remove(&key);
        } else {
            self.group.insert(key, new);
        }
    }

    /// Subscribers whose budget would overflow if `c` joined its face group.
    fn budget_block(&self, (pi, si): Pair) -> Option<Vec<usize>> {
        let key = (pi, self.subs[&si].face);
        let old = self.group.get(&key).cloned().unwrap_or_default();
        if self.p.charging == Charging::PerPair || old.is_empty() {
            // Only the joining subscriber's charge changes, by the full rate.
            let u = self.used.get(&si).copied().unwrap_or(0.0);
            return (u + self.pubs[&pi].rate > self.subs[&si].budget + EPS).then(|| vec![si]);
        }
        let mut new = old.clone();
        new.push(si);
        let oc: HashMap<usize, f64> = self.charges(pi, &old).into_iter().collect();
        let over: Vec<usize> = self
            .charges(pi, &new)
            .into_iter()
            .filter(|&(m, c)| {
                let u = self.used.get(&m).copied().unwrap_or(0.0) - oc.get(&m).copied().unwrap_or(0.0);
                u + c > self.subs[&m].budget + EPS
            })
            .map(|(m, _)| m)
            .collect();
        (!over.is_empty()).then_some(over)
    }

    // ---------------------------------------------------------------- routing

    fn in_tree(&self, pi: usize, v: usize) -> bool {
        v == self.pubs[&pi].router || self.tree_in.contains_key(&(pi, v))
    }

    fn tree_walk(&self, pi: usize, mut v: usize) -> Vec<usize> {
        let root = self.pubs[&pi].router;
        let mut out = vec![];
        while v != root {
            let l = self.tree_in[&(pi, v)];
            out.push(l);
            v = self.topo.links[l].from;
        }
        out.reverse();
        out
    }

    /// Capacity-aware branch: Dijkstra from every node of the publisher's current
    /// tree to `dst`, over links with residual capacity, never re-entering the tree
    /// (so the result stays a tree). Cost is load-sensitive, or delay when `by_delay`.
    fn branch(&self, pi: usize, dst: usize, by_delay: bool) -> Option<Vec<usize>> {
        if self.in_tree(pi, dst) {
            return Some(self.tree_walk(pi, dst));
        }
        let rate = self.pubs[&pi].rate;
        let n = self.topo.n;
        let mut dist = vec![f64::INFINITY; n];
        let mut prev: Vec<Option<usize>> = vec![None; n];
        let mut done = vec![false; n];
        for v in 0..n {
            if self.in_tree(pi, v) {
                dist[v] = if by_delay {
                    self.tree_walk(pi, v).iter().map(|&l| self.topo.hop_delay(l, rate)).sum()
                } else {
                    0.0
                };
            }
        }
        loop {
            let u = (0..n)
                .filter(|&v| !done[v] && dist[v].is_finite())
                .min_by(|&a, &b| dist[a].total_cmp(&dist[b]))?;
            if u == dst {
                break;
            }
            done[u] = true;
            for &(v, l) in &self.topo.adj[u] {
                if done[v] || self.in_tree(pi, v) || self.load[l] + rate > self.limit(l) {
                    continue;
                }
                let w = if by_delay {
                    self.topo.hop_delay(l, rate)
                } else {
                    1.0 + 2.0 * self.load[l] / self.limit(l)
                };
                if dist[u] + w < dist[v] {
                    dist[v] = dist[u] + w;
                    prev[v] = Some(l);
                }
            }
        }
        let mut branch = vec![];
        let mut v = dst;
        while let Some(l) = prev[v] {
            branch.push(l);
            v = self.topo.links[l].from;
        }
        branch.reverse();
        let mut path = self.tree_walk(pi, v);
        path.extend(branch);
        Some(path)
    }

    fn route(&self, (pi, si): Pair) -> Result<Vec<usize>, Block> {
        let (p, s) = (&self.pubs[&pi], &self.subs[&si]);
        if self.policy == Policy::None || self.p.routing != Routing::Capacity {
            return Ok(self.topo.path(p.router, s.router));
        }
        for by_delay in [false, true] {
            if let Some(path) = self.branch(pi, s.router, by_delay) {
                if self.delay(p, s, &path) <= p.deadline {
                    return Ok(path);
                }
            }
        }
        // Nothing fits: name a saturated link on the default path for preemption.
        let spt = self.topo.path(p.router, s.router);
        let sat = spt.into_iter().find(|&l| self.refc_of(pi, l) == 0 && self.load[l] + p.rate > self.limit(l));
        Err(sat.map_or(Block::NoPath, Block::Link))
    }

    /// Path for `c` if it fits all constraints now.
    fn plan(&self, c: Pair) -> Result<Vec<usize>, Block> {
        if self.policy == Policy::None {
            return self.route(c);
        }
        // Cheapest check first: budgets often run out before links do.
        if let Some(over) = self.budget_block(c) {
            return Err(Block::Budget(over));
        }
        let (p, s) = (&self.pubs[&c.0], &self.subs[&c.1]);
        let path = self.route(c)?;
        if self.delay(p, s, &path) > p.deadline {
            return Err(Block::Delay);
        }
        if let Some(&l) = path.iter().find(|&&l| self.refc_of(c.0, l) == 0 && self.load[l] + p.rate > self.limit(l)) {
            return Err(Block::Link(l));
        }
        if s.peer
            && !self.acc_refc.contains_key(&self.acc_key(c.0, s))
            && self.acc_load[s.router] + p.rate > self.acc_limit()
        {
            return Err(Block::Access(s.router));
        }
        Ok(path)
    }

    // ------------------------------------------------------ reserve / release

    fn admit_raw(&mut self, c: Pair, path: Vec<usize>) {
        let (pi, si) = c;
        let rate = self.pubs[&pi].rate;
        for &l in &path {
            let r = self.refc.entry((pi, l)).or_insert(0);
            *r += 1;
            if *r == 1 {
                self.load[l] += rate;
                self.tree_in.insert((pi, self.topo.links[l].to), l);
            }
        }
        let s = &self.subs[&si];
        let (face, router, peer, ak) = (s.face, s.router, s.peer, self.acc_key(pi, s));
        if peer {
            let r = self.acc_refc.entry(ak).or_insert(0);
            *r += 1;
            if *r == 1 {
                self.acc_load[router] += rate;
            }
        }
        let mut g = self.group.get(&(pi, face)).cloned().unwrap_or_default();
        g.push(si);
        self.set_group((pi, face), g);
        self.admitted.insert(c, path);
        if self.evicted.remove(&c) {
            self.readmitted += 1;
        }
    }

    /// Undo `admit_raw` without re-checking other budgets.
    fn release_raw(&mut self, c: Pair) -> Option<Vec<usize>> {
        let path = self.admitted.remove(&c)?;
        let (pi, si) = c;
        let rate = self.pubs[&pi].rate;
        for &l in &path {
            let r = self.refc.get_mut(&(pi, l)).unwrap();
            *r -= 1;
            if *r == 0 {
                self.refc.remove(&(pi, l));
                self.load[l] -= rate;
                self.tree_in.remove(&(pi, self.topo.links[l].to));
            }
        }
        let s = &self.subs[&si];
        let (face, router, peer, ak) = (s.face, s.router, s.peer, self.acc_key(pi, s));
        if peer {
            let r = self.acc_refc.get_mut(&ak).unwrap();
            *r -= 1;
            if *r == 0 {
                self.acc_refc.remove(&ak);
                self.acc_load[router] -= rate;
            }
        }
        let mut g = self.group[&(pi, face)].clone();
        g.retain(|&m| m != si);
        self.set_group((pi, face), g);
        Some(path)
    }

    /// After a group shrinks, charges can move to members that cannot pay
    /// (first-declared, split); release those pairs into pending.
    fn enforce(&mut self, keys: &[(usize, usize)]) {
        if self.policy == Policy::None {
            return;
        }
        for &key in keys {
            loop {
                let over = self.group.get(&key).and_then(|g| {
                    g.iter().copied().find(|m| self.used[m] > self.subs[m].budget + EPS)
                });
                let Some(m) = over else { break };
                self.release_raw((key.0, m));
                self.pending.insert((key.0, m));
                self.forced += 1;
            }
        }
    }

    // ------------------------------------------------------------- preemption

    /// RFC 4829-style victims: lowest priority first, then largest rate.
    fn victims(&self, b: &Block, prio: u8) -> Option<Vec<Pair>> {
        let lower = |q: usize| self.pubs[&q].prio < prio;
        let order = |a: &usize, b: &usize| {
            let (pa, pb) = (&self.pubs[a], &self.pubs[b]);
            pa.prio.cmp(&pb.prio).then(pb.rate.total_cmp(&pa.rate)).then(a.cmp(b))
        };
        let vs: Vec<Pair> = match b {
            Block::Delay | Block::NoPath => return None,
            Block::Link(l) => {
                let q = self.refc.keys().filter(|k| k.1 == *l && lower(k.0)).map(|k| k.0).min_by(order)?;
                self.admitted.iter().filter(|(c, path)| c.0 == q && path.contains(l)).map(|(c, _)| *c).collect()
            }
            Block::Access(r) => {
                let q = self.acc_refc.keys().filter(|k| k.1 == *r && lower(k.0)).map(|k| k.0).min_by(order)?;
                self.admitted
                    .keys()
                    .filter(|c| c.0 == q && self.subs[&c.1].peer && self.subs[&c.1].router == *r)
                    .copied()
                    .collect()
            }
            Block::Budget(over) => {
                let c = self
                    .admitted
                    .keys()
                    .filter(|c| over.contains(&c.1) && lower(c.0))
                    .min_by(|a, b| order(&a.0, &b.0).then(a.cmp(b)))?;
                vec![*c]
            }
        };
        (!vs.is_empty()).then_some(vs)
    }

    fn try_preempt(&mut self, c: Pair) -> bool {
        let prio = self.pubs[&c.0].prio;
        let mut out: Vec<(Pair, Vec<usize>)> = vec![];
        loop {
            match self.plan(c) {
                Ok(path) => {
                    self.admit_raw(c, path);
                    let keys: Vec<_> = out.iter().map(|(v, _)| (v.0, self.subs[&v.1].face)).collect();
                    for (v, _) in &out {
                        self.pending.insert(*v);
                        self.evicted.insert(*v);
                    }
                    self.preempted += out.len();
                    self.enforce(&keys);
                    return true;
                }
                Err(b) => match self.victims(&b, prio) {
                    Some(vs) => {
                        for v in vs {
                            if let Some(path) = self.release_raw(v) {
                                out.push((v, path));
                            }
                        }
                    }
                    None => break,
                },
            }
        }
        for (v, path) in out.into_iter().rev() {
            self.admit_raw(v, path); // roll back
        }
        false
    }

    // ----------------------------------------------------------------- events

    /// Greedy: priority first, then smaller rate. Preemption only on declarations.
    fn try_all(&mut self, mut cands: Vec<Pair>, may_preempt: bool) {
        cands.sort_by(|a, b| {
            let (pa, pb) = (&self.pubs[&a.0], &self.pubs[&b.0]);
            pb.prio.cmp(&pa.prio).then(pa.rate.total_cmp(&pb.rate)).then(a.cmp(b))
        });
        let preempt = may_preempt && self.p.preempt && self.policy != Policy::None;
        for c in cands {
            if self.admitted.contains_key(&c) {
                continue;
            }
            let ok = match self.plan(c) {
                Ok(path) => {
                    self.admit_raw(c, path);
                    true
                }
                Err(_) => preempt && self.try_preempt(c),
            };
            if ok {
                self.pending.remove(&c);
            } else {
                self.pending.insert(c);
            }
        }
    }

    fn retry(&mut self) {
        let cands: Vec<Pair> = self.pending.iter().copied().collect();
        self.try_all(cands, false);
    }

    pub fn apply(&mut self, ev: &Event) {
        let t0 = Instant::now();
        match ev {
            Event::DeclPub(id, p) => {
                self.pubs.insert(*id, p.clone());
                let elig: Vec<Pair> =
                    self.subs.iter().filter(|(_, s)| p.ke.intersects(&s.sel)).map(|(&si, _)| (*id, si)).collect();
                self.fanout.push(elig.len());
                let cands = elig.into_iter().filter(|&c| self.admissible(c)).collect();
                self.try_all(cands, true);
            }
            Event::DeclSub(id, s) => {
                self.subs.insert(*id, s.clone());
                let elig: Vec<Pair> =
                    self.pubs.iter().filter(|(_, p)| p.ke.intersects(&s.sel)).map(|(&pi, _)| (pi, *id)).collect();
                if self.policy == Policy::Snapshot {
                    self.expanded.extend(elig.iter().copied());
                }
                let cands = elig.into_iter().filter(|&c| self.admissible(c)).collect();
                self.try_all(cands, true);
            }
            Event::UndeclPub(id) => {
                // The publisher's groups empty completely, so no budget re-check.
                let gone: Vec<Pair> = self.admitted.keys().filter(|c| c.0 == *id).copied().collect();
                for c in gone {
                    self.release_raw(c);
                }
                self.pending.retain(|c| c.0 != *id);
                self.expanded.retain(|c| c.0 != *id);
                self.evicted.retain(|c| c.0 != *id);
                self.pubs.remove(id);
                self.retry();
            }
            Event::UndeclSub(id) => {
                let face = self.subs[id].face;
                let gone: Vec<Pair> = self.admitted.keys().filter(|c| c.1 == *id).copied().collect();
                for c in gone {
                    self.release_raw(c);
                    self.enforce(&[(c.0, face)]);
                }
                self.pending.retain(|c| c.1 != *id);
                self.expanded.retain(|c| c.1 != *id);
                self.evicted.retain(|c| c.1 != *id);
                self.subs.remove(id);
                self.used.remove(id);
                self.retry();
            }
        }
        self.event_ns.push(t0.elapsed().as_nanos());
    }

    /// Snapshot policy: re-expand every subscription against current publishers.
    pub fn refresh(&mut self) {
        let mut new = vec![];
        for (&pi, p) in &self.pubs {
            for (&si, s) in &self.subs {
                if !self.expanded.contains(&(pi, si)) && p.ke.intersects(&s.sel) {
                    new.push((pi, si));
                }
            }
        }
        self.expanded.extend(new.iter().copied());
        self.try_all(new, false);
    }

    // ---------------------------------------------------------------- metrics

    /// Admitted pairs whose guarantee is broken (only possible without admission).
    pub fn violated(&self) -> usize {
        self.admitted
            .iter()
            .filter(|(&(pi, si), path)| {
                let (p, s) = (&self.pubs[&pi], &self.subs[&si]);
                self.used[&si] > s.budget + EPS
                    || self.delay(p, s, path) > p.deadline
                    || path.iter().any(|&l| self.load[l] > self.limit(l) + EPS)
                    || (s.peer && self.acc_load[s.router] > self.acc_limit() + EPS)
            })
            .count()
    }
}
