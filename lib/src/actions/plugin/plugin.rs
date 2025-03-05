use std::fmt::{self, Display};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::info;

use super::{PluginSpec, PLUGINS};
use crate::{
    actions::Action, atoms::plugin::PluginExec, contexts::Contexts, manifests::Manifest,
    steps::Step,
};
use schemars::JsonSchema;

#[derive(JsonSchema, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Plugin {
    #[serde(flatten)]
    pub spec: JsonValue,
    #[serde(skip)]
    pub runtime: Option<PluginSpec>,
}

impl PartialEq for Plugin {
    fn eq(&self, other: &Self) -> bool {
        self.spec == other.spec
    }
}

impl Plugin {
    fn name(&self) -> String {
        self.spec["_name"].as_str().unwrap_or_default().to_string()
    }
}

impl Eq for Plugin {}

impl Display for Plugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Plugin: {self:?}")
    }
}

impl Action for Plugin {
    // FIXME: this should be in the plugin spec
    fn summarize(&self) -> String {
        "I am a plugin".to_string()
    }

    fn plan(&self, _manifest: &Manifest, _context: &Contexts) -> Result<Vec<Step>> {
        info!("Plugin Config: {}", self);

        Ok(vec![Step {
            atom: Box::new(PluginExec {
                runtime: PLUGINS
                    .get(&self.name())
                    .context("Plugin not found")?
                    .clone(),
                spec: self.spec.clone(),
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
