//! The upgrade is only kept if both versions mean the same: loaded with the same contexts, they
//! have the same actions with the same fields and variants, the same dependencies and labels,
//! and their source paths point to the same files

use super::sources::{base, normalized, starts_with_dot, Base};
use super::{is_toml, write, UpgradeError};
use crate::contexts::{manifest::with_manifest, Contexts};
use crate::manifests::load::{load_text, LoadErrorKind};
use crate::manifests::Manifest;
use serde_json::{Map, Value as Json};
use serde_yaml_ng::Value;
use std::path::Path;

/// Fails unless the format 1 `original` and the `upgraded` format 2 manifest mean the same
pub(super) fn same_meaning(
    original: &str,
    upgraded: &Value,
    path: &Path,
    contexts: &Contexts,
) -> Result<(), UpgradeError> {
    let contexts = with_manifest_of(path, contexts);
    let version_1 = load_text(original, path, &contexts).map_err(|err| {
        UpgradeError::Unclear(format!(
            "it doesn't load here, so the upgrade can't be checked: {err}"
        ))
    })?;
    // Version 2 leaves out the actions of a manifest whose `where` is false; version 1 decides
    // that later. Both are compared without it, `where` itself stays as it is
    let mut upgraded = upgraded.clone();
    if let Value::Mapping(fields) = &mut upgraded {
        fields.remove("where");
    }
    let text = write(&upgraded, is_toml(path))?;
    let version_2 = load_text(&text, path, &contexts).map_err(|err| {
        UpgradeError::Differs(format!("version 2 doesn't load: {}", load_error(&err)))
    })?;

    let (before, after) = (meaning(&version_1, false), meaning(&version_2, true));
    match difference(&before, &after, "") {
        Some(difference) => Err(UpgradeError::Differs(difference)),
        None => Ok(()),
    }
}

/// `contexts` with `manifest.name` and `manifest.dir` of the file at `path`
fn with_manifest_of(path: &Path, contexts: &Contexts) -> Contexts {
    let name = path.file_stem().unwrap_or_default().to_string_lossy();
    let dir = path.parent().unwrap_or(Path::new("."));
    with_manifest(contexts, &name, dir)
}

/// The text of a load error with its causes
fn load_error(err: &LoadErrorKind) -> String {
    crate::utilities::error_chain(err)
}

/// What `manifest` means, as JSON to compare: its fields without `version` and `where`, and
/// each action with its sources as the files they point to
fn meaning(manifest: &Manifest, version_2: bool) -> Json {
    let mut json = serde_json::to_value(manifest).unwrap_or_default();
    if let Json::Object(fields) = &mut json {
        fields.remove("version");
        fields.remove("where");
        if let Some(Json::Array(actions)) = fields.get_mut("actions") {
            for action in actions {
                action_meaning(action, version_2);
            }
        }
    }
    json
}

/// What the action entry `action` means, with its variants
fn action_meaning(action: &mut Json, version_2: bool) {
    let Json::Object(fields) = action else {
        return;
    };
    let name = fields
        .get("action")
        .and_then(Json::as_str)
        .unwrap_or_default()
        .to_string();
    fields_meaning(&name, fields, version_2);
    if let Some(Json::Array(variants)) = fields.get_mut("variants") {
        for variant in variants {
            if let Json::Object(variant) = variant {
                fields_meaning(&name, variant, version_2);
            }
        }
    }
}

/// What the fields of the action `name` mean: `file.link`'s `source` and `target` as `from` and
/// `to`, which they replace, and each source path as the file it points to
fn fields_meaning(name: &str, fields: &mut Map<String, Json>, version_2: bool) {
    if name == "file.link" {
        for (old, field) in [("source", "from"), ("target", "to")] {
            match fields.remove(old) {
                Some(value) if !value.is_null() => {
                    fields.insert(field.to_string(), value);
                }
                _ => {}
            }
        }
    }
    for (field, value) in fields.iter_mut() {
        if let (Some(base), Json::String(path)) = (base(name, field), &*value) {
            *value = Json::String(file_of(base, version_2, path));
        }
    }
}

/// The file the source `path` points to, relative to the manifest's directory (`manifest:`) or
/// to where comtrya runs (`cwd:`)
fn file_of(base: Base, version_2: bool, path: &str) -> String {
    if Path::new(path).is_absolute() {
        return path.to_string();
    }
    let next_to_manifest = version_2 && starts_with_dot(path);
    match base {
        Base::Files if !next_to_manifest => {
            format!("manifest:{}", normalized(&format!("files/{path}")))
        }
        Base::AsWritten if !next_to_manifest => format!("cwd:{}", normalized(path)),
        _ => format!("manifest:{}", normalized(path)),
    }
}

/// The first place where `before` and `after` differ, at `at`, as a sentence
fn difference(before: &Json, after: &Json, at: &str) -> Option<String> {
    match (before, after) {
        (Json::Object(before), Json::Object(after)) => {
            let keys = before
                .keys()
                .chain(after.keys().filter(|key| !before.contains_key(*key)));
            keys.into_iter().find_map(|key| {
                let at = if at.is_empty() {
                    key.clone()
                } else {
                    format!("{at}.{key}")
                };
                let missing = Json::Null;
                difference(
                    before.get(key).unwrap_or(&missing),
                    after.get(key).unwrap_or(&missing),
                    &at,
                )
            })
        }
        (Json::Array(before), Json::Array(after)) if before.len() == after.len() => before
            .iter()
            .zip(after)
            .enumerate()
            .find_map(|(index, (before, after))| {
                difference(before, after, &format!("{at}[{index}]"))
            }),
        _ if before == after => None,
        _ => Some(described(before, after, at)),
    }
}

/// A difference at `at` as a sentence, with what to do if it's in a variant
fn described(before: &Json, after: &Json, at: &str) -> String {
    let mut sentence = format!("{at} is {before} in version 1, but {after} in version 2");
    if at.contains("variants[") {
        sentence.push_str(
            "; a version 2 variant takes the fields it doesn't set from its action, so set the field in the variant of the original",
        );
    }
    sentence
}
