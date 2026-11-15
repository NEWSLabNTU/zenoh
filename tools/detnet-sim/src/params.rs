//! Command-line parameters.

use crate::state::Policy;

/// Which subscriber pays for a stream delivered once to a face that holds several
/// overlapping deterministic subscriptions (envelope charging, admission-model.typ).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Charging {
    /// Every matching subscription pays the full rate (double charging; baseline).
    PerPair,
    /// The most specific matching selector pays (fewest `**`, fewest `*`, most chunks).
    MostSpecific,
    /// The earliest declared matching subscription pays.
    FirstDeclared,
    /// The rate is split equally among the matching subscriptions.
    Split,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Routing {
    /// Per-source shortest-path trees by hop count (Zenoh default, equal weights).
    Spt,
    /// Per-source shortest-path trees with static weights 1 Gb/s / capacity
    /// (Zenoh `transport_weights` set inversely to link capacity).
    Weighted,
    /// Per-pair explicit branch chosen at admission time on residual capacity,
    /// grafted onto the publisher's tree (segment lists / BIER-TE, track E1/E2).
    Capacity,
}

/// Delivery from an edge router into its peer region.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PeerMode {
    /// One copy per subscribing peer face (Zenoh: peers do not relay).
    Star,
    /// One copy per publisher per peer region (hypothetical relaying or multicast).
    Tree,
}

#[derive(Clone, Debug)]
pub struct Params {
    pub seed: u64,
    pub routers: usize,
    pub extra_links: usize,
    pub fast_share: f64,
    pub events: usize,
    pub regions: usize,
    pub vehicles: usize,
    pub target_pubs: usize,
    pub target_subs: usize,
    pub guard: f64,
    pub sample_every: usize,
    /// Subscriber class weights: ops centre, region monitor, vehicle `**`,
    /// cross-region vehicle, exact stream. Normalised to sum 1.
    pub mix: [f64; 5],
    pub shared_face: f64,
    pub peer_share: f64,
    pub peer_routers: usize,
    pub peer_cap: f64,
    pub peer_mode: PeerMode,
    pub charging: Charging,
    pub routing: Routing,
    pub preempt: bool,
    pub refresh: usize,
    pub policies: Vec<Policy>,
    pub csv: bool,
    pub header: bool,
}

pub const USAGE: &str = "\
detnet-sim [options]
  --seed=N --routers=N --extra-links=N --fast-share=F --events=N
  --regions=N --vehicles=N --pubs=N --subs=N --guard=F
  --mix=OPS,REGION,VEHICLE,CROSS,EXACT   subscriber class weights (default 2,10,30,15,43)
  --shared-face=F      share of subscriptions declared on an existing face
  --peer-share=F       share of new faces that are peers in a peer region
  --peer-routers=N     routers hosting a peer region (default 5)
  --peer-cap=BPS       peer-region access capacity (default 100e6)
  --peer-mode=star|tree
  --charging=per-pair|most-specific|first-declared|split
  --routing=spt|weighted|capacity
  --preempt            priority preemption on declaration events
  --refresh=N          snapshot policy: re-expand wildcards every N events (0 = never)
  --policies=none,exactid,snapshot,wildcard
  --csv [--no-header] | --csv-header";

impl Params {
    pub fn from_args() -> Self {
        let mut p = Params {
            seed: 1,
            routers: 30,
            extra_links: 30,
            fast_share: 0.3,
            events: 20_000,
            regions: 4,
            vehicles: 50,
            target_pubs: 200,
            target_subs: 300,
            guard: 0.2,
            sample_every: 100,
            mix: [0.02, 0.10, 0.30, 0.15, 0.43],
            shared_face: 0.0,
            peer_share: 0.0,
            peer_routers: 5,
            peer_cap: 100e6,
            peer_mode: PeerMode::Star,
            charging: Charging::PerPair,
            routing: Routing::Spt,
            preempt: false,
            refresh: 1000,
            policies: vec![Policy::None, Policy::ExactId, Policy::Snapshot, Policy::Wildcard],
            csv: false,
            header: true,
        };
        for a in std::env::args().skip(1) {
            let (k, v) = a.split_once('=').unwrap_or((a.as_str(), ""));
            let n = || v.parse::<usize>().unwrap_or_else(|_| bad(&a));
            let f = || v.parse::<f64>().unwrap_or_else(|_| bad(&a));
            match k {
                "--seed" => p.seed = n() as u64,
                "--routers" => p.routers = n(),
                "--extra-links" => p.extra_links = n(),
                "--fast-share" => p.fast_share = f(),
                "--events" => p.events = n(),
                "--regions" => p.regions = n(),
                "--vehicles" => p.vehicles = n(),
                "--pubs" => p.target_pubs = n(),
                "--subs" => p.target_subs = n(),
                "--guard" => p.guard = f(),
                "--mix" => {
                    let w: Vec<f64> = v.split(',').map(|x| x.parse().unwrap_or_else(|_| bad(&a))).collect();
                    let sum: f64 = w.iter().sum();
                    if w.len() != 5 || sum <= 0.0 {
                        bad(&a);
                    }
                    for (i, x) in w.iter().enumerate() {
                        p.mix[i] = x / sum;
                    }
                }
                "--shared-face" => p.shared_face = f(),
                "--peer-share" => p.peer_share = f(),
                "--peer-routers" => p.peer_routers = n(),
                "--peer-cap" => p.peer_cap = f(),
                "--peer-mode" => {
                    p.peer_mode = match v {
                        "star" => PeerMode::Star,
                        "tree" => PeerMode::Tree,
                        _ => bad(&a),
                    }
                }
                "--charging" => {
                    p.charging = match v {
                        "per-pair" => Charging::PerPair,
                        "most-specific" => Charging::MostSpecific,
                        "first-declared" => Charging::FirstDeclared,
                        "split" => Charging::Split,
                        _ => bad(&a),
                    }
                }
                "--routing" => {
                    p.routing = match v {
                        "spt" => Routing::Spt,
                        "weighted" => Routing::Weighted,
                        "capacity" => Routing::Capacity,
                        _ => bad(&a),
                    }
                }
                "--preempt" => p.preempt = true,
                "--refresh" => p.refresh = n(),
                "--policies" => p.policies = v.split(',').map(|x| Policy::parse(x).unwrap_or_else(|| bad(&a))).collect(),
                "--csv" => p.csv = true,
                "--no-header" => p.header = false,
                "--csv-header" => {
                    println!("{}", crate::HEADER);
                    std::process::exit(0);
                }
                "--help" | "-h" => {
                    println!("{USAGE}");
                    std::process::exit(0);
                }
                _ => bad(&a),
            }
        }
        p
    }

    pub fn mix_str(&self) -> String {
        self.mix.iter().map(|w| format!("{:.0}", 100.0 * w)).collect::<Vec<_>>().join(":")
    }
}

impl Charging {
    pub fn name(self) -> &'static str {
        match self {
            Charging::PerPair => "per-pair",
            Charging::MostSpecific => "most-specific",
            Charging::FirstDeclared => "first-declared",
            Charging::Split => "split",
        }
    }
}

impl Routing {
    pub fn name(self) -> &'static str {
        match self {
            Routing::Spt => "spt",
            Routing::Weighted => "weighted",
            Routing::Capacity => "capacity",
        }
    }
}

impl PeerMode {
    pub fn name(self) -> &'static str {
        match self {
            PeerMode::Star => "star",
            PeerMode::Tree => "tree",
        }
    }
}

fn bad(arg: &str) -> ! {
    eprintln!("bad argument {arg}\n{USAGE}");
    std::process::exit(2);
}
