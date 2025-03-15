use std::fmt::{self, Display};

use anyhow::Result;
#[allow(unused_imports)]
use tracing::{debug, error, trace};

use crate::{
    actions::PluginSpec,
    atoms::{Atom, Outcome},
    utilities::lua::json_to_lua_value,
};

#[derive(Debug)]
pub struct PluginExec {
    pub runtime: PluginSpec,
    pub spec: serde_json::Value,
}

impl PluginExec {
    pub fn run_function() {
        todo!()
    }
}

impl Atom for PluginExec {
    fn plan(&self) -> Result<Outcome> {
        Ok(Outcome {
            side_effects: vec![],
            should_run: true,
        })
    }

    fn execute(&mut self) -> Result<()> {
        self.runtime
            .call(json_to_lua_value(self.spec.clone(), &self.runtime.lua)?)?;
        Ok(())
    }

    fn output_string(&self) -> String {
        self.spec
            .get("action")
            .and_then(|action| action.as_str())
            .map(|s| s.to_string())
            .unwrap_or(String::from("unknown"))
    }
}
impl Display for PluginExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: This should come from the plugin spec right?
        write!(f, "Plugin: {:?}", self)
    }
}
