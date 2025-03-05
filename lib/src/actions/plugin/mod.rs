#[allow(clippy::module_inception)]
pub mod plugin;

use std::{
    collections::HashMap,
    ffi::OsStr,
    fmt::{Display, Formatter, Result as FmtResult},
    fs::read_to_string,
    hash::{Hash, Hasher},
    sync::LazyLock,
};

use anyhow::{Context, Result};
use dirs_next::data_local_dir;
use tealr::mlu::mlua::{
    Error as LuaError, FromLua, Function, Lua, Result as LuaResult, Value as LuaValue,
};
use tracing::error;
use walkdir::WalkDir;

pub use plugin::Plugin;

#[derive(Debug, Clone)]
pub struct PluginSpec {
    pub name: String,
    pub is_privileged: bool,
    pub func: Function,
    pub lua: Lua,
}

impl FromLua for PluginSpec {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let table = value
            .as_table()
            .ok_or_else(|| LuaError::external("Cannot read plugin"))?;

        let name = table
            .get::<String>("name")
            .or_else(|_| table.get::<String>(1))
            .map_err(|_| LuaError::external("Missing name defined in plugin"))?;

        let func = table.get::<Function>("func").or_else(|_| {
            table
                .sequence_values()
                .filter_map(Result::ok)
                .find_map(|v| match v {
                    Some(LuaValue::Function(f)) => Some(f),
                    _ => None,
                })
                .ok_or_else(|| LuaError::runtime("No function found in plugin"))
        })?;

        Ok(Self {
            name,
            is_privileged: table.get("is_privileged").unwrap_or(false),
            func,
            lua: lua.clone(),
        })
    }
}

impl PluginSpec {
    pub fn call(&self, args: LuaValue) -> Result<LuaValue> {
        self.func.call(args).context("Failed to call plugin")
    }
}

impl Display for PluginSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Plugin: {}", self.name)
    }
}

impl PartialEq for PluginSpec {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && other.is_privileged == self.is_privileged
    }
}

impl Eq for PluginSpec {}
impl Hash for PluginSpec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.is_privileged.hash(state);
    }
}

fn load_plugins() -> Result<HashMap<String, PluginSpec>> {
    let dir = data_local_dir()
        .context("Cannot access local data directory")?
        .join("comtrya")
        .join("plugins");
    println!("Dir: {dir:?}");

    let specs = WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file() && entry.path().extension() == Some(OsStr::new("lua"))
        })
        .filter_map(|entry| {
            let path = entry.path();
            let Ok(content) = read_to_string(path).inspect_err(|e| error!("Failed to read {e}"))
            else {
                return None;
            };

            let lua = unsafe { Lua::unsafe_new() };
            lua.load(content)
                .eval::<PluginSpec>()
                .map(|spec| (spec.name.clone(), spec))
                .inspect_err(|e| error!("Failed to eval lua in {path:?}: {e}",))
                .ok()
        })
        .collect::<HashMap<String, PluginSpec>>();

    Ok(specs)
}

static PLUGINS: LazyLock<HashMap<String, PluginSpec>> =
    LazyLock::new(|| load_plugins().unwrap_or_default());

// impl FromLuaMulti for PluginSpec {
//     fn from_lua_multi(values: mlua::MultiValue, lua: &mlua::Lua) -> mlua::Result<Self> {
//         if let Some(Value::Table(table)) = values.iter().next() {
//             Ok(Self {
//                 name: table.get(0)?,
//                 is_privileged: table.get("is_privileged")?,
//                 func: table.get(2)?,
//                 lua,
//             })
//         } else {
//             Err(mlua::Error::RuntimeError("Expected table".to_string()))
//         }
//     }
// }
