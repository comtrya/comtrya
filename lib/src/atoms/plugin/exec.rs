use std::{
    fmt::{self, Display},
    ops::Deref,
};

use anyhow::Result;
#[allow(unused_imports)]
use tracing::{debug, error, trace};

use crate::{
    actions::PluginSpec,
    atoms::{Atom, Outcome},
    utilities::lua::{json_to_lua_value, LuaRuntime},
};

#[derive(Clone, Debug, Default)]
pub struct PluginRuntimeSpec {
    pub lua: LuaRuntime,
    pub spec: PluginSpec,
}

impl Deref for PluginRuntimeSpec {
    type Target = PluginSpec;

    fn deref(&self) -> &Self::Target {
        &self.spec
    }
}

impl PartialEq for PluginRuntimeSpec {
    fn eq(&self, other: &Self) -> bool {
        self.spec == other.spec
    }
}

impl Eq for PluginRuntimeSpec {}

impl PartialOrd for PluginRuntimeSpec {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.spec.cmp(&other.spec))
    }
}
impl Ord for PluginRuntimeSpec {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.spec.cmp(&other.spec)
    }
}

#[derive(Debug)]
pub struct PluginExec {
    pub plugin_impl: String,
    pub runtime: PluginRuntimeSpec,
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
        self.runtime.call(
            &self.plugin_impl,
            json_to_lua_value(&self.spec.clone(), &self.runtime.lua)?,
        )?;
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
