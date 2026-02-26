use std::{
    fmt::{self, Display},
    ops::Deref,
};

use anyhow::Result;
use tealr::mlu::mlua::{Function, Value as LuaValue};
#[allow(unused_imports)]
use tracing::{debug, error, trace};

use crate::{
    atoms::{plugin::PluginSpec, Atom, Outcome},
    utilities::lua::LuaRuntime,
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
        Some(self.cmp(other))
    }
}
impl Ord for PluginRuntimeSpec {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.spec.cmp(&other.spec)
    }
}

#[derive(Debug)]
pub struct PluginExec {
    pub exec_name: String,
    pub func: Function,
    pub spec: LuaValue,
    output: Option<String>,
}

impl PluginExec {
    pub fn new(exec_name: String, func: Function, spec: LuaValue) -> Self {
        Self {
            exec_name,
            func,
            spec,
            output: None,
        }
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
        // TODO: Should this accept a LuaUserError? I don't see a need for this to accept a string
        // when the plugin can just print what it needs anyway.
        self.output = self.func.call::<Option<String>>(&self.spec)?;
        Ok(())
    }

    fn output_string(&self) -> String {
        self.output.as_deref().unwrap_or_default().to_string()
    }
}
impl Display for PluginExec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.exec_name)
    }
}
