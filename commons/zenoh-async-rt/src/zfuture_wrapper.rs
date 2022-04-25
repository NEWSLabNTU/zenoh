use futures::Future;
use pin_project::pin_project;
use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[pin_project]
pub struct ZFuture<F: Future>(#[pin] F);

impl<F: Future> ZFuture<F> {
    pub fn new(future: F) -> Self {
        Self(future)
    }

    pub fn wait(self) -> F::Output {
        async_std::task::block_on(self.0)
    }
}

impl<F: Future> Deref for ZFuture<F> {
    type Target = F;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<F: Future> DerefMut for ZFuture<F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<F: Future> Future for ZFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().0.poll(cx)
    }
}
