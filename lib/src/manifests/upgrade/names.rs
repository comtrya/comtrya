//! Format 2 knows one name for each action and field. Old names, like `cmd.run` for
//! `command.run` or `source` for `from`, become that name

use super::super::v2::aliases::{aliased_action, field_aliases, version_1_fields};
use crate::actions::Actions;
use serde_yaml_ng::{Mapping, Value};

/// Gives the action entry `action` at `at`, and its variants, their one name for the action and
/// each field. Returns the action's name, empty for an entry without one
pub(super) fn rename(
    action: &mut Mapping,
    at: &str,
    warnings: &mut Vec<String>,
) -> Result<String, String> {
    let Some(name) = action.get("action").and_then(Value::as_str) else {
        return Ok(String::new());
    };
    let name = aliased_action(name).unwrap_or(name).to_string();
    action.insert(Value::from("action"), Value::from(name.as_str()));

    let Some(schema) = Actions::schema(&name) else {
        return Ok(name);
    };
    let mut aliases: Vec<Alias> = field_aliases(schema.as_value())
        .into_iter()
        .map(|(alias, field)| (alias, field, false))
        .collect();
    aliases.extend(version_1_fields(&name).map(|(alias, field)| (alias, field, true)));

    rename_fields(action, &aliases, at, warnings)?;
    if let Some(Value::Sequence(variants)) = action.get_mut("variants") {
        for (index, variant) in variants.iter_mut().enumerate() {
            if let Value::Mapping(variant) = variant {
                let at = format!("{at}.variants[{index}]");
                rename_fields(variant, &aliases, &at, warnings)?;
            }
        }
    }
    Ok(name)
}

/// An old name of a field, as (old name, the one name, whether format 1 had both as fields)
type Alias = (String, String, bool);

/// Renames each key of `fields` that is an old name, keeping the order of keys
fn rename_fields(
    fields: &mut Mapping,
    aliases: &[Alias],
    at: &str,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    for (alias, field, both_fields) in aliases {
        if !fields.contains_key(alias.as_str()) {
            continue;
        }
        if fields.contains_key(field.as_str()) {
            if !both_fields {
                return Err(format!(
                    "{at}: `{alias}` and `{field}` are the same field, set only one"
                ));
            }
            // Format 1's `file.link` reads `source` and `target` before `from` and `to`
            warnings.push(format!(
                "{at}: `{alias}` and `{field}` are both set; `{field}` gets the value of `{alias}`, which version 1 used"
            ));
            fields.shift_remove(field.as_str());
        }
        *fields = renamed(std::mem::take(fields), alias, field);
    }
    Ok(())
}

/// `fields` with the key `from` called `to`, in the same place
fn renamed(fields: Mapping, from: &str, to: &str) -> Mapping {
    fields
        .into_iter()
        .map(|(key, value)| {
            if key.as_str() == Some(from) {
                (Value::from(to), value)
            } else {
                (key, value)
            }
        })
        .collect()
}
