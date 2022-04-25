use futures::{future::BoxFuture, FutureExt};
use std::fmt;
use std::time::Instant;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

pub fn sleep(dur: Duration) -> Sleep {
    Sleep {
        future: async_std::task::sleep(dur).boxed(),
    }
}

pub fn sleep_until(deadline: Instant) -> Sleep {
    let remaining = deadline.saturating_duration_since(Instant::now());
    sleep(remaining)
}

pub struct Sleep {
    future: BoxFuture<'static, ()>,
}

impl fmt::Debug for Sleep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sleep").finish()
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.future.poll_unpin(cx)
    }
}
