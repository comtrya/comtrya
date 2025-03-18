use std::{
    hash::{Hash, Hasher},
    ops::{Deref, DerefMut},
};

use anyhow::Result;
use serde_json::Value as JsonValue;
use tealr::{
    mlu::{
        mlua::{Error as LuaError, Function, Lua, Value as LuaValue},
        FromToLua,
    },
    ToTypename,
};
use tracing::error;

use schemars::{
    gen::SchemaGenerator,
    schema::{InstanceType, Metadata, Schema, SchemaObject},
    JsonSchema,
};

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

pub fn json_to_lua_value(json: &JsonValue, lua: &Lua) -> Result<LuaValue, LuaError> {
    match json {
        JsonValue::Null => Ok(LuaValue::Nil),
        JsonValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
        JsonValue::Number(n) => n
            .is_i64()
            .then(|| n.as_i64().map(LuaValue::Integer))
            .unwrap_or_else(|| n.as_f64().map(LuaValue::Number))
            .ok_or_else(|| LuaError::external("Failed to convert number")),
        JsonValue::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        JsonValue::Array(arr) => lua.create_table().map(|table| {
            arr.iter()
                .filter_map(|value| json_to_lua_value(value, lua).ok())
                .enumerate()
                .for_each(|(i, v)| {
                    if let Err(e) = table.set(i + 1, v) {
                        error!("Failed to set value in table: {}", e);
                    };
                });
            LuaValue::Table(table)
        }),
        JsonValue::Object(map) => {
            let table = lua.create_table()?;
            for (k, v) in map {
                table.set(k.clone(), json_to_lua_value(v, lua)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

#[derive(Clone, Debug, FromToLua, ToTypename, PartialEq)]
pub struct LuaFunction(pub Function);

impl Eq for LuaFunction {}

impl PartialOrd for LuaFunction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.dump(false).cmp(&other.0.dump(false)))
    }
}

impl Ord for LuaFunction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.dump(false).cmp(&other.0.dump(false))
    }
}

impl Hash for LuaFunction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.dump(false).hash(state);
    }
}

impl Deref for LuaFunction {
    type Target = Function;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LuaFunction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl JsonSchema for LuaFunction {
    fn schema_name() -> String {
        "LuaFunction".to_string()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        let schema_obj = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            metadata: Some(Box::new(Metadata {
                description: Some("Lua function reference (opaque to schema)".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };

        Schema::Object(schema_obj)
    }
}

impl Default for LuaFunction {
    fn default() -> Self {
        let lua = Lua::new();
        let func = lua
            .create_function(|_, ()| Ok(()))
            .expect("Failed to create default function");
        LuaFunction(func)
    }
}

#[derive(Clone, Debug)]
pub struct LuaRuntime(pub Lua);

impl Deref for LuaRuntime {
    type Target = Lua;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LuaRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl JsonSchema for LuaRuntime {
    fn schema_name() -> String {
        "LuaRuntime".to_string()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        let schema_obj = SchemaObject {
            instance_type: Some(InstanceType::Object.into()),
            metadata: Some(Box::new(Metadata {
                description: Some("Lua runtime state (opaque to schema)".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        };

        Schema::Object(schema_obj)
    }
}

impl Default for LuaRuntime {
    fn default() -> Self {
        LuaRuntime(Lua::new())
    }
}
