#[allow(clippy::module_inception)]
pub mod plugin;

use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter, Result as FmtResult},
    hash::Hash,
    sync::{Mutex, OnceLock},
};

use anyhow::{anyhow, Context, Result};
use serde_with::serde_as;
use tealr::{
    mlu::mlua::{
        Error as LuaError, FromLua, Function as TrueLuaFunction, Lua, Table as LuaTable,
        Value as LuaValue,
    },
    ToTypename,
};

use crate::{
    atoms::plugin::PluginRuntimeSpec,
    utilities::lua::{LuaFunction, LuaRuntime},
};
pub use plugin::Plugin;
use plugin::{RepoOrDir, Source};

#[derive(Clone, Debug, ToTypename, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginSpecActions {
    pub plan: Option<LuaFunction>,
    pub exec: LuaFunction,
    pub is_privileged: bool,
}

#[serde_as]
#[derive(Clone, Debug, ToTypename, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginSpec {
    pub name: String,
    pub summary: Option<String>,
    pub actions: BTreeMap<String, PluginSpecActions>,
}

impl FromLua for PluginSpec {
    fn from_lua(value: LuaValue, _lua: &Lua) -> Result<Self, LuaError> {
        let table = match value {
            LuaValue::Table(t) => t,
            _ => {
                return Err(LuaError::FromLuaConversionError {
                    from: value.type_name(),
                    to: String::from("PluginSpec"),
                    message: Some("expected table".to_string()),
                })
            }
        };

        let mut actions = BTreeMap::new();

        for pair in table
            .get::<LuaTable>("actions")?
            .pairs::<String, LuaTable>()
        {
            let (key, action) = pair?;

            actions.insert(
                key,
                PluginSpecActions {
                    plan: action.get::<TrueLuaFunction>("plan").ok().map(LuaFunction),
                    exec: LuaFunction(action.get::<TrueLuaFunction>("exec")?),
                    is_privileged: action.get("is_privileged").unwrap_or(false),
                },
            );
        }

        Ok(PluginSpec {
            name: table.get("name")?,
            summary: table.get("summary").ok(),
            actions,
        })
    }
}

impl PluginSpec {
    pub fn get_impl(&self, name: &str) -> Option<&PluginSpecActions> {
        self.actions.get(name)
    }

    pub fn call(&self, name: &str, args: LuaValue) -> Result<LuaValue, LuaError> {
        match self.get_impl(name) {
            Some(impls) => impls.exec.call(args),
            None => Err(LuaError::external("Missing plugin arg")),
        }
    }

    pub fn plan(&self, name: &str, args: LuaValue) -> Option<Result<LuaValue, LuaError>> {
        match self.get_impl(name) {
            Some(impls) => impls.plan.as_ref(),
            None => None,
        }
        .map(|plan| plan.call(args))
    }
}

impl Display for PluginSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Plugin: {}", self.name)
    }
}

fn get_plugin(source: RepoOrDir) -> Result<PluginRuntimeSpec> {
    let Ok(mut plugins) = get_plugins().lock() else {
        return Err(anyhow!("Failed to get plugins"));
    };

    if let Some(spec) = plugins.get(&source) {
        return Ok(spec.clone());
    }

    let content = source.source()?;
    let lua = unsafe { Lua::unsafe_new() };

    let spec = lua
        .load(content)
        .eval::<PluginSpec>()
        .context("Failed deserialize plugin")?;

    let runtime_spec = PluginRuntimeSpec {
        lua: LuaRuntime(lua),
        spec,
    };
    plugins.insert(source, runtime_spec.clone());

    Ok(runtime_spec)
}

fn get_plugins() -> &'static Mutex<BTreeMap<RepoOrDir, PluginRuntimeSpec>> {
    PLUGINS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

static PLUGINS: OnceLock<Mutex<BTreeMap<RepoOrDir, PluginRuntimeSpec>>> = OnceLock::new();
