//! Admission simulator for wildcard-defined receiver sets (claim C2).
//!
//! Model: docs/detnet/admission-model.typ. One Zenoh router region; data follows
//! the source router's shortest-path tree (hop count, Zenoh's default equal link
//! weights); publishers and subscribers attach to routers and declare key
//! expressions matched with zenoh-keyexpr's `intersects`.
//!
//! Three policies run on the same random declaration trace:
//! - `none`:     no admission; every eligible pair is delivered, violations counted.
//! - `exact-id`: prior-art style; only pairs whose deterministic selector is a
//!   concrete key equal to the publisher's key are admitted (RSVP/MSRP-like).
//! - `wildcard`: the C2 model; any pair with `k_p intersects d_s` is a candidate,
//!   admitted greedily per declaration event against capacity, deadline, budget.
//!
//! Usage: detnet-sim [--seed=N] [--routers=N] [--events=N] [--vehicles=N]
//!                   [--pubs=N] [--subs=N] [--guard=F] [--csv]
//!
//! The trace churns around target populations: below target a declaration is
//! three times as likely as an undeclaration, above target the reverse.

use std::collections::{BTreeSet, HashMap, VecDeque};
use std::time::Instant;

use rand::{rngs::StdRng, Rng, SeedableRng};
use zenoh_keyexpr::OwnedKeyExpr;

// ---------------------------------------------------------------- parameters

#[derive(Clone, Debug)]
struct Params {
    seed: u64,
    routers: usize,
    extra_links: usize,
    events: usize,
    regions: usize,
    vehicles: usize,
    target_pubs: usize,
    target_subs: usize,
    guard: f64,
    sample_every: usize,
    csv: bool,
}

impl Params {
    fn from_args() -> Self {
        let mut p = Params {
            seed: 1,
            routers: 30,
            extra_links: 30,
            events: 20_000,
            regions: 4,
            vehicles: 50,
            target_pubs: 200,
            target_subs: 300,
            guard: 0.2,
            sample_every: 100,
            csv: false,
        };
        for a in std::env::args().skip(1) {
            let (k, v) = a.split_once('=').unwrap_or((a.as_str(), ""));
            let n = || v.parse::<usize>().expect("numeric value");
            match k {
                "--seed" => p.seed = n() as u64,
                "--routers" => p.routers = n(),
                "--extra-links" => p.extra_links = n(),
                "--events" => p.events = n(),
                "--regions" => p.regions = n(),
                "--vehicles" => p.vehicles = n(),
                "--pubs" => p.target_pubs = n(),
                "--subs" => p.target_subs = n(),
                "--guard" => p.guard = v.parse().expect("float"),
                "--csv" => p.csv = true,
                _ => panic!("unknown argument {a}"),
            }
        }
        p
    }
}

// ------------------------------------------------------------------ topology

const PKT_BITS: f64 = 12_000.0; // 1500-byte packet

struct Link {
    cap: f64,  // b/s
    prop: f64, // s
}

struct Topology {
    n: usize,
    links: Vec<Link>,                // directed
    adj: Vec<Vec<(usize, usize)>>,   // node -> (neighbour, link id)
    pred: Vec<Vec<Option<(usize, usize)>>>, // pred[src][v] = (prev node, link id)
}

impl Topology {
    fn random(p: &Params, rng: &mut StdRng) -> Self {
        let n = p.routers;
        let mut t = Topology { n, links: vec![], adj: vec![vec![]; n], pred: vec![] };
        let add = |t: &mut Topology, a: usize, b: usize, rng: &mut StdRng| {
            if a == b || t.adj[a].iter().any(|&(x, _)| x == b) {
                return;
            }
            let cap = if rng.gen_bool(0.3) { 1e9 } else { 100e6 };
            let prop = rng.gen_range(0.0005..0.005);
            for (u, v) in [(a, b), (b, a)] {
                t.links.push(Link { cap, prop });
                let id = t.links.len() - 1;
                t.adj[u].push((v, id));
            }
        };
        for i in 0..n {
            add(&mut t, i, (i + 1) % n, rng); // ring keeps it connected
        }
        for _ in 0..p.extra_links {
            let (a, b) = (rng.gen_range(0..n), rng.gen_range(0..n));
            add(&mut t, a, b, rng);
        }
        // Shortest-path trees by hop count (BFS), one per source router.
        t.pred = (0..n)
            .map(|src| {
                let mut pred = vec![None; n];
                let mut seen = vec![false; n];
                let mut q = VecDeque::from([src]);
                seen[src] = true;
                while let Some(u) = q.pop_front() {
                    for &(v, l) in &t.adj[u] {
                        if !seen[v] {
                            seen[v] = true;
                            pred[v] = Some((u, l));
                            q.push_back(v);
                        }
                    }
                }
                pred
            })
            .collect();
        t
    }

