use anyhow::Result;

use crate::{
    config::Config,
    contexts::{Context, ContextProvider},
};

pub struct VariablesContextProvider<'a> {
    pub config: &'a Config,
}

impl<'a> ContextProvider for VariablesContextProvider<'a> {
    fn get_prefix(&self) -> String {
        String::from("variables")
    }

    fn get_contexts(&self) -> Result<Vec<super::Context>> {
        let mut contexts = vec![];

        for (key, value) in self.config.variables.iter() {
            contexts.push(Context::KeyValueContext(
                key.to_owned(),
                value.to_owned().into(),
            ));
        }

        Ok(contexts)
    }
}
