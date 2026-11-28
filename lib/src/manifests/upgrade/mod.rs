//! Rewrites a format 1 manifest as format 2, for `comtrya manifests upgrade`:
//!
//! - `version: 2` comes first.
//! - Old names of actions and fields, like `cmd.run` or `source`, become the one name format 2
//!   knows (see the `v2` module).
//! - Variants keep only the fields that differ from their action, since a format 2 variant takes
//!   the others from it.
//! - Source paths that start with `./` or `../` are written so they mean the same in format 2,
//!   where they are next to the manifest instead of below `files/`.
//! - Values that are one unquoted `{{ … }}` get quotes, since format 2 reads the file before it
//!   renders templates.
//!
//! The upgrade is checked: both versions are loaded with the same contexts and have to mean the
//! same, otherwise nothing is upgraded. A file whose templates aren't values, like `{% if %}`
//! around keys, can't be upgraded and is refused with the line that fails.

/// Loads both versions and compares what they mean
mod check;
/// Actions and fields by their one name
mod names;
/// Variants that keep only what differs from their action
mod overrides;
/// Quotes around values that are one template
mod quote;
/// Source paths that mean the same in format 2
mod sources;
#[cfg(test)]
mod tests;

use super::load::parse;
use super::v2;
use crate::contexts::Contexts;
use serde_yaml_ng::{Mapping, Value};
use std::path::Path;
use thiserror::Error as ThisError;

/// A manifest rewritten as format 2
#[derive(Debug)]
pub struct Upgraded {
    /// The new contents of the file
    pub text: String,
    /// What the upgrade couldn't keep, like comments
    pub warnings: Vec<String>,
}

/// Why a manifest can't be upgraded
#[derive(Debug, ThisError)]
pub enum UpgradeError {
    /// The file can't be read
    #[error("the file can't be read: {0}")]
    Read(#[source] std::io::Error),

    /// The file is already format 2, or newer
    #[error("it already uses manifest version {0}")]
    NotVersion1(u64),

    /// The file isn't YAML or TOML before its templates are rendered
    #[error("it isn't valid YAML or TOML before its templates are rendered, so templates are more than values ({0}); rewrite them as values by hand")]
    Unreadable(String),

    /// Something in the file has no format 2 version that means the same
    #[error("{0}")]
    Unclear(String),

    /// The upgraded file can't be written as YAML or TOML
    #[error("the upgraded manifest can't be written: {0}")]
    Write(String),

    /// The upgraded manifest doesn't mean the same as the original
    #[error("the upgraded manifest would mean something else: {0}")]
    Differs(String),
}

/// Reads the manifest at `path` and rewrites it as format 2, checked with `contexts`
///
/// # Errors
///
/// When the file can't be read, isn't format 1, or has no format 2 version that means the same
pub fn upgrade(path: &Path, contexts: &Contexts) -> Result<Upgraded, UpgradeError> {
    let text = std::fs::read_to_string(path).map_err(UpgradeError::Read)?;
    upgrade_text(&text, path, contexts)
}

/// Rewrites the format 1 manifest `text` as format 2; `path` says whether it's YAML or TOML
///
/// # Errors
///
/// When the text isn't format 1, or has no format 2 version that means the same
pub fn upgrade_text(
    text: &str,
    path: &Path,
    contexts: &Contexts,
) -> Result<Upgraded, UpgradeError> {
    let toml = is_toml(path);
    let quoted = quote::templates(text, toml).map_err(UpgradeError::Unclear)?;
    let document: Value = parse(&quoted, path)
        .map_err(|err| UpgradeError::Unreadable(crate::utilities::error_chain(&err)))?;
    let Value::Mapping(mut fields) = document else {
        return Err(UpgradeError::Unclear(String::from(
            "a manifest has to be a map of keys",
        )));
    };
    check_version(&fields)?;
    no_templates_in_where(&Value::Mapping(fields.clone()), "")?;

    let mut warnings = comments(text);
    rewrite_actions(&mut fields, &mut warnings)?;
    let upgraded = with_version_2(fields);

    let upgraded_text = write(&upgraded, toml)?;
    check::same_meaning(text, &upgraded, path, contexts)?;
    Ok(Upgraded {
        text: upgraded_text,
        warnings,
    })
}

/// Whether `path` is a TOML file, otherwise it's YAML
fn is_toml(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "toml")
}

/// Fails unless the manifest is format 1: without `version`, or with `version: 1`
fn check_version(fields: &Mapping) -> Result<(), UpgradeError> {
    match fields.get("version").and_then(Value::as_u64) {
        None | Some(1) => Ok(()),
        Some(version) => Err(UpgradeError::NotVersion1(version)),
    }
}

/// Fails on a `where` with a template in it. Format 1 rendered it with the whole file; format 2
/// reads `where` as a condition only, which reads the contexts itself
fn no_templates_in_where(value: &Value, at: &str) -> Result<(), UpgradeError> {
    let Value::Mapping(fields) = value else {
        if let Value::Sequence(values) = value {
            for (index, value) in values.iter().enumerate() {
                no_templates_in_where(value, &format!("{at}[{index}]"))?;
            }
        }
        return Ok(());
    };

    for (key, value) in fields {
        let key = key.as_str().unwrap_or("?");
        let here = if at.is_empty() {
            key.to_string()
        } else {
            format!("{at}.{key}")
        };
        if key == "where" && value.as_str().is_some_and(quote::has_template) {
            return Err(UpgradeError::Unclear(format!(
                "{here} has a template; in manifest version 2 `where` reads the contexts itself, like `user.name == \"ada\"`, so write it without `{{{{ }}}}`"
            )));
        }
        no_templates_in_where(value, &here)?;
    }
    Ok(())
}

/// A warning if `text` has comments: the upgraded file doesn't keep them
fn comments(text: &str) -> Vec<String> {
    let has_comments = text.lines().any(|line| {
        let line = line.trim_start();
        line.starts_with('#') || line.contains(" #")
    });
    if has_comments {
        vec![String::from(
            "comments aren't kept; copy the ones you need from the original",
        )]
    } else {
        vec![]
    }
}

/// Rewrites each entry of `actions`: names, source paths and variants
fn rewrite_actions(fields: &mut Mapping, warnings: &mut Vec<String>) -> Result<(), UpgradeError> {
    let Some(Value::Sequence(actions)) = fields.get_mut("actions") else {
        return Ok(());
    };

    for (index, action) in actions.iter_mut().enumerate() {
        let at = format!("actions[{index}]");
        let Value::Mapping(action) = action else {
            continue;
        };
        let name = names::rename(action, &at, warnings).map_err(UpgradeError::Unclear)?;
        sources::rewrite(&name, action, &at).map_err(UpgradeError::Unclear)?;
        overrides::rewrite(action, &at, warnings).map_err(UpgradeError::Unclear)?;
    }
    Ok(())
}

/// The manifest with `version: 2` as its first key
fn with_version_2(fields: Mapping) -> Value {
    let mut upgraded = Mapping::new();
    upgraded.insert(Value::from("version"), Value::from(v2::VERSION));
    for (key, value) in fields {
        if key.as_str() != Some("version") {
            upgraded.insert(key, value);
        }
    }
    Value::Mapping(upgraded)
}

/// The manifest as YAML, or as TOML
pub(super) fn write(document: &Value, toml: bool) -> Result<String, UpgradeError> {
    if toml {
        toml::to_string(document).map_err(|err| UpgradeError::Write(err.to_string()))
    } else {
        serde_yaml_ng::to_string(document).map_err(|err| UpgradeError::Write(err.to_string()))
    }
}
