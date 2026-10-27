use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use reqwest::Url;

use crate::config::Config;

pub mod dns;
pub mod file;
pub mod op;

/// Reads the variables of `include_variables` in `Comtrya.yaml`, the `include_variables` context
pub fn gather(config: &Config) -> Result<BTreeMap<String, String>> {
    let mut contexts = HashMap::<String, String>::new();

    if let Some(variable_includes) = &config.include_variables {
        for variable_include in variable_includes {
            // Vault and item names may contain spaces, which a URL can't
            if variable_include.starts_with("op://") {
                op::item_values(variable_include, &mut contexts)?;
                continue;
            }

            let url = Url::parse(variable_include)?;

            if url.scheme() == "dns+txt" {
                dns::txt_record_values(&url, &mut contexts)?;
            } else if url.scheme() == "file+toml" {
                file::toml_values(&url, &mut contexts)?;
            } else if url.scheme() == "file+yaml" {
                file::yaml_values(&url, &mut contexts)?;
            } else {
                return Err(anyhow::anyhow!(
                    "Unknown variable include scheme: {}",
                    url.scheme()
                ));
            }
        }
    }

    Ok(contexts.into_iter().collect())
}
