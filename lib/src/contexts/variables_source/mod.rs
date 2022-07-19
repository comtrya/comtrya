use std::collections::HashMap;

use anyhow::Result;
use reqwest::Url;

use crate::{config::Config, contexts::Context, contexts::ContextProvider};

pub mod dns;
pub mod file;

pub struct VariablesSourceContextProvider<'a> {
    pub config: &'a Config,
}

impl<'a> ContextProvider for VariablesSourceContextProvider<'a> {
    fn get_prefix(&self) -> String {
        String::from("variables_source")
    }

    fn get_contexts(&self) -> Result<Vec<super::Context>> {
        let mut contexts = HashMap::<String, String>::new();

        if let Some(variable_sources) = &self.config.variables_source {
            for variable_source in variable_sources {
                let url = Url::parse(variable_source)?;

                if url.scheme() == "dns+txt" {
                    dns::txt_record_values(&url, &mut contexts)?;
                } else if url.scheme() == "file+toml" {
                    file::toml_values(&url, &mut contexts)?;
                } else if url.scheme() == "file+yaml" {
                    file::yaml_values(&url, &mut contexts)?;
                } else {
                    return Err(anyhow::anyhow!(
                        "Unknown variables source scheme: {}",
                        url.scheme()
                    ));
                }
            }
        }

        let contexts = contexts
            .into_iter()
            .map(|(key, value)| Context::KeyValueContext(key, value))
            .collect::<Vec<_>>();

        Ok(contexts)
    }
}
