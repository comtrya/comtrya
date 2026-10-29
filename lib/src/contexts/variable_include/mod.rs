use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use reqwest::Url;

use crate::config::Config;
use crate::runner::SystemRunner;
use tracing::warn;

pub mod dns;
pub mod file;
pub mod op;

/// Reads the variables of `include_variables` in `Comtrya.yaml`, the `include_variables` context.
/// A source that fails is logged, and only its variables are left out
pub fn gather(config: &Config) -> BTreeMap<String, String> {
    let mut contexts = HashMap::<String, String>::new();

    for source in config.include_variables.iter().flatten() {
        if let Err(err) = include(source, &mut contexts) {
            warn!("Can't read the variables of {source} in include_variables: {err}");
        }
    }

    contexts.into_iter().collect()
}

/// Adds the variables of one `include_variables` source to `contexts`
fn include(source: &str, contexts: &mut HashMap<String, String>) -> Result<()> {
    // Vault and item names may contain spaces, which a URL can't
    if source.starts_with("op://") {
        return op::item_values(source, &SystemRunner, contexts);
    }

    let url = Url::parse(source)?;
    match url.scheme() {
        "dns+txt" => dns::txt_record_values(&url, contexts),
        "file+toml" => file::toml_values(&url, contexts),
        "file+yaml" => file::yaml_values(&url, contexts),
        scheme => Err(anyhow::anyhow!("Unknown variable include scheme: {scheme}")),
    }
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
