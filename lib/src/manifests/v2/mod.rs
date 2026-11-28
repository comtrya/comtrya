//! Manifest format 2 (a preview until 0.13.0): the file is parsed first, then its `where` is
//! evaluated, and only then are the templates in its values rendered, one value at a time.
//! Format 1 renders the whole file as text before parsing it
//!
//! Format 2 knows one name for each action and field: the aliases of format 1, like `cmd.run`
//! or `source` for `from`, are errors with the name to use. Every action can have a `title`,
//! which log lines show instead of the generated summary

/// Old names of actions and fields, which format 2 doesn't accept
mod aliases;
/// Templates in values
mod render;
/// Numbers and booleans from templates
mod typed;

use super::load::{parse, LoadErrorKind};
use super::{unknown_keys, Manifest};
use crate::actions::Actions;
use crate::contexts::{evaluate_condition, Contexts};
use crate::utilities::error_chain;
use serde::Deserialize;
use serde_yaml_ng::Value;
use std::path::Path;
use tracing::{debug, warn};

/// The manifest format this module reads
pub(super) const VERSION: u32 = 2;

/// Reads a format 2 manifest from the contents of its file
pub(super) fn load(
    contents: &str,
    path: &Path,
    contexts: &Contexts,
) -> Result<Manifest, LoadErrorKind> {
    let mut document: Value =
        parse(contents, path).map_err(|err| LoadErrorKind::Parse(error_chain(&err)))?;

    if let Some(condition) = false_where(&document, contexts) {
        debug!("Not rendering templates, because the 'where' condition is false");
        return Ok(skipped(&document, condition));
    }

    let rendered = render::all(&mut document, contexts).map_err(LoadErrorKind::Render)?;
    deferred_unsupported(&rendered).map_err(LoadErrorKind::Render)?;
    typed::apply(&mut document, &rendered);
    let titles = take_titles(&mut document).map_err(LoadErrorKind::Parse)?;
    aliases::check(&document).map_err(LoadErrorKind::Parse)?;

    let unknown = serde_json::to_value(&document)
        .map(|generic| unknown_keys(&generic))
        .unwrap_or_default();
    let mut manifest = parse_manifest(document).map_err(|err| {
        // A misspelled required field is the likely cause
        let hints: String = unknown.iter().map(|key| format!("; {key}")).collect();
        LoadErrorKind::Parse(format!("{err}{hints}"))
    })?;
    for key in unknown {
        warn!("{}: {key}", path.display());
    }

    for (action, title) in manifest.actions.iter_mut().zip(titles) {
        if let Some(title) = title {
            action.set_title(title);
        }
    }
    Ok(manifest)
}

/// Fails on the first value that uses `registered.*`: rendering those when their action is
/// planned comes with `register`
fn deferred_unsupported(rendered: &render::Rendered) -> Result<(), String> {
    match rendered.deferred.first() {
        Some(at) => Err(format!(
            "{at}: `registered` values need `register`, which manifest version 2 doesn't have yet"
        )),
        None => Ok(()),
    }
}

/// The manifest's `where`, if it is there and false. One that can't be evaluated counts as
/// true here, so the executor reports it
fn false_where(document: &Value, contexts: &Contexts) -> Option<String> {
    let condition = document.get("where")?.as_str()?;

    evaluate_condition(condition, contexts)
        .is_ok_and(|result| !result)
        .then(|| condition.to_string())
}

/// The keys of a manifest that don't hold actions, read without rendering templates
#[derive(Default, Deserialize)]
#[serde(default)]
struct Header {
    /// See [`Manifest::labels`]
    labels: Vec<String>,
    /// See [`Manifest::depends`]
    depends: Vec<String>,
}

/// A manifest whose `where` is false: its labels and dependencies, but no actions, since their
/// templates may only work where the manifest applies
fn skipped(document: &Value, condition: String) -> Manifest {
    let header = Header::deserialize(document).unwrap_or_default();

    Manifest {
        version: Some(VERSION),
        r#where: Some(condition),
        labels: header.labels,
        depends: header.depends,
        ..Default::default()
    }
}

/// Removes `title` from each action, in the order of the actions
fn take_titles(document: &mut Value) -> Result<Vec<Option<String>>, String> {
    let Some(actions) = document.get_mut("actions").and_then(Value::as_sequence_mut) else {
        return Ok(vec![]);
    };

    actions
        .iter_mut()
        .enumerate()
        .map(|(index, action)| {
            let title = action
                .as_mapping_mut()
                .and_then(|fields| fields.remove("title"));
            match title {
                None => Ok(None),
                Some(Value::String(title)) => Ok(Some(title)),
                Some(_) => Err(format!("actions[{index}]: `title` has to be text")),
            }
        })
        .collect()
}

/// The manifest from its parsed, rendered file. Actions are read one at a time, so an error
/// names the action: `actions[1]: missing field `from``
fn parse_manifest(mut document: Value) -> Result<Manifest, String> {
    let actions = match document.as_mapping_mut() {
        Some(fields) => fields.remove("actions"),
        None => return Err(String::from("a manifest has to be a map of keys")),
    };

    let mut manifest = Manifest::deserialize(document).map_err(|err| error_chain(&err))?;
    manifest.actions = match actions {
        None | Some(Value::Null) => vec![],
        Some(Value::Sequence(actions)) => actions
            .into_iter()
            .enumerate()
            .map(|(index, action)| {
                Actions::deserialize(action).map_err(|err| format!("actions[{index}]: {err}"))
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(String::from("`actions` has to be a list")),
    };
    Ok(manifest)
}

#[cfg(test)]
mod tests;
