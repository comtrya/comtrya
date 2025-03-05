use anyhow::Result;
use serde_json::Value as JsonValue;
use tealr::mlu::mlua::{Error as LuaError, Lua, Value as LuaValue};

#[allow(dead_code)]
pub fn lua_value_to_json(value: LuaValue) -> JsonValue {
    match value {
        LuaValue::Nil => JsonValue::Null,
        LuaValue::Boolean(b) => JsonValue::Bool(b),
        LuaValue::Integer(i) => JsonValue::Number(i.into()),
        LuaValue::Number(n) => {
            JsonValue::Number(serde_json::Number::from_f64(n).unwrap_or(0.into()))
        }
        LuaValue::String(s) => JsonValue::String(s.to_string_lossy()),
        LuaValue::Table(t) => {
            if t.clone().pairs::<LuaValue, LuaValue>().count() > 0
                && t.clone().pairs::<i64, LuaValue>().count() == 0
            {
                // Treat as object
                let mut map = serde_json::Map::new();
                for pair in t.pairs::<LuaValue, LuaValue>() {
                    let (k, v) = pair.unwrap();
                    if let LuaValue::String(key) = k {
                        map.insert(key.to_string_lossy(), lua_value_to_json(v));
                    }
                }
                JsonValue::Object(map)
            } else {
                // Treat as array
                let mut array = Vec::new();
                for pair in t.pairs::<i64, LuaValue>() {
                    let (_, v) = pair.unwrap();
                    array.push(lua_value_to_json(v));
                }
                JsonValue::Array(array)
            }
        }
        _ => JsonValue::Null,
    }
}

pub fn json_to_lua_value(json: JsonValue, lua: &Lua) -> Result<LuaValue, LuaError> {
    match json {
        JsonValue::Null => Ok(LuaValue::Nil),
        JsonValue::Bool(b) => Ok(LuaValue::Boolean(b)),
        JsonValue::Number(n) => {
            if n.is_i64() {
                let value = n.as_i64().unwrap();
                Ok(LuaValue::Integer(value))
            } else {
                Ok(LuaValue::Number(n.as_f64().unwrap()))
            }
        }
        JsonValue::String(s) => Ok(LuaValue::String(lua.create_string(&s)?)),
        JsonValue::Array(arr) => {
            let table = lua.create_table()?;
            for (i, value) in arr.into_iter().enumerate() {
                table.set(i + 1, json_to_lua_value(value, lua)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        JsonValue::Object(map) => {
            let table = lua.create_table()?;
            for (k, v) in map {
                table.set(k, json_to_lua_value(v, lua)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}
