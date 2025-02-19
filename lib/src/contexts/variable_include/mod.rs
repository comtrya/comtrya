use std::collections::HashMap;

use anyhow::Result;
use reqwest::Url;

use crate::{config::Config, contexts::Context, contexts::ContextProvider};

pub mod dns;
pub mod file;

pub struct VariableIncludeContextProvider<'a> {
    pub config: &'a Config,
}

impl ContextProvider for VariableIncludeContextProvider<'_> {
    fn get_prefix(&self) -> String {
        String::from("include_variables")
    }

    fn get_contexts(&self) -> Result<Vec<super::Context>> {
        let mut contexts = HashMap::<String, String>::new();

        if let Some(variable_includes) = &self.config.include_variables {
            for variable_include in variable_includes {
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

        let contexts = contexts
            .into_iter()
            .map(|(key, value)| Context::KeyValueContext(key, value.into()))
            .collect::<Vec<_>>();

        Ok(contexts)
    }
}
