use clap::Parser;
use log::info;
use std::time::Instant;
use zenoh::prelude::r#async::*;
use zenoh_examples::CommonArgs;

#[derive(clap::Parser, Clone, PartialEq, Eq, Hash, Debug)]
struct Args {
    #[arg(short, long, default_value = "demo/example/zenoh-prio")]
    /// The key expression to write to.
    key: KeyExpr<'static>,

    /// Print the statistics
    #[arg(short = 't', long)]
    print: bool,

    /// Number of messages in each throughput measurements
    #[arg(short, long, default_value = "100000")]
    number: usize,

    #[arg(short = 's', long)]
    /// Sets the size of the payload to publish
    payload_size: usize,

    priorities: Vec<u8>,

    #[command(flatten)]
    common: CommonArgs,
}

#[async_std::main]
async fn main() -> zenoh::Result<()> {
    // Initiate logging
    env_logger::init();

    let Args {
        key,
        priorities,
        print,
        number,
        payload_size,
        common,
    } = Args::parse();
    let config: Config = common.into();

    let priorities: Vec<Priority> = priorities
        .into_iter()
        .map(|p| {
            p.try_into()
                .unwrap_or_else(|_| panic!("'{p}' is not a valid priority"))
        })
        .collect();
    let data: Value = (0..10)
        .cycle()
        .take(payload_size)
        .collect::<Vec<u8>>()
        .into();

    info!("Opening session...");
    let session = zenoh::open(config).res().await?;

    info!("Declaring Publisher on '{key}'...");
    let mut publisher = session
        .declare_publisher(key)
        .congestion_control(CongestionControl::Block)
        .res()
        .await?;

    let mut start = Instant::now();

    for (count, &prio) in (1..=number).cycle().zip(priorities.iter().cycle()) {
        publisher.put(data.clone()).res().await?;
        publisher = publisher.priority(prio);

        if print && count == number {
            let thpt = number as f64 / start.elapsed().as_secs_f64();
            println!("{thpt} msg/s");
            start = Instant::now();
        }
    }

    Ok(())
}
