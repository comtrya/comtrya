//! Numbers and booleans from templates. A rendered template is text, but a field like
//! `privileged` or `chmod` doesn't hold text. So a value that is one template expression and
//! nothing else, like `"{{ variables.root }}"`, is read as YAML again after rendering, when the
//! field it's in can't hold text. It becomes the number or boolean YAML reads, like `true`,
//! `644` or `0.5`; any other result stays text, and the field then reports the wrong type.
//!
//! Only the fields of actions and their variants are typed this way, not values nested deeper.
//! Text fields keep what was rendered, so `args: ["{{ variables.port }}"]` stays `"8080"`

use super::render::Rendered;
use crate::actions::Actions;
use serde_json::Value as Json;
use serde_yaml_ng::Value;

/// Keys of an action entry that aren't fields of the action
const NOT_FIELDS: [&str; 3] = ["action", "where", "variants"];

/// Turns the values in `rendered.whole` that sit in fields without text into numbers or
/// booleans, see the module docs
pub(super) fn apply(document: &mut Value, rendered: &Rendered) {
    let Some(actions) = document.get_mut("actions").and_then(Value::as_sequence_mut) else {
        return;
    };

    for (index, action) in actions.iter_mut().enumerate() {
        let name = action
            .get("action")
            .and_then(Value::as_str)
            .map(str::to_string);
        let Some(schema) = name.as_deref().and_then(Actions::schema) else {
            continue;
        };
        let at = format!("actions[{index}]");
        let schema = schema.as_value();

        fields(action, schema, &at, rendered);
        let variants = action.get_mut("variants").and_then(Value::as_sequence_mut);
        for (index, variant) in variants.into_iter().flatten().enumerate() {
            fields(
                variant,
                schema,
                &format!("{at}.variants[{index}]"),
                rendered,
            );
        }
    }
}

/// Types the fields of `object`, an action at `at` or one of its variants, by `schema`
fn fields(object: &mut Value, schema: &Json, at: &str, rendered: &Rendered) {
    let Some(fields) = object.as_mapping_mut() else {
        return;
    };

    for (key, value) in fields.iter_mut() {
        let Some(key) = key.as_str().filter(|key| !NOT_FIELDS.contains(key)) else {
            continue;
        };
        if !rendered.whole.contains(&format!("{at}.{key}")) {
            continue;
        }
        let property = schema
            .get("properties")
            .and_then(|properties| properties.get(key));
        if property.is_some_and(|property| holds_no_text(property, schema)) {
            retype(value);
        }
    }
}

/// Replaces `value`, rendered text, with the number or boolean YAML reads from it, if any
fn retype(value: &mut Value) {
    let Some(text) = value.as_str() else {
        return;
    };
    if let Ok(typed @ (Value::Bool(_) | Value::Number(_))) = serde_yaml_ng::from_str(text) {
        *value = typed;
    }
}

/// Whether the schema of `property` names its types and text isn't one of them. `root` has
/// the `$defs` that references point to
fn holds_no_text(property: &Json, root: &Json) -> bool {
    let types = types_of(property, root);
    !types.is_empty() && !types.contains(&"string")
}

