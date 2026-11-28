//! The schema of manifest version 2, made from the one of version 1: the same actions and
//! fields, with one name each, `title` and `register`, variants that set only what differs,
//! and a template in place of a number or a boolean

use serde_json::{json, Value};

/// Real fields that version 2 doesn't accept, by action: the version 1 names of `from` and
/// `to`. The aliases of other fields aren't properties anyway, only `x-aliases`
const V1_ONLY_FIELDS: [(&str, &str); 2] = [("file.link", "source"), ("file.link", "target")];

/// Fields that version 2 requires instead, by action
const V2_REQUIRED: [(&str, &str); 2] = [("file.link", "from"), ("file.link", "to")];

/// Text that is one template and nothing else, like `{{ variables.mode }}`. Version 2 reads
/// it as a number or a boolean after rendering when its field can't hold text
const ONE_TEMPLATE: &str = r"^\s*\{\{.*\}\}\s*$";

/// Keys of an action that a template doesn't type: `where` isn't rendered, `variants` is a list
/// of actions
const NOT_TYPED: [&str; 2] = ["where", "variants"];

/// The version 2 schema from the version 1 `schema`
pub fn from_v1(mut schema: Value) -> Value {
    let defs = schema["$defs"].clone();
    version_two(&mut schema);

    for (name, def) in action_defs(&defs) {
        let variant = variant_def(&defs[&def]);
        if let Some(action) = schema["$defs"].get_mut(&def) {
            fields(action, &name, &defs);
            add_property(action, "title", title());
            if name == "command.run" {
                add_property(action, "register", register());
            }
        }
        if let Some(variant) = variant.and_then(|variant| schema["$defs"].get_mut(&variant)) {
            fields(variant, &name, &defs);
            // A variant takes what it doesn't set from its action
            if let Some(fields) = variant.as_object_mut() {
                fields.remove("required");
            }
        }
    }
    one_name_each(&mut schema["$defs"]["Actions"]);
    schema
}

/// `version: 2` is required, and the schema says it's for version 2
fn version_two(schema: &mut Value) {
    schema["title"] = json!("Manifest (version 2)");
    schema["properties"]["version"] = json!({
        "description": "The manifest format: `2` for this schema. Version 1 manifests use comtrya.dev/schema.json",
        "const": 2
    });
    let required = &mut schema["required"];
    if !required.is_array() {
        *required = json!([]);
    }
    if let Some(required) = required.as_array_mut() {
        required.push(json!("version"));
    }
}

/// Each action's name and the definition of its fields, from the `Actions` list
fn action_defs(defs: &Value) -> Vec<(String, String)> {
    let actions = defs["Actions"]["oneOf"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    actions
        .iter()
        .filter_map(|action| {
            let name = action["properties"]["action"]["const"].as_str()?;
            let def = definition(action["$ref"].as_str()?)?;
            Some((name.to_string(), def.to_string()))
        })
        .collect()
}

/// The definition a `$ref` like `#/$defs/Variant3` points to
fn definition(reference: &str) -> Option<&str> {
    reference.strip_prefix("#/$defs/")
}

/// The definition of the variants of an action
fn variant_def(action: &Value) -> Option<String> {
    let reference = action["properties"]["variants"]["items"]["$ref"].as_str()?;
    definition(reference).map(str::to_string)
}

/// The fields of the action `name` (or one of its variants) as version 2 has them
fn fields(def: &mut Value, name: &str, defs: &Value) {
    let Some(properties) = def["properties"].as_object_mut() else {
        return;
    };
    for (action, field) in V1_ONLY_FIELDS {
        if action == name {
            properties.remove(field);
        }
    }
    for (key, property) in properties.iter_mut() {
        if let Some(property) = property.as_object_mut() {
            property.remove("x-aliases");
        }
        if !NOT_TYPED.contains(&key.as_str()) && !holds_text(property, defs) {
            *property = or_template(property.take());
        }
    }
    required_in_v2(def, name);
}

/// Replaces the version 1 names in `required` with the version 2 ones
fn required_in_v2(def: &mut Value, name: &str) {
    let required: Vec<&str> = V2_REQUIRED
        .iter()
        .filter(|(action, _)| *action == name)
        .map(|(_, field)| *field)
        .collect();
    if !required.is_empty() {
        def["required"] = json!(required);
    }
}

/// Whether a value of `schema` can be text, following `$ref`, `anyOf` and `oneOf`
fn holds_text(schema: &Value, defs: &Value) -> bool {
    let is_string = |value: &Value| value.as_str() == Some("string");
    let type_is_text = match &schema["type"] {
        Value::Array(types) => types.iter().any(is_string),
        other => is_string(other),
    };
    let reference_is_text = schema["$ref"]
        .as_str()
        .and_then(definition)
        .is_some_and(|def| holds_text(&defs[def], defs));
    let any_is_text = ["anyOf", "oneOf"].iter().any(|key| {
        schema[*key]
            .as_array()
            .is_some_and(|schemas| schemas.iter().any(|schema| holds_text(schema, defs)))
    });
    let enum_is_text = schema["enum"]
        .as_array()
        .is_some_and(|values| values.iter().any(Value::is_string))
        || schema["const"].is_string();
    type_is_text || reference_is_text || any_is_text || enum_is_text
}

/// `field`, or one template that renders to it
fn or_template(mut field: Value) -> Value {
    let description = field
        .as_object_mut()
        .and_then(|field| field.remove("description"));
    let mut either = json!({
        "anyOf": [field, { "type": "string", "pattern": ONE_TEMPLATE }]
    });
    if let Some(description) = description {
        either["description"] = description;
    }
    either
}

/// Adds the property `key` to the definition of an action
fn add_property(def: &mut Value, key: &str, property: Value) {
    if let Some(properties) = def["properties"].as_object_mut() {
        properties.insert(key.to_string(), property);
    }
}

/// `title`, shown in the log instead of the generated summary
fn title() -> Value {
    json!({
        "description": "Shown in the log instead of the generated summary",
        "type": "string"
    })
}

/// `register`, only on `command.run`
fn register() -> Value {
    json!({
        "description": "Keeps what the command printed, for later actions to read as `registered.<name>.stdout`, `.stderr` and `.exit_code`",
        "type": "string",
        "pattern": "^[A-Za-z0-9_]+$"
    })
}

/// The descriptions of the actions name only the one name, not the version 1 aliases
fn one_name_each(actions: &mut Value) {
    for action in actions["oneOf"].as_array_mut().into_iter().flatten() {
        let Some(description) = action["description"].as_str() else {
            continue;
        };
        let name = description
            .split_once(", or ")
            .map_or(description, |(name, _)| name);
        action["description"] = json!(name);
    }
}
