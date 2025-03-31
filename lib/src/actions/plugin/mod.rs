#[allow(clippy::module_inception)]
mod plugin;
mod plugin_spec;

use std::{collections::BTreeMap, sync::OnceLock};

use parking_lot::Mutex;
use tealr::mlu::mlua::{Lua, Result, StdLib};

use crate::{atoms::plugin::PluginRuntimeSpec, utilities::lua::LuaRuntime};
use plugin::{RepoOrDir, Source};

pub use plugin::Plugin;
pub use plugin_spec::PluginSpec;

fn get_plugin(source: RepoOrDir) -> Result<PluginRuntimeSpec> {
    let mut plugins = PLUGINS.get_or_init(|| Mutex::new(BTreeMap::new())).lock();

    if let Some(spec) = plugins.get(&source).cloned() {
        return Ok(spec);
    }

    let lua = unsafe {
        let lua_init = Lua::unsafe_new();
        lua_init.load_std_libs(StdLib::ALL)?;

        lua_init
    };
    let runtime_spec = PluginRuntimeSpec {
        spec: lua.load(source.source()?).eval()?,
        lua: LuaRuntime(lua),
    };

    plugins.insert(source, runtime_spec.clone());

    Ok(runtime_spec)
}

static PLUGINS: OnceLock<Mutex<BTreeMap<RepoOrDir, PluginRuntimeSpec>>> = OnceLock::new();
