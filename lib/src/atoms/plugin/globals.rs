use std::{thread::sleep, time::Duration};

use anyhow::{anyhow, Error, Result};
use tealr::{
    mlu::{
        mlua::{Lua, Table},
        TealData, UserData,
    },
    ToTypename,
};

use crate::{contexts::Contexts, utilities::lua::json_to_lua};

#[derive(Debug, ToTypename, UserData)]
pub struct LuaUserError(String);

impl TealData for LuaUserError {}

impl From<LuaUserError> for Error {
    fn from(err: LuaUserError) -> Self {
        anyhow!("{}", err.0)
    }
}

pub fn setup_globals(lua: &Lua, contexts: Contexts) -> Result<()> {
    let globals = lua.globals();

    globals.set(
        "sleep",
        lua.create_function(|_, seconds: u64| {
            let duration = Duration::from_millis(seconds);
            sleep(duration);
            Ok(())
        })?,
    )?;

    globals.set(
        "Error",
        lua.create_function(|lua, message: String| lua.create_userdata(LuaUserError(message)))?,
    )?;

    add_context(lua, contexts, &globals)?;

    Ok(())
}

fn add_context(lua: &Lua, contexts: Contexts, globals: &Table) -> Result<()> {
    let table = lua.create_table()?;

    for (key, inner_map) in contexts {
        let inner_table = lua.create_table()?;

        for (k, value) in inner_map {
            match serde_json::to_value(value)
                .map_err(Error::from)
                .map(|json| json_to_lua(&json, lua).map_err(Error::from))
            {
                Ok(Ok(lua_val)) => inner_table.set(k, lua_val)?,
                Ok(Err(e)) => Err(anyhow!("Converting {key}: {k} to Lua {e}"))?,
                Err(e) => Err(anyhow!("Converting {k} to Lua {e}"))?,
            }
        }

        table.set(key, inner_table)?;
    }

    globals.set("contexts", table).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tealr::mlu::mlua::Error as LuaError;

    use super::*;

    #[test]
    fn can_get_context_from_lua() -> Result<(), LuaError> {
        let lua = Lua::new();

        add_context(
            &lua,
            BTreeMap::from([(
                String::from("foo"),
                BTreeMap::from([(String::from("bar"), String::from("baz").into())]),
            )]),
            &lua.globals(),
        )?;

        lua.load(
            r#"
            assert(contexts ~= nil)
            assert(contexts.foo ~= nil)
            assert(contexts.foo.bar == "baz")
            "#,
        )
        .exec()
    }
}
