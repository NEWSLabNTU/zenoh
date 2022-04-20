use futures::{future::BoxFuture, FutureExt};
use std::fmt;
use std::time::Instant;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

pub use spawn::*;
mod spawn {
    use super::*;

    pub fn spawn<F, T>(future: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        JoinHandle(async_std::task::spawn(future))
    }

    pub fn spawn_blocking<F, T>(f: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        JoinHandle(async_std::task::spawn_blocking(f))
    }

    pub struct JoinHandle<T>(async_std::task::JoinHandle<T>);

    impl<T> JoinHandle<T> {
        pub async fn cancel(self) -> Option<T> {
            self.0.cancel().await
        }
    }

    impl<T> fmt::Debug for JoinHandle<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_tuple("JoinHandle").finish()
        }
    }

    impl<T> Future for JoinHandle<T> {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            self.0.poll_unpin(cx)
        }
    }
}

pub use sleep::*;
mod sleep {
    use super::*;

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
}

pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    async_std::task::block_on(future)
}

pub async fn yield_now() {
    async_std::task::yield_now().await
}
