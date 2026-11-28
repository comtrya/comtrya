//! A format 1 variant is a whole action. A format 2 variant takes the fields it doesn't set from
//! its action, so it keeps only the fields that differ

use super::super::v2::variants::ACTION_ONLY;
use serde_yaml_ng::{Mapping, Value};

/// Leaves out the fields of each variant of the action entry `action` at `at` that are the same
/// as the action's
pub(super) fn rewrite(
    action: &mut Mapping,
    at: &str,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let base = action.clone();
    let Some(Value::Sequence(variants)) = action.get_mut("variants") else {
        return Ok(());
    };

    for (index, variant) in variants.iter_mut().enumerate() {
        let at = format!("{at}.variants[{index}]");
        let Value::Mapping(variant) = variant else {
            return Err(format!("{at}: a variant has to be a map of keys"));
        };
        drop_action_only(variant, &at, warnings);
        variant
            .retain(|key, value| key.as_str() == Some("where") || base.get(key) != Some(&*value));
    }
    Ok(())
}

/// Leaves out the keys of `variant` that only the whole action has in format 2. Format 1 didn't
/// read them in a variant either
fn drop_action_only(variant: &mut Mapping, at: &str, warnings: &mut Vec<String>) {
    for key in ACTION_ONLY.iter().filter(|key| **key != "where") {
        if variant.shift_remove(*key).is_some() {
            warnings.push(format!(
                "{at}: `{key}` is left out, it belongs to the whole action and version 1 didn't read it in a variant"
            ));
        }
    }
}