    fn path(&self, src: usize, dst: usize) -> Vec<usize> {
        let mut out = vec![];
        let mut v = dst;
        while v != src {
            let (u, l) = self.pred[src][v].expect("connected");
            out.push(l);
            v = u;
        }
        out.reverse();
        out
    }
}

// --------------------------------------------------------------- declarations

#[derive(Clone)]
struct Pub {
    ke: OwnedKeyExpr,
    router: usize,
    rate: f64,
    deadline: f64,
    prio: u8, // higher = more important
}

#[derive(Clone)]
struct Sub {
    sel: OwnedKeyExpr, // deterministic selector d_s
    router: usize,
    budget: f64,
}

fn is_wild(k: &OwnedKeyExpr) -> bool {
    k.as_str().contains('*')
}

fn ke(s: String) -> OwnedKeyExpr {
    OwnedKeyExpr::new(s).expect("valid key expression")
}

fn random_pub(p: &Params, rng: &mut StdRng, routers: usize) -> Pub {
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

fn random_sub(p: &Params, rng: &mut StdRng, routers: usize) -> Sub {
    let r = rng.gen_range(0..p.regions);
    let v = rng.gen_range(0..p.vehicles);
    let router = rng.gen_range(0..routers);
    let x: f64 = rng.gen();
    let (sel, budget) = if x < 0.02 {
        ("fleet/*/*/base".to_string(), 400e6) // operations centre
    } else if x < 0.12 {
        (format!("fleet/r{r}/**"), 100e6) // region monitor
    } else if x < 0.42 {
        (format!("fleet/r{r}/v{v}/**"), 10e6) // nearby peer watching one vehicle
    } else if x < 0.57 {
        (format!("fleet/*/v{v}/base"), 5e6) // vehicle id across regions
    } else {
        (format!("fleet/r{r}/v{v}/base"), 5e6) // exact stream
    };
    Sub { sel: ke(sel), router, budget }
}

#[derive(Clone)]
enum Event {
    DeclPub(usize, Pub),
    DeclSub(usize, Sub),
    UndeclPub(usize),
    UndeclSub(usize),
}

fn trace(p: &Params, topo: &Topology, rng: &mut StdRng) -> Vec<Event> {
    let (mut pubs, mut subs) = (Vec::<usize>::new(), Vec::<usize>::new());
    let mut next = 0usize;
    let mut out = Vec::with_capacity(p.events);
    while out.len() < p.events {
        let p_decl = |n: usize, target: usize| if n < target { 0.75 } else { 0.25 };
        if rng.gen_bool(0.5) {
            if pubs.is_empty() || rng.gen_bool(p_decl(pubs.len(), p.target_pubs)) {
                next += 1;
                pubs.push(next);
                out.push(Event::DeclPub(next, random_pub(p, rng, topo.n)));
            } else {
                let i = rng.gen_range(0..pubs.len());
                out.push(Event::UndeclPub(pubs.swap_remove(i)));
            }
        } else if subs.is_empty() || rng.gen_bool(p_decl(subs.len(), p.target_subs)) {
            next += 1;
            subs.push(next);
            out.push(Event::DeclSub(next, random_sub(p, rng, topo.n)));
        } else {
            let i = rng.gen_range(0..subs.len());
            out.push(Event::UndeclSub(subs.swap_remove(i)));
        }
    }
    out
}

// ------------------------------------------------------------------- policies

#[derive(Clone, Copy, PartialEq, Debug)]
enum Policy {
    None,
    ExactId,
    Wildcard,
}

type Pair = (usize, usize); // (publisher id, subscriber id)

struct State<'a> {
    policy: Policy,
    topo: &'a Topology,
    guard: f64,
    pubs: HashMap<usize, Pub>,
    subs: HashMap<usize, Sub>,
    admitted: BTreeSet<Pair>,
    pending: BTreeSet<Pair>,      // eligible but rejected; retried after releases
    refc: HashMap<(usize, usize), u32>, // (publisher, link) -> admitted paths using it
    load: Vec<f64>,
    used: HashMap<usize, f64>,    // subscriber -> admitted rate
    fanout: Vec<usize>,           // candidates per publisher declaration
    event_ns: Vec<u128>,
}

impl<'a> State<'a> {
    fn new(policy: Policy, topo: &'a Topology, guard: f64) -> Self {
        State {
            policy,
            topo,
            guard,
            pubs: HashMap::new(),
            subs: HashMap::new(),
            admitted: BTreeSet::new(),
            pending: BTreeSet::new(),
            refc: HashMap::new(),
            load: vec![0.0; topo.links.len()],
            used: HashMap::new(),
            fanout: vec![],
            event_ns: vec![],
        }
    }

