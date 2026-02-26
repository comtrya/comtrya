use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    hash::{Hash, Hasher},
    ops::Deref,
    ptr::addr_of,
};

use tealr::{
    mlu::mlua::{Error as LuaError, FromLua, Function, Lua, Table, Value},
    ToTypename,
};

use crate::utilities::lua::LuaFunction;

#[derive(Debug, Clone, Default)]
pub struct ComparableLua(Lua);

impl Deref for ComparableLua {
    type Target = Lua;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for ComparableLua {
    fn hash<H: Hasher>(&self, state: &mut H) {
        addr_of!(self.0).hash(state)
    }
}

impl PartialEq for ComparableLua {
    fn eq(&self, other: &Self) -> bool {
        addr_of!(self) == addr_of!(other)
    }
}

impl Eq for ComparableLua {}

impl PartialOrd for ComparableLua {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComparableLua {
    fn cmp(&self, other: &Self) -> Ordering {
        addr_of!(self).cmp(&addr_of!(other))
    }
}

#[derive(Debug, Clone)]
pub struct ComparableFunction(Function);

impl Deref for ComparableFunction {
    type Target = Function;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for ComparableFunction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash based on pointer address or another stable identifier
        std::ptr::addr_of!(self.0).hash(state);
    }
}

impl PartialEq for ComparableFunction {
    fn eq(&self, other: &Self) -> bool {
        self.to_pointer() == other.to_pointer()
    }
}

impl Eq for ComparableFunction {}

impl PartialOrd for ComparableFunction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComparableFunction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_pointer().cmp(&other.to_pointer())
    }
}

#[derive(Clone, Debug, ToTypename, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum StringOrFunction {
    String(String),
    Function(ComparableFunction),
    #[default]
    Invalid,
}

#[derive(Clone, Debug, ToTypename, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginAction {
    pub plan: Option<LuaFunction>,
    pub exec: LuaFunction,
    pub is_privileged: bool,
}

#[derive(Clone, Debug, ToTypename, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PluginSpec {
    name: StringOrFunction,
    summary: Option<StringOrFunction>,
    pub actions: BTreeMap<String, PluginAction>,
    #[tealr(skip)]
    pub lua: ComparableLua,
}

impl PluginSpec {
    pub fn name(&self) -> String {
        match self.name {
            StringOrFunction::String(ref s) => s.clone(),
            StringOrFunction::Function(ref f) => f
                .call::<String>(Value::default())
                .unwrap_or(String::from("anonymous")),
            StringOrFunction::Invalid => String::from("anonymous"),
        }
    }

    pub fn summary(&self) -> String {
        match self.summary {
            Some(StringOrFunction::String(ref s)) => s.clone(),
            Some(StringOrFunction::Function(ref f)) => f
                .call::<String>(Value::default())
                .unwrap_or(String::from("anonymous")),
            _ => format!("Plugin {} completed.", self.name()),
        }
    }

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
        write!(f, "plugin.{}", self.name())
    }
}

impl FromLua for PluginSpec {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self, LuaError> {
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

        let name = table.get::<Value>("name").map(|n| match n {
            Value::String(s) => StringOrFunction::String(s.to_string_lossy()),
            Value::Function(f) => StringOrFunction::Function(ComparableFunction(f)),
            _ => StringOrFunction::Invalid,
        })?;

        let summary = table.get::<Value>("summary").ok().and_then(|s| match s {
            Value::String(s) => Some(StringOrFunction::String(s.to_string_lossy())),
            Value::Function(f) => Some(StringOrFunction::Function(ComparableFunction(f))),
            _ => None,
        });

        Ok(PluginSpec {
            name,
            summary,
            actions,
            lua: ComparableLua(lua.to_owned()),
        })
    }
}
