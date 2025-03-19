use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
};

use tealr::{
    mlu::mlua::{Error as LuaError, FromLua, Function, Lua, Table, Value},
    ToTypename,
};

use crate::utilities::lua::LuaFunction;

#[derive(Clone, Debug, ToTypename, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginAction {
    pub plan: Option<LuaFunction>,
    pub exec: LuaFunction,
    pub is_privileged: bool,
}

#[derive(Clone, Debug, ToTypename, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginSpec {
    pub name: String,
    pub summary: Option<String>,
    pub actions: BTreeMap<String, PluginAction>,
}

impl PluginSpec {
    pub fn get_action(&self, name: &str) -> Option<&PluginAction> {
        self.actions.get(name)
    }

    pub fn exec_action(&self, action_name: &str, args: Value) -> Result<Value, LuaError> {
        self.get_action(action_name)
            .map(|action| action.exec.call(args))
            .unwrap_or_else(|| Err(LuaError::external("No action found")))
    }

    pub fn plan_action(&self, name: &str, args: Value) -> Option<Result<Value, LuaError>> {
        self.get_action(name)
            .and_then(|action| action.plan.as_ref())
            .map(|plan| plan.call(args))
    }
}

impl Display for PluginSpec {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "plugin.{}", self.name)
    }
}

impl FromLua for PluginSpec {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self, LuaError> {
        let Value::Table(table) = value else {
            return Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: String::from("PluginSpec"),
                message: Some("Expected a Lua table but got a different type".to_string()),
            });
        };

        let actions: BTreeMap<String, PluginAction> = table
            .get::<Table>("actions")?
            .pairs::<String, Table>()
            .flatten()
            .map(|(key, action)| {
                let action = PluginAction {
                    plan: action.get::<Function>("plan").map(LuaFunction).ok(),
                    exec: action.get::<Function>("exec").map(LuaFunction)?,
                    is_privileged: action.get("is_privileged").unwrap_or(false),
                };
                Ok((key, action))
            })
            .collect::<Result<_, LuaError>>()?;

        Ok(PluginSpec {
            name: table.get("name")?,
            summary: table.get("summary").ok(),
            actions,
        })
    }
}
