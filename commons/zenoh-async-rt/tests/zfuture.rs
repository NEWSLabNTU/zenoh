use std::time::{Duration, Instant};
use zenoh_async_rt::zfuture;

const DURATION: Duration = Duration::from_millis(100);

struct Sleeper;

impl Sleeper {
    #[zfuture]
    async fn sleep() {
        async_std::task::sleep(DURATION).await
    }
}

#[test]
fn derive_zfuture_blocking() {
    let start = Instant::now();
    Sleeper::sleep().wait();
    assert!(start.elapsed() >= DURATION);
}

#[async_std::test]
async fn derive_zfuture_async() {
    let start = Instant::now();
    Sleeper::sleep().await;
    assert!(start.elapsed() >= DURATION);
}
