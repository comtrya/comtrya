//! Variants in format 2 override fields of their action: a variant sets only what differs, and
//! takes the other fields from the action. Each key a variant sets replaces the action's key
//! whole, a map or a list too.
//!
//! Format 1 reads every variant as a complete action. So the merge happens here, before the
//! actions are read: each variant gets the action's fields that it doesn't set itself, and the
//! format 1 reader sees complete variants. Choosing the variant doesn't change: the first one
//! whose `where` is true, else the action's own `where`

use serde_yaml_ng::{Mapping, Value};

/// Keys of the action that a variant doesn't take: they decide about the whole action, not one
/// of its versions
const ACTION_ONLY: [&str; 5] = ["action", "where", "variants", "title", "continue_on_error"];

/// Keys a variant can't set, since they belong to the whole action. `where` is the variant's
/// own condition
const NOT_IN_VARIANTS: [&str; 4] = ["action", "variants", "title", "continue_on_error"];

/// Fills in each variant of each action in `document` with the action's fields it doesn't set
///
/// # Errors
///
/// When a variant isn't a map, or sets a key of the whole action, like `title`
pub(super) fn complete(document: &mut Value) -> Result<(), String> {
    let actions = document.get_mut("actions").and_then(Value::as_sequence_mut);

    for (index, action) in actions.into_iter().flatten().enumerate() {
        complete_action(action).map_err(|err| format!("actions[{index}].{err}"))?;
    }
    Ok(())
}

/// Fills in the variants of one action
fn complete_action(action: &mut Value) -> Result<(), String> {
    let Some(fields) = action.as_mapping_mut() else {
        return Ok(());
    };
    let Some(Value::Sequence(mut variants)) = fields.remove("variants") else {
        return Ok(());
    };

    let base = shared_fields(fields);
    for (index, variant) in variants.iter_mut().enumerate() {
        complete_variant(variant, &base).map_err(|err| format!("variants[{index}]: {err}"))?;
    }
    fields.insert(Value::from("variants"), Value::Sequence(variants));
    Ok(())
}

/// The fields of an action that its variants take
fn shared_fields(fields: &Mapping) -> Mapping {
    fields
        .iter()
        .filter(|(key, _)| !key.as_str().is_some_and(|key| ACTION_ONLY.contains(&key)))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// Adds the fields of `base` that `variant` doesn't set to it
fn complete_variant(variant: &mut Value, base: &Mapping) -> Result<(), String> {
    let Some(fields) = variant.as_mapping_mut() else {
        return Err(String::from("a variant has to be a map of keys"));
    };
    let set = fields.keys().filter_map(Value::as_str);
    if let Some(key) = set.into_iter().find(|key| NOT_IN_VARIANTS.contains(key)) {
        return Err(format!(
            "`{key}` belongs to the whole action, a variant can't set it"
        ));
    }

    for (key, value) in base {
        if !fields.contains_key(key) {
            fields.insert(key.clone(), value.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
