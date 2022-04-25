use std::future::Future;

pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    async_std::task::block_on(future)
}

pub async fn yield_now() {
    async_std::task::yield_now().await
}
