//! Format 2 knows one name for each action and each field. The aliases of format 1, like
//! `cmd.run` for `command.run` or `source` for `from`, are errors that name the one to use

use crate::actions::Actions;
use serde_json::Value as Json;
use serde_yaml_ng::Value;

/// Fails on the first action or field in `document` that uses an alias
pub(super) fn check(document: &Value) -> Result<(), String> {
    let actions = document.get("actions").and_then(Value::as_sequence);

    for (index, action) in actions.into_iter().flatten().enumerate() {
        check_action(action).map_err(|err| format!("actions[{index}]: {err}"))?;
    }
    Ok(())
}

/// Fails if `action`, one entry of `actions`, or one of its variants uses an alias
fn check_action(action: &Value) -> Result<(), String> {
    let Some(name) = action.get("action").and_then(Value::as_str) else {
        return Ok(());
    };
    if let Some(actual) = aliased_action(name) {
        return Err(format!(
            "`{name}` is another name for `{actual}`; manifest version 2 only knows `{actual}`"
        ));
    }
    let Some(schema) = Actions::schema(name) else {
        return Ok(());
    };
    let mut aliases = field_aliases(schema.as_value());
    aliases.extend(version_1_fields(name));

    check_fields(action, &aliases)?;
    let variants = action.get("variants").and_then(Value::as_sequence);
    for (index, variant) in variants.into_iter().flatten().enumerate() {
        check_fields(variant, &aliases).map_err(|err| format!("variant {}: {err}", index + 1))?;
    }
    Ok(())
}

/// The action that `name` is an alias of, `None` for an action's own name or an unknown one
fn aliased_action(name: &str) -> Option<&'static str> {
    Actions::NAMES
        .iter()
        .find(|(_, aliases)| aliases.contains(&name))
        .map(|(actual, _)| *actual)
}

/// Fails on the first key of `object` that is an alias, see [`field_aliases`]
fn check_fields(object: &Value, aliases: &[(String, String)]) -> Result<(), String> {
    let keys = object
        .as_mapping()
        .into_iter()
        .flatten()
        .map(|(key, _)| key);

    for key in keys.filter_map(Value::as_str) {
        if let Some((_, field)) = aliases.iter().find(|(alias, _)| alias == key) {
            return Err(format!(
                "`{key}` is another name for `{field}`; manifest version 2 only knows `{field}`"
            ));
        }
    }
    Ok(())
}

/// Fields of format 1 that are their own fields, not serde aliases, but another name for a field
/// format 2 keeps, as (action, field, the one to use). `file.link` had `source`/`target` next to
/// `from`/`to`; format 2 says `from`/`to`, like the other actions
const VERSION_1_FIELDS: [(&str, &str, &str); 2] = [
    ("file.link", "source", "from"),
    ("file.link", "target", "to"),
];

/// The fields of the action `name` that format 2 doesn't know, as (field, the one to use)
fn version_1_fields(name: &str) -> impl Iterator<Item = (String, String)> + '_ {
    VERSION_1_FIELDS
        .iter()
        .filter(move |(action, _, _)| *action == name)
        .map(|(_, field, actual)| ((*field).to_string(), (*actual).to_string()))
}

/// The aliases of the fields of an action's schema, as (alias, field), from their `x-aliases`
fn field_aliases(schema: &Json) -> Vec<(String, String)> {
    let Some(properties) = schema.get("properties").and_then(Json::as_object) else {
        return vec![];
    };

    properties
        .iter()
        .flat_map(|(field, property)| {
            let aliases = property.get("x-aliases").and_then(Json::as_array);
            aliases
                .into_iter()
                .flatten()
                .filter_map(Json::as_str)
                .map(move |alias| (alias.to_string(), field.clone()))
        })
        .collect()
}
