//! One-way latency probe for the DetNet testbed.
//!
//! `zprobe pub` sends payloads `[seq: u64][send_ns: u64][padding]` at a fixed rate;
//! `zprobe sub` stamps arrival, writes one CSV row per sample and prints a summary.
//! All containers share the host clock, so `recv_ns - send_ns` is one-way latency.

use std::{
    fs::File,
    io::{BufWriter, Write},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use clap::{Parser, Subcommand, ValueEnum};
use zenoh::{
    qos::{CongestionControl, Priority},
    Config, Wait,
};

#[derive(Parser)]
struct Cli {
    /// Endpoint of the router to connect to, e.g. tcp/r1:7447
    #[arg(short = 'e', long, global = true, default_value = "tcp/127.0.0.1:7447")]
    connect: Vec<String>,
    /// Extra config entries as key=json5 (repeatable)
    #[arg(long = "cfg", global = true)]
    cfg: Vec<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Publish timestamped samples
    Pub {
        #[arg(short, long, default_value = "detnet/probe")]
        key: String,
        /// Messages per second; 0 = as fast as possible
        #[arg(short, long, default_value_t = 1000)]
        rate: u64,
        /// Payload size in bytes (>= 16)
        #[arg(short, long, default_value_t = 1024)]
        size: usize,
        /// Seconds to run
        #[arg(short, long, default_value_t = 10)]
        duration: u64,
        /// Zenoh priority 1 (RealTime) .. 7 (Background)
        #[arg(short, long, default_value_t = 5)]
        prio: u8,
        #[arg(long)]
        express: bool,
        #[arg(long, value_enum, default_value_t = Cc::Block)]
        congestion: Cc,
        /// Seconds to wait before publishing (lets routes settle)
        #[arg(long, default_value_t = 2)]
        warmup: u64,
    },
    /// Subscribe and record latency
    Sub {
        #[arg(short, long, default_value = "detnet/probe")]
        key: String,
        /// Seconds to run
        #[arg(short, long, default_value_t = 15)]
        duration: u64,
        /// CSV output (seq,send_ns,recv_ns,lat_ns); none = summary only
        #[arg(short, long)]
        out: Option<String>,
        /// Label printed in the summary line
        #[arg(long, default_value = "probe")]
        label: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Cc {
    Block,
    Drop,
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

fn open(cli: &Cli) -> zenoh::Session {
    let mut c = Config::default();
    c.insert_json5("mode", "\"client\"").unwrap();
    let eps: Vec<String> = cli.connect.iter().map(|e| format!("\"{e}\"")).collect();
    c.insert_json5("connect/endpoints", &format!("[{}]", eps.join(",")))
        .unwrap();
    c.insert_json5("scouting/multicast/enabled", "false").unwrap();
    for kv in &cli.cfg {
        let (k, v) = kv.split_once('=').expect("--cfg key=value");
        c.insert_json5(k, v).unwrap();
    }
    zenoh::open(c).wait().expect("open session")
}

fn main() {
    zenoh::init_log_from_env_or("error");
    let cli = Cli::parse();
    match &cli.cmd {
        Cmd::Pub {
            key,
            rate,
            size,
            duration,
            prio,
            express,
            congestion,
            warmup,
        } => run_pub(
            &cli, key, *rate, *size, *duration, *prio, *express, *congestion, *warmup,
        ),
        Cmd::Sub {
            key,
            duration,
            out,
            label,
        } => run_sub(&cli, key, *duration, out.as_deref(), label),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_pub(
    cli: &Cli,
    key: &str,
    rate: u64,
    size: usize,
    duration: u64,
    prio: u8,
    express: bool,
    cc: Cc,
    warmup: u64,
) {
    let size = size.max(16);
    let session = open(cli);
    let publisher = session
        .declare_publisher(key.to_string())
        .priority(Priority::try_from(prio).expect("prio 1..7"))
        .express(express)
        .congestion_control(match cc {
            Cc::Block => CongestionControl::Block,
            Cc::Drop => CongestionControl::Drop,
        })
        .wait()
        .unwrap();
    std::thread::sleep(Duration::from_secs(warmup));

    let mut buf = vec![0u8; size];
    let period = (rate > 0).then(|| Duration::from_nanos(1_000_000_000 / rate));
    let start = Instant::now();
    let end = start + Duration::from_secs(duration);
    let mut seq: u64 = 0;
    loop {
        let now = Instant::now();
        if now >= end {
            break;
        }
        if let Some(p) = period {
            // Absolute schedule: lateness does not accumulate.
            let due = start + p * seq as u32;
            if due > now {
                std::thread::sleep(due - now);
            }
        }
        buf[0..8].copy_from_slice(&seq.to_le_bytes());
        buf[8..16].copy_from_slice(&now_ns().to_le_bytes());
        publisher.put(buf.clone()).wait().unwrap();
        seq += 1;
    }
    let secs = start.elapsed().as_secs_f64();
    eprintln!(
        "pub key={key} sent={seq} rate={:.0}/s throughput={:.1} Mb/s",
        seq as f64 / secs,
        seq as f64 * size as f64 * 8.0 / secs / 1e6
    );
}

fn run_sub(cli: &Cli, key: &str, duration: u64, out: Option<&str>, label: &str) {
    let session = open(cli);
    let rows: Arc<Mutex<Vec<(u64, u64, u64)>>> =
        Arc::new(Mutex::new(Vec::with_capacity(1 << 20)));
    let r = rows.clone();
    let _sub = session
        .declare_subscriber(key.to_string())
        .callback(move |s| {
            let recv = now_ns();
            let p = s.payload().to_bytes();
            if p.len() >= 16 {
                let seq = u64::from_le_bytes(p[0..8].try_into().unwrap());
                let send = u64::from_le_bytes(p[8..16].try_into().unwrap());
                r.lock().unwrap().push((seq, send, recv));
            }
        })
        .wait()
        .unwrap();
    std::thread::sleep(Duration::from_secs(duration));
    drop(_sub);
    let rows = std::mem::take(&mut *rows.lock().unwrap());

    if let Some(path) = out {
        let mut w = BufWriter::new(File::create(path).expect("create csv"));
        writeln!(w, "seq,send_ns,recv_ns,lat_ns").unwrap();
        for (seq, send, recv) in &rows {
            writeln!(w, "{seq},{send},{recv},{}", recv.saturating_sub(*send)).unwrap();
        }
    }
    let mut lat: Vec<u64> = rows.iter().map(|(_, s, r)| r.saturating_sub(*s)).collect();
    lat.sort_unstable();
    let n = lat.len();
    if n == 0 {
        println!("{label}: no samples");
        return;
    }
    let q = |f: f64| lat[((n as f64 * f) as usize).min(n - 1)] as f64 / 1e3;
    let (lo, hi) = rows
        .iter()
        .fold((u64::MAX, 0), |(lo, hi), r| (lo.min(r.0), hi.max(r.0)));
    let expected = hi - lo + 1;
    println!(
        "{label}: n={n} loss={:.3}% p50={:.0}us p99={:.0}us p99.9={:.0}us p99.99={:.0}us max={:.0}us",
        100.0 * (1.0 - n as f64 / expected as f64),
        q(0.5),
        q(0.99),
        q(0.999),
        q(0.9999),
        lat[n - 1] as f64 / 1e3
    );
}
