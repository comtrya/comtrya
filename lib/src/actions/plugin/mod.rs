#[allow(clippy::module_inception)]
mod plugin;

use std::{collections::BTreeMap, sync::OnceLock};

use parking_lot::Mutex;
use tealr::mlu::mlua::{Lua, Result, StdLib};

use crate::{
    atoms::plugin::{setup_globals, PluginSpec},
    contexts::Contexts,
};
pub use plugin::Plugin;
use plugin::{RepoOrDir, Source};

fn get_plugin(source: &RepoOrDir, contexts: Option<Contexts>) -> Result<PluginSpec> {
    let mut plugins = PLUGINS.get_or_init(|| Mutex::new(BTreeMap::new())).lock();

    if let Some(spec) = plugins.get(source).cloned() {
        return Ok(spec);
    }

    let lua = unsafe {
        let lua_init = Lua::unsafe_new();
        lua_init.load_std_libs(StdLib::ALL)?;

        lua_init
    };

    if let Some(contexts) = contexts {
        setup_globals(&lua, contexts)?;
    }

    let plugin_spec: PluginSpec = lua.load(source.source()?).eval()?;

    plugins.insert(source.to_owned(), plugin_spec.clone());

    Ok(plugin_spec)
}

static PLUGINS: OnceLock<Mutex<BTreeMap<RepoOrDir, PluginSpec>>> = OnceLock::new();
