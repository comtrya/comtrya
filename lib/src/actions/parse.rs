//! Reads actions from manifests in two steps, without `#[serde(flatten)]`: the `action` key
//! picks the action from the registry, then the frontend takes `where`, `variants` and
//! `continue_on_error` itself
//! and hands the remaining keys to the action's own struct
//!
//! The entry is read into a [`serde_yaml_ng::Value`] first, for YAML and TOML alike. That is
//! what flatten did internally too, so scalars like `0644` or `yes` come out the same

use super::{Actions, ConditionalVariantAction, Variant};
use crate::manifests::closest;
use serde::de::{DeserializeOwned, Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_yaml_ng::{Mapping, Value};

/// The keys of one action entry, or of one variant
type Fields = Mapping;

impl<'de> Deserialize<'de> for Actions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ActionsVisitor)
    }
}

/// Reads an action entry as a map, and parses it while the map is still open, so the error is
/// reported at the entry, not at the list of actions
struct ActionsVisitor;

impl<'de> Visitor<'de> for ActionsVisitor {
    type Value = Actions;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an action")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Actions, A::Error> {
        let mut fields = Fields::new();
        while let Some((key, value)) = map.next_entry::<Value, Value>()? {
            fields.insert(key, value);
        }

        let name: String = take(&mut fields, "action")
            .map_err(A::Error::custom)?
            .ok_or_else(|| A::Error::missing_field("action"))?;

        Actions::parse(&name, fields)
            .unwrap_or_else(|| Err(unknown_action(&name)))
            .map_err(A::Error::custom)
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for ConditionalVariantAction<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::from_fields(Fields::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

impl<T: DeserializeOwned> ConditionalVariantAction<T> {
    /// The action from its keys without `action`: `where`, `variants`, `continue_on_error`,
    /// and the action's own
    pub(super) fn from_fields(mut fields: Fields) -> Result<Self, serde_yaml_ng::Error> {
        let condition = take(&mut fields, "where")?;
        let continue_on_error = take(&mut fields, "continue_on_error")?.unwrap_or_default();
        let variants: Vec<Fields> = take(&mut fields, "variants")?.unwrap_or_default();

        Ok(Self {
            condition,
            continue_on_error,
            variants: variants
                .into_iter()
                .enumerate()
                .map(|(index, fields)| {
                    Variant::from_fields(fields).map_err(|err| {
                        serde_yaml_ng::Error::custom(format!("variant {}: {err}", index + 1))
                    })
                })
                .collect::<Result<_, _>>()?,
            action: T::deserialize(Value::Mapping(fields))?,
        })
    }
}

impl<T: DeserializeOwned> Variant<T> {
    /// A variant from its keys: `where`, and a complete action. In manifest format 1 a variant
    /// replaces the action, it doesn't add to it
    fn from_fields(mut fields: Fields) -> Result<Self, serde_yaml_ng::Error> {
        let condition = take(&mut fields, "where")?;

        Ok(Self::new(
            T::deserialize(Value::Mapping(fields))?,
            condition,
        ))
    }
}

/// Removes `key` from `fields` and reads it as a `T`. `None` if it's missing
fn take<T: DeserializeOwned>(
    fields: &mut Fields,
    key: &str,
) -> Result<Option<T>, serde_yaml_ng::Error> {
    fields
        .remove(key)
        .map(|value| T::deserialize(value).map_err(|err| Error::custom(format!("`{key}`: {err}"))))
        .transpose()
}

/// The error for an action nothing implements, with the closest known name if one is close.
/// Serde's own error would list every action
fn unknown_action(name: &str) -> serde_yaml_ng::Error {
    let names = Actions::NAMES
        .iter()
        .flat_map(|(name, aliases)| std::iter::once(*name).chain(aliases.iter().copied()));

    match closest(name, names) {
        Some(closest) => serde_yaml_ng::Error::custom(format!(
            "unknown action `{name}`, did you mean `{closest}`?"
        )),
        None => serde_yaml_ng::Error::custom(format!("unknown action `{name}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifests::Manifest;
    use pretty_assertions::assert_eq;

    fn error(yaml: &str) -> String {
        serde_yaml_ng::from_str::<Manifest>(yaml)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn a_misspelled_action_name_suggests_the_closest_action() {
        let yaml = "actions:\n  - action: directory.create\n    path: a\n  - action: file.cpy\n";

        let error = error(yaml);
        assert!(
            error.contains("unknown action `file.cpy`, did you mean `file.copy`?"),
            "{error}"
        );
        assert!(error.contains("actions[1]"), "{error}");
        assert!(error.contains("line 4"), "{error}");
    }

    #[test]
    fn an_action_name_that_is_nothing_like_an_action_has_no_suggestion() {
        let error = error("actions:\n  - action: frobnicate\n");

        assert!(error.contains("unknown action `frobnicate`"), "{error}");
        assert!(!error.contains("did you mean"), "{error}");
    }

    #[test]
    fn an_entry_without_action_is_an_error() {
        let error = error("actions:\n  - path: a\n");

        assert!(error.contains("missing field `action`"), "{error}");
    }

    #[test]
    fn a_missing_field_names_the_field() {
        let error = error("actions:\n  - action: file.copy\n    to: b\n");

        assert!(error.contains("missing field `from`"), "{error}");
    }

    #[test]
    fn where_and_variants_are_not_fields_of_the_action() {
        let yaml = r#"
actions:
  - action: directory.create
    path: /a
    where: os.name == "linux"
    variants:
      - where: os.name == "macos"
        path: /b
"#;
        let manifest: Manifest = serde_yaml_ng::from_str(yaml).unwrap();

        let Actions::DirectoryCreate(action) = &manifest.actions[0] else {
            panic!("not directory.create: {:?}", manifest.actions[0]);
        };
        assert_eq!(Some(r#"os.name == "linux""#), action.condition.as_deref());
        assert_eq!("/a", action.action.path);
        assert_eq!(
            Some(r#"os.name == "macos""#),
            action.variants[0].condition.as_deref()
        );
        assert_eq!("/b", action.variants[0].action.path);
    }

    #[test]
    fn a_variant_is_a_complete_action() {
        let yaml = r#"
actions:
  - action: file.copy
    from: a
    to: b
    variants:
      - where: "true"
        to: c
"#;

        let error = error(yaml);
        assert!(error.contains("variant 1: missing field `from`"), "{error}");
    }

    #[test]
    fn a_where_that_is_not_text_is_an_error() {
        let error = error("actions:\n  - action: directory.create\n    path: a\n    where: 5\n");

        assert!(error.contains("`where`"), "{error}");
    }

    #[test]
    fn toml_manifests_parse_the_same() {
        let toml = r#"
[[actions]]
action = "dir.create"
path = "/a"
where = "true"
"#;
        let manifest: Manifest = toml::from_str(toml).unwrap();

        assert_eq!("directory.create", manifest.actions[0].to_string());
    }
}
