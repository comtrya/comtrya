use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::{atoms::plugins::PluginAtom, steps::Step};

use super::Action;

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct PluginAction {
    pub name: String,
    pub arguments: Option<Vec<String>>,
}

impl Action for PluginAction {
    fn plan(
        &self,
        _: &crate::manifests::Manifest,
        _: &crate::contexts::Contexts,
        plugin_functions: &crate::plugins::PluginFunctions,
    ) -> Vec<crate::steps::Step> {
        let mut steps = vec![];

        if let Some(function) = plugin_functions.functions.get(&self.name) {
            debug!("Loaded plugin {}", self.name);

            steps.push(Step {
                atom: Box::new(PluginAtom {
                    name: self.name.clone(),
                    arguments: self.arguments.clone().unwrap_or_default(),
                    function: function.clone(),
                }),
                initializers: vec![],
                finalizers: vec![],
            })
        } else {
            info!("No plugin named '{}' found", self.name);
        }

        steps
    }
}
