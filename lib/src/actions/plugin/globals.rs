use std::{thread::sleep, time::Duration};

use anyhow::Result;
use tealr::mlu::mlua::{Lua, Table, Value as LuaValue};

use crate::{contexts::Contexts, utilities::lua::json_to_lua_value};

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

    add_context(lua, contexts, globals)?;

    Ok(())
}

fn add_context(lua: &Lua, contexts: Contexts, globals: Table) -> Result<()> {
    globals.set(
        "get_context",
        lua.create_function(move |lua, key: String| match contexts.get(&key) {
            None => Ok(LuaValue::Nil),
            Some(v) => {
                let table = lua.create_table()?;
                for (k, val) in v.iter() {
                    let Ok(json_val) = serde_json::to_value(val) else {
                        eprintln!("Failed converting value to JSON");
                        continue;
                    };
                    let Ok(lua_val) = json_to_lua_value(&json_val, lua) else {
                        eprintln!("Failed converting to Lua");
                        continue;
                    };
                    let Ok(lua_key) = lua.create_string(k) else {
                        eprintln!("Failed creating key");
                        continue;
                    };
                    table.set(LuaValue::String(lua_key), lua_val)?;
                }
                Ok(LuaValue::Table(table))
            }
        })?,
    )?;

    Ok(())
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
            lua.globals(),
        )?;

        lua.load(
            r#"
            local context = get_context("foo")
            assert(context ~= nil)
            assert(context.bar == "baz")
            "#,
        )
        .exec()
    }
}
