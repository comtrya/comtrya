use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{atoms::plugin::ExecPlugin, plugins::Plugin, steps::Step};

use super::Action;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PluginCommand {
    pub name: String,
}

impl Action for PluginCommand {
    fn plan(
        &self,
        _: &crate::manifests::Manifest,
        contexts: &crate::contexts::Contexts,
        plugins: &[Plugin],
    ) -> Vec<crate::steps::Step> {
        if let Some(plugin) = plugins.iter().find(|plugin| plugin.name == self.name) {
            vec![Step {
                atom: Box::new(ExecPlugin {
                    plugin: plugin.to_owned(),
                    contexts: contexts.to_owned(),
                }),
                initializers: vec![],
                finalizers: vec![],
            }]
        } else {
            warn!("Cannot find plugin with name {}", self.name);

            vec![]
        }
    }
}