    fn eligible(&self, p: &Pub, s: &Sub) -> bool {
        p.ke.intersects(&s.sel)
    }

    /// Pairs the policy is allowed to reserve for.
    fn admissible(&self, p: &Pub, s: &Sub) -> bool {
        match self.policy {
            Policy::None | Policy::Wildcard => true,
            Policy::ExactId => !is_wild(&s.sel) && !is_wild(&p.ke) && p.ke == s.sel,
        }
    }

    fn delay(&self, p: &Pub, path: &[usize]) -> f64 {
        path.iter()
            .map(|&l| {
                let link = &self.topo.links[l];
                PKT_BITS / p.rate + PKT_BITS / link.cap + link.prop
            })
            .sum()
    }

    fn fits(&self, (pi, si): Pair) -> bool {
        if self.policy == Policy::None {
            return true;
        }
        let (p, s) = (&self.pubs[&pi], &self.subs[&si]);
        if self.used.get(&si).copied().unwrap_or(0.0) + p.rate > s.budget {
            return false;
        }
        let path = self.topo.path(p.router, s.router);
        if self.delay(p, &path) > p.deadline {
            return false;
        }
        path.iter().all(|&l| {
            self.refc.get(&(pi, l)).copied().unwrap_or(0) > 0
                || self.load[l] + p.rate <= (1.0 - self.guard) * self.topo.links[l].cap
        })
    }

    fn admit(&mut self, (pi, si): Pair) {
        let (rate, path) = {
            let (p, s) = (&self.pubs[&pi], &self.subs[&si]);
            (p.rate, self.topo.path(p.router, s.router))
        };
        for l in path {
            let c = self.refc.entry((pi, l)).or_insert(0);
            *c += 1;
            if *c == 1 {
                self.load[l] += rate;
            }
        }
        *self.used.entry(si).or_insert(0.0) += rate;
        self.admitted.insert((pi, si));
    }

    fn release(&mut self, (pi, si): Pair) {
        if !self.admitted.remove(&(pi, si)) {
            return;
        }
        let (rate, path) = {
            let (p, s) = (&self.pubs[&pi], &self.subs[&si]);
            (p.rate, self.topo.path(p.router, s.router))
        };
        for l in path {
            let c = self.refc.get_mut(&(pi, l)).unwrap();
            *c -= 1;
            if *c == 0 {
                self.refc.remove(&(pi, l));
                self.load[l] -= rate;
            }
        }
        *self.used.get_mut(&si).unwrap() -= rate;
    }

    fn try_all(&mut self, mut cands: Vec<Pair>) {
        // Greedy: priority first, then smaller rate.
        cands.sort_by(|a, b| {
            let (pa, pb) = (&self.pubs[&a.0], &self.pubs[&b.0]);
            pb.prio.cmp(&pa.prio).then(pa.rate.total_cmp(&pb.rate))
        });
        for c in cands {
            if self.fits(c) {
                self.pending.remove(&c);
                self.admit(c);
            } else {
                self.pending.insert(c);
            }
        }
    }

    fn apply(&mut self, ev: &Event) {
        let t0 = Instant::now();
        match ev {
            Event::DeclPub(id, p) => {
                self.pubs.insert(*id, p.clone());
                let cands: Vec<Pair> = self
                    .subs
                    .iter()
                    .filter(|(_, s)| self.eligible(p, s))
                    .map(|(&si, _)| (*id, si))
                    .collect();
                self.fanout.push(cands.len());
                let cands = cands
                    .into_iter()
                    .filter(|c| self.admissible(p, &self.subs[&c.1]))
                    .collect();
                self.try_all(cands);
            }
            Event::DeclSub(id, s) => {
                self.subs.insert(*id, s.clone());
                let cands = self
                    .pubs
                    .iter()
                    .filter(|(_, p)| self.eligible(p, s) && self.admissible(p, s))
                    .map(|(&pi, _)| (pi, *id))
                    .collect();
                self.try_all(cands);
            }
            Event::UndeclPub(id) => {
                let gone: Vec<Pair> = self.admitted.iter().filter(|c| c.0 == *id).copied().collect();
                gone.into_iter().for_each(|c| self.release(c));
                self.pending.retain(|c| c.0 != *id);
                self.pubs.remove(id);
                self.retry();
            }
            Event::UndeclSub(id) => {
                let gone: Vec<Pair> = self.admitted.iter().filter(|c| c.1 == *id).copied().collect();
                gone.into_iter().for_each(|c| self.release(c));
                self.pending.retain(|c| c.1 != *id);
                self.subs.remove(id);
                self.used.remove(id);
                self.retry();
            }
        }
        self.event_ns.push(t0.elapsed().as_nanos());
    }

    fn retry(&mut self) {
        let cands: Vec<Pair> = self.pending.iter().copied().collect();
        self.try_all(cands);
    }

