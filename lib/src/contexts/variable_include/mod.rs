use std::collections::{BTreeMap, HashMap};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use anyhow::Result;
use reqwest::Url;

use crate::config::Config;
use crate::runner::SystemRunner;
use tracing::warn;

pub mod dns;
pub mod file;
pub mod op;

/// How long all sources together may take. A source that is still running then is left out, so
/// an unreachable DNS server or a stuck `op` can't hang comtrya
const TIMEOUT: Duration = Duration::from_secs(30);

/// The variables of one source
type Values = HashMap<String, String>;

/// Reads the variables of `include_variables` in `Comtrya.yaml`, the `include_variables` context.
/// The sources are read at the same time. A source that fails or takes longer than [`TIMEOUT`]
/// is logged, and only its variables are left out
pub fn gather(config: &Config) -> BTreeMap<String, String> {
    let sources: Vec<String> = config.include_variables.iter().flatten().cloned().collect();

    gather_with(&sources, TIMEOUT, read)
}

/// Reads every source with `read` in its own thread and merges the variables in the order of
/// `sources`, so a later source overrides an earlier one, as before they were read at once
fn gather_with(
    sources: &[String],
    timeout: Duration,
    read: impl Fn(&str) -> Result<Values> + Send + Sync + 'static,
) -> BTreeMap<String, String> {
    let read = Arc::new(read);
    let deadline = Instant::now() + timeout;

    let pending: Vec<_> = sources
        .iter()
        .map(|source| (source, spawn_read(source, &read)))
        .collect();

    let mut variables = BTreeMap::new();
    for (source, receiver) in pending {
        let waited = receiver.recv_timeout(deadline.saturating_duration_since(Instant::now()));

        match waited {
            Ok(Ok(values)) => variables.extend(values),
            Ok(Err(err)) => {
                warn!("Can't read the variables of {source} in include_variables: {err}")
            }
            Err(mpsc::RecvTimeoutError::Timeout) => warn!(
                "Left out the variables of {source} in include_variables: no answer within {}s",
                timeout.as_secs()
            ),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                warn!("Can't read the variables of {source} in include_variables: it crashed")
            }
        }
    }

    variables
}

/// Reads `source` in a new thread. The thread isn't joined: one that never answers is left
/// behind and ends with comtrya
fn spawn_read(
    source: &str,
    read: &Arc<impl Fn(&str) -> Result<Values> + Send + Sync + 'static>,
) -> mpsc::Receiver<Result<Values>> {
    let (sender, receiver) = mpsc::channel();
    let source = source.to_string();
    let read = Arc::clone(read);

    std::thread::spawn(move || {
        // The receiver is gone after a timeout, nobody needs the answer then
        let _ = sender.send(read(&source));
    });

    receiver
}

/// Reads the variables of one `include_variables` source
fn read(source: &str) -> Result<Values> {
    let mut values = Values::new();

    // Vault and item names may contain spaces, which a URL can't
    if source.starts_with("op://") {
        op::item_values(source, &SystemRunner, &mut values)?;
        return Ok(values);
    }

    let url = Url::parse(source)?;
    match url.scheme() {
        "dns+txt" => dns::txt_record_values(&url, &mut values)?,
        "file+toml" => file::toml_values(&url, &mut values)?,
        "file+yaml" => file::yaml_values(&url, &mut values)?,
        scheme => anyhow::bail!("Unknown variable include scheme: {scheme}"),
    }

    Ok(values)
}

#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;

    #[test]
    fn a_failing_source_keeps_the_variables_of_the_others() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("vars.yaml");
        std::fs::write(&file, "ship: Daedalus\n").unwrap();

        let config = Config {
            include_variables: Some(vec![
                String::from("file+yaml:///comtrya-no-such-file.yaml"),
                format!("file+yaml://{}", file.display()),
            ]),
            ..Default::default()
        };

        let variables = gather(&config);
        assert!(variables.contains_key("ship"), "{variables:?}");
    }
}

#[cfg(test)]
mod concurrency_tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::sync::Barrier;

    fn sources(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn values(pairs: &[(&str, &str)]) -> Values {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn sources_are_read_at_the_same_time() {
        // Each source waits for the other: read one after the other, both would time out
        let barrier = Barrier::new(2);

        let variables = gather_with(
            &sources(&["a", "b"]),
            Duration::from_secs(10),
            move |name| {
                barrier.wait();
                Ok(values(&[(name, "read")]))
            },
        );

        assert_eq!(2, variables.len(), "{variables:?}");
    }

    #[test]
    fn a_later_source_overrides_an_earlier_one() {
        let variables = gather_with(
            &sources(&["slow", "fast"]),
            Duration::from_secs(10),
            |name| {
                if name == "slow" {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Ok(values(&[("ship", name)]))
            },
        );

        assert_eq!("fast", variables["ship"]);
    }

    #[test]
    fn a_source_without_an_answer_is_left_out_after_the_timeout() {
        let started = Instant::now();

        let variables = gather_with(
            &sources(&["stuck", "quick"]),
            Duration::from_millis(200),
            |name| {
                if name == "stuck" {
                    std::thread::sleep(Duration::from_secs(60));
                }
                Ok(values(&[(name, "read")]))
            },
        );

        assert_eq!(vec!["quick"], variables.keys().collect::<Vec<_>>());
        assert!(started.elapsed() < Duration::from_secs(30));
    }

    #[test]
    fn a_failing_or_crashing_source_keeps_the_others() {
        let variables = gather_with(
            &sources(&["fails", "panics", "works"]),
            Duration::from_secs(10),
            |name| match name {
                "fails" => anyhow::bail!("nope"),
                "panics" => panic!("boom"),
                _ => Ok(values(&[(name, "read")])),
            },
        );

        assert_eq!(vec!["works"], variables.keys().collect::<Vec<_>>());
    }
}
