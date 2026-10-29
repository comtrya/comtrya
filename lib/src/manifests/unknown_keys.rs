//! Keys in a manifest that nothing reads. Manifest format 1 ignores them, so a typo like
//! `form:` instead of `from:` silently does nothing. They are warned about instead, with the
//! known key that's closest. The same goes for an action name nothing implements

use super::Manifest;
use crate::actions::Actions;
use schemars::{schema_for, Schema};
use serde_json::Value;

/// A key nothing reads
#[derive(Debug, PartialEq)]
pub struct UnknownKey {
    /// Where the key is, like `action 2 (file.copy)`
    pub at: String,
    /// The key
    pub key: String,
    /// The known key it's closest to, if one is close
    pub closest: Option<String>,
}

impl std::fmt::Display for UnknownKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{}` in {} is ignored", self.key, self.at)?;
        if let Some(closest) = &self.closest {
            write!(f, ", did you mean `{closest}`?")?;
        }
        Ok(())
    }
}

/// The keys an object with this schema reads: its properties, and the aliases in their
/// `x-aliases`
pub fn schema_keys(schema: Schema) -> Vec<String> {
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return vec![];
    };

    let aliases = properties
        .values()
        .filter_map(|property| property.get("x-aliases")?.as_array())
        .flatten()
        .filter_map(Value::as_str);
    properties
        .keys()
        .map(String::as_str)
        .chain(aliases)
        .map(String::from)
        .collect()
}

/// Every unknown key of `manifest`, a whole parsed manifest file: at the top, in actions and
/// in their variants
pub fn unknown_keys(manifest: &Value) -> Vec<UnknownKey> {
    let mut unknown = unknown_in(
        manifest,
        &schema_keys(schema_for!(Manifest)),
        "the manifest",
    );

    let actions = manifest.get("actions").and_then(Value::as_array);
    for (index, action) in actions.into_iter().flatten().enumerate() {
        let Some(name) = action.get("action").and_then(Value::as_str) else {
            continue;
        };
        let Some(mut known) = Actions::known_keys(name) else {
            continue;
        };
        let at = format!("action {} ({name})", index + 1);

        known.retain(|key| key != "variants");
        let variants = action.get("variants").and_then(Value::as_array);
        for (index, variant) in variants.into_iter().flatten().enumerate() {
            let at = format!("variant {} of {at}", index + 1);
            unknown.extend(unknown_in(variant, &known, &at));
        }

        known.extend([String::from("action"), String::from("variants")]);
        unknown.extend(unknown_in(action, &known, &at));
    }

    unknown
}

/// The keys of `object` that aren't `known`, if it is an object
fn unknown_in(object: &Value, known: &[String], at: &str) -> Vec<UnknownKey> {
    let keys = object
        .as_object()
        .into_iter()
        .flat_map(|object| object.keys());
    keys.filter(|key| !known.contains(key))
        .map(|key| UnknownKey {
            at: at.to_string(),
            key: key.clone(),
            closest: closest(key, known.iter().map(String::as_str)),
        })
        .collect()
}

/// The names of the actions that don't exist in `manifest`, a whole parsed manifest file,
/// with the closest known name if one is close. Serde's own error lists every action
pub fn unknown_actions(manifest: &Value) -> Vec<String> {
    let actions = manifest.get("actions").and_then(Value::as_array);
    let names = || {
        Actions::NAMES
            .iter()
            .flat_map(|(name, aliases)| std::iter::once(*name).chain(aliases.iter().copied()))
    };

    let mut unknown = vec![];
    for (index, action) in actions.into_iter().flatten().enumerate() {
        let Some(name) = action.get("action").and_then(Value::as_str) else {
            continue;
        };
        if names().any(|known| known == name) {
            continue;
        }
        let mut message = format!("unknown action `{name}` in action {}", index + 1);
        if let Some(closest) = closest(name, names()) {
            message.push_str(&format!(", did you mean `{closest}`?"));
        }
        unknown.push(message);
    }
    unknown
}

/// The known word with the fewest typos from `key`, if it has at most two and isn't a
/// completely different word
fn closest<'a>(key: &str, known: impl IntoIterator<Item = &'a str>) -> Option<String> {
    known
        .into_iter()
        .map(|candidate| (strsim::damerau_levenshtein(key, candidate), candidate))
        .filter(|(distance, _)| *distance <= 2 && *distance < key.len())
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, candidate)| candidate.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    /// The unknown keys of a YAML manifest, as messages
    fn unknown(yaml: &str) -> Vec<String> {
        let manifest: Value = serde_yaml_ng::from_str(yaml).unwrap();
        unknown_keys(&manifest)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    }

    /// The unknown action messages of a YAML manifest
    fn unknown_action_names(yaml: &str) -> Vec<String> {
        unknown_actions(&serde_yaml_ng::from_str(yaml).unwrap())
    }

    #[test]
    fn a_misspelled_action_name_suggests_the_closest_action() {
        let yaml = "actions:\n  - action: directory.create\n    path: a\n  - action: file.cpy\n";

        assert_eq!(
            vec!["unknown action `file.cpy` in action 2, did you mean `file.copy`?"],
            unknown_action_names(yaml)
        );
    }

    #[test]
    fn an_action_name_that_is_nothing_like_an_action_has_no_suggestion() {
        let yaml = "actions:\n  - action: frobnicate\n";

        assert_eq!(
            vec!["unknown action `frobnicate` in action 1"],
            unknown_action_names(yaml)
        );
    }

    #[test]
    fn action_names_and_aliases_are_not_unknown() {
        let names: Vec<String> = Actions::NAMES
            .iter()
            .flat_map(|(name, aliases)| std::iter::once(name).chain(aliases.iter()))
            .map(|name| format!("  - action: {name}\n"))
            .collect();
        let yaml = format!("actions:\n{}", names.concat());

        assert_eq!(Vec::<String>::new(), unknown_action_names(&yaml));
    }

    #[test]
    fn a_typo_in_an_action_is_found_with_the_key_it_means() {
        let yaml = "actions:\n  - action: file.copy\n    form: a\n    to: b\n";

        assert_eq!(
            vec!["`form` in action 1 (file.copy) is ignored, did you mean `from`?"],
            unknown(yaml)
        );
    }

    #[test]
    fn known_keys_aliases_where_and_variants_are_fine() {
        let yaml = r#"
version: 1
where: os.name == "linux"
depends: [a]
labels: [b]
actions:
  - action: cmd.run
    command: echo
    sudo: true
    where: os.name == "linux"
    variants:
      - where: os.name == "macos"
        command: printf
  - action: file.download
    source: https://example.com/a
    target: /tmp/a
"#;

        assert_eq!(Vec::<String>::new(), unknown(yaml));
    }

    #[test]
    fn typos_in_the_manifest_and_in_variants_are_found() {
        let yaml = r#"
depend: [a]
actions:
  - action: package.install
    list: [git]
    variants:
      - where: os.name == "macos"
        lsit: [git]
        variants: []
"#;

        assert_eq!(
            vec![
                "`depend` in the manifest is ignored, did you mean `depends`?",
                "`lsit` in variant 1 of action 1 (package.install) is ignored, did you mean `list`?",
                "`variants` in variant 1 of action 1 (package.install) is ignored",
            ],
            unknown(yaml)
        );
    }

    #[test]
    fn a_completely_different_word_has_no_suggestion() {
        let yaml = "actions:\n  - action: directory.create\n    path: a\n    xyz: b\n";

        assert_eq!(
            vec!["`xyz` in action 1 (directory.create) is ignored"],
            unknown(yaml)
        );
    }

    /// A field alias missing from `x-aliases` would be warned about as unknown
    #[test]
    fn every_field_alias_is_a_known_key() {
        let actions = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/actions");
        let known: Vec<String> = Actions::NAMES
            .iter()
            .flat_map(|(name, _)| Actions::known_keys(name).unwrap())
            .collect();

        let mut checked = 0;
        for file in walk(&actions) {
            let source = std::fs::read_to_string(&file).unwrap();
            let lines: Vec<&str> = source.lines().map(str::trim).collect();
            for (index, line) in lines.iter().enumerate() {
                if !line.starts_with("#[serde(") || !is_field(&lines[index + 1..]) {
                    continue;
                }
                for alias in line.split("alias = \"").skip(1) {
                    let alias = alias.split('"').next().unwrap();
                    assert!(known.contains(&alias.to_string()), "{alias} in {file:?}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 0);
    }

    /// Whether the attributes before `lines` belong to a struct field like `pub from: String,`,
    /// not to an enum variant like `FreeBSD,`
    fn is_field(lines: &[&str]) -> bool {
        lines
            .iter()
            .find(|line| !line.starts_with("#[") && !line.starts_with("///"))
            .is_some_and(|line| line.contains(':'))
    }

    /// The `.rs` files below `dir`
    fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut files = vec![];
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files.extend(walk(&path));
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
        files
    }
}
