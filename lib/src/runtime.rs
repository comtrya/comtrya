//! The one tokio runtime of comtrya: DNS lookups, downloads and the `include_variables` sources
//! run on it, instead of each building its own runtime or blocking HTTP client

use std::future::Future;
use std::sync::LazyLock;
use tokio::runtime::{Builder, Runtime};

/// Built on first use, and never shut down: a task that's still running when comtrya exits, like
/// an `include_variables` source that never answered, doesn't keep it from exiting
static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .thread_name("comtrya-runtime")
        .enable_all()
        .build()
        .expect("the tokio runtime can be built")
});

/// Runs `future` to completion on the runtime and waits for it.
///
/// For synchronous code, like the executor and the atoms. It panics when called from async code
/// on the runtime; code inside [`spawn_blocking`] may call it
pub fn block_on<F: Future>(future: F) -> F::Output {
    RUNTIME.handle().block_on(future)
}

/// Runs the blocking `work` on the runtime's thread pool for blocking code, so async code can
/// wait for it without stopping other tasks
pub fn spawn_blocking<F, R>(work: F) -> tokio::task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    RUNTIME.spawn_blocking(work)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocking_work_can_wait_for_async_work() {
        let answer = block_on(async { spawn_blocking(|| block_on(async { 42 })).await.unwrap() });

        assert_eq!(42, answer);
    }
}