/// The JSON types `property` allows, following `$ref`s and `anyOf`/`oneOf`. Empty if it allows
/// anything
fn types_of<'a>(property: &'a Json, root: &'a Json) -> Vec<&'a str> {
    if let Some(reference) = property.get("$ref").and_then(Json::as_str) {
        let target = reference
            .strip_prefix("#/$defs/")
            .and_then(|name| root.get("$defs")?.get(name));
        return target.map_or_else(Vec::new, |target| types_of(target, root));
    }
    for alternatives in ["anyOf", "oneOf"] {
        if let Some(alternatives) = property.get(alternatives).and_then(Json::as_array) {
            return alternatives
                .iter()
                .flat_map(|alternative| types_of(alternative, root))
                .collect();
        }
    }
    match property.get("type") {
        Some(Json::String(name)) => vec![name.as_str()],
        Some(Json::Array(names)) => names.iter().filter_map(Json::as_str).collect(),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;
    use crate::contexts::Contexts;
    use crate::manifests::{load_all, Loaded};
    use crate::values::Value;
    use std::collections::BTreeMap;
    use std::fs;

    /// Contexts with `variables.root` = `true`, `variables.mode` = `0755` and
    /// `variables.port` = `8080`, all text
    fn contexts() -> Contexts {
        let variables = [("root", "true"), ("mode", "0755"), ("port", "8080")]
            .map(|(key, value)| (key.to_string(), Value::from(value)));
        Contexts::from([(String::from("variables"), BTreeMap::from(variables))])
    }

    /// Loads a manifest directory with one file, `a.yaml` with `contents`
    fn load(contents: &str) -> Loaded {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.yaml"), contents).unwrap();
        load_all(dir.path().to_path_buf(), &contexts())
    }

    /// The one action of the one manifest in `contents`
    fn action(contents: &str) -> Actions {
        let mut loaded = load(contents);
        assert!(loaded.errors.is_empty(), "{:?}", loaded.errors);
        loaded.manifests.remove("a").unwrap().actions.remove(0)
    }

    #[test]
    fn a_template_gives_a_boolean_field_its_boolean() {
        let Actions::CommandRun(run) = action(
            "version: 2\nactions:\n  - action: command.run\n    command: id\n    privileged: \"{{ variables.root }}\"\n",
        ) else {
            panic!("not command.run");
        };
        assert!(run.action.privileged);
    }

    #[test]
    fn a_template_in_continue_on_error_is_a_boolean() {
        let Actions::CommandRun(run) = action(
            "version: 2\nactions:\n  - action: command.run\n    command: id\n    continue_on_error: \"{{ variables.root }}\"\n",
        ) else {
            panic!("not command.run");
        };
        assert!(run.continue_on_error);
    }

    #[test]
    fn a_templated_chmod_is_the_same_as_in_version_1() {
        let chmod = |contents: &str| {
            let Actions::FileCopy(copy) = action(contents) else {
                panic!("not file.copy");
            };
            copy.action.chmod
        };

        let v1 = chmod("actions:\n  - action: file.copy\n    from: a\n    to: b\n    chmod: {{ variables.mode }}\n");
        let v2 = chmod("version: 2\nactions:\n  - action: file.copy\n    from: a\n    to: b\n    chmod: \"{{ variables.mode }}\"\n");
        assert_eq!(0o755, v1);
        assert_eq!(v1, v2);
    }

    #[test]
    fn text_fields_keep_rendered_numbers_as_text() {
        let Actions::CommandRun(run) = action(
            "version: 2\nactions:\n  - action: command.run\n    command: \"{{ variables.port }}\"\n    args: [\"{{ variables.port }}\"]\n",
        ) else {
            panic!("not command.run");
        };
        assert_eq!("8080", run.action.command);
        assert_eq!(vec!["8080"], run.action.args);
    }

    #[test]
    fn variants_are_typed_too() {
        let Actions::CommandRun(run) = action(
            "version: 2\nactions:\n  - action: command.run\n    command: id\n    variants:\n      - where: \"true\"\n        command: id\n        privileged: \"{{ variables.root }}\"\n",
        ) else {
            panic!("not command.run");
        };
        assert!(run.variants[0].action.privileged);
    }

    #[test]
    fn more_than_one_expression_stays_text() {
        let loaded = load(
            "version: 2\nactions:\n  - action: command.run\n    command: id\n    privileged: \"{{ variables.root }} {{ variables.root }}\"\n",
        );

        let error = loaded.errors[0].kind.to_string();
        assert!(error.contains("expected a boolean"), "{error}");
    }

    #[test]
    fn registered_values_are_not_rendered_yet() {
        let loaded = load(
            "version: 2\nactions:\n  - action: command.run\n    command: \"{{ registered.x.stdout }}\"\n",
        );

        let error = loaded.errors[0].kind.to_string();
        assert!(
            error.contains("actions[0].command: `registered` values need `register`"),
            "{error}"
        );
    }
}
