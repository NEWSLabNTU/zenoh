//! Admission simulator for wildcard-defined receiver sets (claim C2).
//!
//! Model: docs/detnet/admission-model.typ. One Zenoh router region; publishers and
//! subscribers attach to routers and declare key expressions matched with
//! zenoh-keyexpr's `intersects`. Policies (see `state::Policy`) run on the same
//! random declaration trace. Run with `--help` for options.

mod params;
mod state;
mod topology;
mod workload;

use rand::{rngs::StdRng, SeedableRng};

use params::Params;
use state::{Policy, State};
use topology::Topology;
use workload::{is_wild, trace};

#[derive(Default)]
struct Acc {
    eligible: f64,
    eligible_hi: f64,
    eligible_peer: f64,
    admitted: f64,
    admitted_hi: f64,
    admitted_peer: f64,
    violated: f64,
    wild_share: f64,
    max_util: f64,
    budget_util: f64,
    hops: f64,
    n: f64,
}

impl Acc {
    fn sample(&mut self, st: &State) {
        for p in st.pubs.values() {
            for s in st.subs.values() {
                if p.ke.intersects(&s.sel) {
                    self.eligible += 1.0;
                    self.eligible_hi += f64::from(p.prio >= 2);
                    self.eligible_peer += f64::from(s.peer);
                }
            }
        }
        let (mut wild, mut hops) = (0.0, 0.0);
        for (&(pi, si), path) in &st.admitted {
            let (p, s) = (&st.pubs[&pi], &st.subs[&si]);
            self.admitted_hi += f64::from(p.prio >= 2);
            self.admitted_peer += f64::from(s.peer);
            wild += f64::from(is_wild(&s.sel) || is_wild(&p.ke));
            hops += path.len() as f64;
        }
        let adm = st.admitted.len() as f64;
        self.admitted += adm;
        self.wild_share += if adm > 0.0 { wild / adm } else { 0.0 };
        self.hops += if adm > 0.0 { hops / adm } else { 0.0 };
        self.violated += st.violated() as f64;
        self.max_util += (0..st.topo.links.len()).map(|l| st.load[l] / st.topo.links[l].cap).fold(0.0, f64::max);
        let bu: f64 = st.subs.iter().map(|(si, s)| st.used.get(si).copied().unwrap_or(0.0) / s.budget).sum();
        self.budget_util += bu / st.subs.len().max(1) as f64;
        self.n += 1.0;
    }
}

fn pct(v: &mut [usize], q: f64) -> usize {
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn ratio(a: f64, b: f64) -> f64 {
    if b > 0.0 {
        a / b
    } else {
        f64::NAN
    }
}

pub const HEADER: &str = "policy,seed,routers,extra_links,fast_share,mix,shared_face,peer_share,peer_mode,charging,routing,preempt,refresh,\
eligible,admitted,admitted_frac,admitted_frac_hi,admitted_frac_lo,admitted_frac_peer,violated,violated_frac,wild_share,\
max_util,budget_util,hops,preempted,readmit_frac,forced,fanout_mean,fanout_p95,fanout_max,event_us_mean,event_us_max";

fn main() {
    let p = Params::from_args();
    let mut rng = StdRng::seed_from_u64(p.seed);
    let topo = Topology::random(&p, &mut rng);
    let events = trace(&p, topo.n, &mut rng);

    if p.csv {
        if p.header {
            println!("{HEADER}");
        }
    } else {
        println!(
            "routers={} links={} events={} pubs~{} subs~{} guard={} mix={} shared-face={} peer-share={} peer-mode={} \
             charging={} routing={} preempt={} refresh={} seed={}",
            p.routers,
            topo.links.len() / 2,
            p.events,
            p.target_pubs,
            p.target_subs,
            p.guard,
            p.mix_str(),
            p.shared_face,
            p.peer_share,
            p.peer_mode.name(),
            p.charging.name(),
            p.routing.name(),
            p.preempt,
            p.refresh,
            p.seed
        );
    }

    for &policy in &p.policies {
        let mut st = State::new(policy, &p, &topo);
        let mut acc = Acc::default();
        for (i, ev) in events.iter().enumerate() {
            st.apply(ev);
            if policy == Policy::Snapshot && p.refresh > 0 && (i + 1) % p.refresh == 0 {
                st.refresh();
            }
            // Sample after warm-up (first quarter of the trace).
            if i >= p.events / 4 && i % p.sample_every == 0 {
                acc.sample(&st);
            }
        }
        let n = acc.n.max(1.0);
        let (elig, adm, vio) = (acc.eligible / n, acc.admitted / n, acc.violated / n);
        let frac = ratio(adm, elig);
        let frac_hi = ratio(acc.admitted_hi, acc.eligible_hi);
        let frac_lo = ratio(acc.admitted - acc.admitted_hi, acc.eligible - acc.eligible_hi);
        let frac_peer = ratio(acc.admitted_peer, acc.eligible_peer);
        let readmit = ratio(st.readmitted as f64, st.preempted as f64);
        let fan_mean = st.fanout.iter().sum::<usize>() as f64 / st.fanout.len().max(1) as f64;
        let (fan_p95, fan_max) = (pct(&mut st.fanout, 0.95), pct(&mut st.fanout, 1.0));
        let us_mean = st.event_ns.iter().sum::<u128>() as f64 / st.event_ns.len() as f64 / 1e3;
        let us_max = *st.event_ns.iter().max().unwrap() as f64 / 1e3;
        let name = policy.name();
        if p.csv {
            println!(
                "{name},{},{},{},{},{},{},{},{},{},{},{},{},{elig:.1},{adm:.1},{frac:.4},{frac_hi:.4},{frac_lo:.4},{frac_peer:.4},\
                 {vio:.1},{:.4},{:.4},{:.3},{:.4},{:.2},{},{readmit:.3},{},{fan_mean:.1},{fan_p95},{fan_max},{us_mean:.1},{us_max:.1}",
                p.seed,
                p.routers,
                p.extra_links,
                p.fast_share,
                p.mix_str(),
                p.shared_face,
                p.peer_share,
                p.peer_mode.name(),
                p.charging.name(),
                p.routing.name(),
                p.preempt,
                p.refresh,
                vio / adm.max(1.0),
                acc.wild_share / n,
                acc.max_util / n,
                acc.budget_util / n,
                acc.hops / n,
                st.preempted,
                st.forced,
            );
        } else {
            println!(
                "\n[{name}]\n  eligible pairs (M^det)   {elig:.1}\n  admitted pairs           {adm:.1}  ({:.1}% of eligible; prio 2 {:.1}%, prio 1 {:.1}%, peer subs {:.1}%)\n  admitted but violated    {vio:.1}  ({:.1}% of admitted)\n  admitted via wildcards   {:.1}%\n  max link utilisation     {:.2}\n  mean budget utilisation  {:.1}%\n  mean path hops           {:.2}\n  preempted pairs          {} (re-admitted {:.1}%), forced by charging {}\n  candidates per pub decl  mean {fan_mean:.1}, p95 {fan_p95}, max {fan_max}\n  event cost               mean {us_mean:.1} us, max {us_max:.1} us",
                100.0 * frac,
                100.0 * frac_hi,
                100.0 * frac_lo,
                100.0 * frac_peer,
                100.0 * vio / adm.max(1.0),
                100.0 * acc.wild_share / n,
                acc.max_util / n,
                100.0 * acc.budget_util / n,
                acc.hops / n,
                st.preempted,
                100.0 * readmit,
                st.forced,
            );
        }
    }
}
