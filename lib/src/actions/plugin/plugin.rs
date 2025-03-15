use std::fmt::{self, Display};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::info;

use super::{PluginSpec, PLUGINS};
use crate::{
    actions::Action, atoms::plugin::PluginExec, contexts::Contexts, manifests::Manifest,
    steps::Step, utilities::lua::json_to_lua_value,
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
    fn summarize(&self) -> String {
        PLUGINS
            .get(&self.name())
            .and_then(|p| p.summary.clone())
            .unwrap_or(format!("Ran {} plugin", self.name()))
    }

    fn plan(&self, _manifest: &Manifest, _context: &Contexts) -> Result<Vec<Step>> {
        info!("Plugin Config: {}", self);

        let runtime = PLUGINS
            .get(&self.name())
            .context("Plugin not found")?
            .to_owned();

        runtime.plan.as_ref().and_then(|plan| {
            plan.call::<()>(json_to_lua_value(self.spec.clone(), &runtime.lua).unwrap_or_default())
                .ok()
        });

        Ok(vec![Step {
            atom: Box::new(PluginExec {
                runtime,
                spec: self.spec.clone(),
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
