//! Random router-region topology and static per-source trees.

use std::collections::VecDeque;

use rand::{rngs::StdRng, Rng};

use crate::params::{Params, Routing};

pub const PKT_BITS: f64 = 12_000.0; // 1500-byte packet

pub struct Link {
    pub from: usize,
    pub to: usize,
    pub cap: f64,  // b/s
    pub prop: f64, // s
}

pub struct Topology {
    pub n: usize,
    pub links: Vec<Link>,              // directed
    pub adj: Vec<Vec<(usize, usize)>>, // node -> (neighbour, link id)
    pred: Vec<Vec<Option<(usize, usize)>>>, // pred[src][v] = (prev node, link id)
}

impl Topology {
    pub fn random(p: &Params, rng: &mut StdRng) -> Self {
        let n = p.routers;
        let mut t = Topology { n, links: vec![], adj: vec![vec![]; n], pred: vec![] };
        let add = |t: &mut Topology, a: usize, b: usize, rng: &mut StdRng| {
            if a == b || t.adj[a].iter().any(|&(x, _)| x == b) {
                return;
            }
            let cap = if rng.gen_bool(p.fast_share) { 1e9 } else { 100e6 };
            let prop = rng.gen_range(0.0005..0.005);
            for (u, v) in [(a, b), (b, a)] {
                t.links.push(Link { from: u, to: v, cap, prop });
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
        t.pred = (0..n)
            .map(|src| match p.routing {
                Routing::Weighted => t.dijkstra(src),
                // Capacity routing keeps hop-count trees as its default path.
                Routing::Spt | Routing::Capacity => t.bfs(src),
            })
            .collect();
        t
    }

    fn bfs(&self, src: usize) -> Vec<Option<(usize, usize)>> {
        let mut pred = vec![None; self.n];
        let mut seen = vec![false; self.n];
        let mut q = VecDeque::from([src]);
        seen[src] = true;
        while let Some(u) = q.pop_front() {
            for &(v, l) in &self.adj[u] {
                if !seen[v] {
                    seen[v] = true;
                    pred[v] = Some((u, l));
                    q.push_back(v);
                }
            }
        }
        pred
    }

    /// Static weights 1 Gb/s / capacity: 1 for 1 Gb/s links, 10 for 100 Mb/s.
    fn dijkstra(&self, src: usize) -> Vec<Option<(usize, usize)>> {
        let mut pred = vec![None; self.n];
        let mut dist = vec![f64::INFINITY; self.n];
        let mut done = vec![false; self.n];
        dist[src] = 0.0;
        while let Some(u) = (0..self.n)
            .filter(|&v| !done[v] && dist[v].is_finite())
            .min_by(|&a, &b| dist[a].total_cmp(&dist[b]))
        {
            done[u] = true;
            for &(v, l) in &self.adj[u] {
                let d = dist[u] + 1e9 / self.links[l].cap;
                if d < dist[v] {
                    dist[v] = d;
                    pred[v] = Some((u, l));
                }
            }
        }
        pred
    }

    /// Default path from `src` to `dst` along `src`'s static tree.
    pub fn path(&self, src: usize, dst: usize) -> Vec<usize> {
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

    /// Rate-based per-hop bound: L/r + L/c + propagation.
    pub fn hop_delay(&self, l: usize, rate: f64) -> f64 {
        let link = &self.links[l];
        PKT_BITS / rate + PKT_BITS / link.cap + link.prop
    }
}