    /// Eligible pairs (M^det) in the current state.
    fn eligible_pairs(&self) -> usize {
        self.pubs
            .values()
            .map(|p| self.subs.values().filter(|s| self.eligible(p, s)).count())
            .sum()
    }

    /// Admitted pairs whose guarantee is broken (only possible without admission).
    fn violated(&self) -> usize {
        let cap = |l: usize| (1.0 - self.guard) * self.topo.links[l].cap;
        self.admitted
            .iter()
            .filter(|&&(pi, si)| {
                let (p, s) = (&self.pubs[&pi], &self.subs[&si]);
                let path = self.topo.path(p.router, s.router);
                self.used[&si] > s.budget + 1e-6
                    || self.delay(p, &path) > p.deadline
                    || path.iter().any(|&l| self.load[l] > cap(l) + 1e-6)
            })
            .count()
    }
}

// ----------------------------------------------------------------------- main

#[derive(Default)]
struct Acc {
    eligible: f64,
    admitted: f64,
    violated: f64,
    wild_share: f64,
    max_util: f64,
    n: f64,
}

fn pct(v: &mut [usize], q: f64) -> usize {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn main() {
    let p = Params::from_args();
    let mut rng = StdRng::seed_from_u64(p.seed);
    let topo = Topology::random(&p, &mut rng);
    let events = trace(&p, &topo, &mut rng);

    if p.csv {
        println!("policy,seed,eligible,admitted,admitted_frac,violated,violated_frac,wild_share,max_util,fanout_mean,fanout_p95,fanout_max,event_us_mean,event_us_max");
    } else {
        println!(
            "routers={} links={} events={} regions={} vehicles={} pubs~{} subs~{} guard={} seed={}",
            p.routers,
            topo.links.len() / 2,
            p.events,
            p.regions,
            p.vehicles,
            p.target_pubs,
            p.target_subs,
            p.guard,
            p.seed
        );
    }

    for policy in [Policy::None, Policy::ExactId, Policy::Wildcard] {
        let mut st = State::new(policy, &topo, p.guard);
        let mut acc = Acc::default();
        for (i, ev) in events.iter().enumerate() {
            st.apply(ev);
            // Sample after warm-up (first quarter of the trace).
            if i >= p.events / 4 && i % p.sample_every == 0 {
                let elig = st.eligible_pairs() as f64;
                let wild = st
                    .admitted
                    .iter()
                    .filter(|c| is_wild(&st.subs[&c.1].sel) || is_wild(&st.pubs[&c.0].ke))
                    .count() as f64;
                acc.eligible += elig;
                acc.admitted += st.admitted.len() as f64;
                acc.violated += st.violated() as f64;
                acc.wild_share += if st.admitted.is_empty() { 0.0 } else { wild / st.admitted.len() as f64 };
                let util = (0..topo.links.len())
                    .map(|l| st.load[l] / topo.links[l].cap)
                    .fold(0.0, f64::max);
                acc.max_util += util;
                acc.n += 1.0;
            }
        }
        let n = acc.n.max(1.0);
        let (elig, adm, vio) = (acc.eligible / n, acc.admitted / n, acc.violated / n);
        let fan_mean = st.fanout.iter().sum::<usize>() as f64 / st.fanout.len().max(1) as f64;
        let (fan_p95, fan_max) = (pct(&mut st.fanout, 0.95), pct(&mut st.fanout, 1.0));
        let us_mean = st.event_ns.iter().sum::<u128>() as f64 / st.event_ns.len() as f64 / 1e3;
        let us_max = *st.event_ns.iter().max().unwrap() as f64 / 1e3;
        let name = format!("{policy:?}").to_lowercase();
        if p.csv {
            println!(
                "{name},{},{elig:.1},{adm:.1},{:.3},{vio:.1},{:.3},{:.3},{:.2},{fan_mean:.1},{fan_p95},{fan_max},{us_mean:.1},{us_max:.1}",
                p.seed,
                adm / elig.max(1.0),
                vio / adm.max(1.0),
                acc.wild_share / n,
                acc.max_util / n
            );
        } else {
            println!(
                "\n[{name}]\n  eligible pairs (M^det)   {elig:.1}\n  admitted pairs           {adm:.1}  ({:.1}% of eligible)\n  admitted but violated    {vio:.1}  ({:.1}% of admitted)\n  admitted via wildcards   {:.1}%\n  max link utilisation     {:.2}\n  candidates per pub decl  mean {fan_mean:.1}, p95 {fan_p95}, max {fan_max}\n  event cost               mean {us_mean:.1} us, max {us_max:.1} us",
                100.0 * adm / elig.max(1.0),
                100.0 * vio / adm.max(1.0),
                100.0 * acc.wild_share / n,
                acc.max_util / n
            );
        }
    }
}
