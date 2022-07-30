use anyhow::Result;

use super::ContextProvider;
use crate::contexts::Context;

pub struct EnvContextProvider;

impl ContextProvider for EnvContextProvider {
    fn get_prefix(&self) -> String {
        String::from("env")
    }

    fn get_contexts(&self) -> Result<Vec<super::Context>> {
        let mut contexts = vec![];

        for (key, value) in std::env::vars() {
            contexts.push(Context::KeyValueContext(key, value.into()));
        }

        Ok(contexts)
    }
}
