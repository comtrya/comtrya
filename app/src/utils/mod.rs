mod dependency_graph;
use anyhow::{anyhow, Result};
use backoff::{future::retry, ExponentialBackoff};
pub use dependency_graph::DependencyGraph;
use std::iter::from_fn;
use tokio::time::{timeout, Duration};

pub trait ZipLongest<T>: Iterator<Item = T> {
    fn zip_longest<U, I>(self, other: I) -> impl Iterator<Item = (Option<T>, Option<U>)>
    where
        Self: Sized,
        I: IntoIterator<Item = U>;
}

impl<T, I: Iterator<Item = T>> ZipLongest<T> for I {
    fn zip_longest<U, J>(self, other: J) -> impl Iterator<Item = (Option<T>, Option<U>)>
    where
        J: IntoIterator<Item = U>,
    {
        let mut a = self.fuse();
        let mut b = other.into_iter().fuse();

        from_fn(move || match (a.next(), b.next()) {
            (None, None) => None,
            (a, b) => Some((a, b)),
        })
    }
}

#[allow(dead_code)]
pub async fn try_acquire_lock<T>(operation: impl Fn() -> Result<T>) -> Result<T>
where
    T: Send + 'static,
{
    let backoff = ExponentialBackoff {
        initial_interval: Duration::from_millis(10),
        max_interval: Duration::from_secs(1),
        max_elapsed_time: Some(Duration::from_secs(5)),
        ..Default::default()
    };

    retry(backoff, || async {
        match timeout(Duration::from_millis(100), async { operation() }).await {
            Ok(result) => result.map_err(backoff::Error::Permanent),
            Err(_) => Err(backoff::Error::Permanent(anyhow!(
                "Lock acquisition timeout"
            ))),
        }
    })
    .await
}
