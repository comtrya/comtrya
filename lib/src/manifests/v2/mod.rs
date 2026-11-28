//! Manifest format 2 (a preview until 0.13.0): the file is parsed first, then its `where` is
//! evaluated, and only then are the templates in its values rendered, one value at a time.
//! Format 1 renders the whole file as text before parsing it
//!
//! Format 2 knows one name for each action and field: the aliases of format 1, like `cmd.run`
//! or `source` for `from`, are errors with the name to use. Every action can have a `title`,
//! which log lines show instead of the generated summary. A variant sets only the fields that
//! differ from its action

/// Old names of actions and fields, which format 2 doesn't accept
pub(super) mod aliases;
/// Templates in values
mod render;
/// Numbers and booleans from templates
mod typed;
/// Variants that override fields of their action
pub(super) mod variants;

use super::load::{parse, LoadErrorKind};
use super::{unknown_keys, Manifest};
use crate::actions::{Actions, Deferred};
use crate::contexts::{evaluate_condition, Contexts};
use crate::registered::{self, Registers};
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
    only_in_actions(&rendered.deferred).map_err(LoadErrorKind::Render)?;
    typed::apply(&mut document, &rendered);
    let titles = take_titles(&mut document).map_err(LoadErrorKind::Parse)?;
    let registers = take_registers(&mut document).map_err(LoadErrorKind::Parse)?;
    aliases::check(&document).map_err(LoadErrorKind::Parse)?;

    let unknown = serde_json::to_value(&document)
        .map(|generic| unknown_keys(&generic))
        .unwrap_or_default();
    variants::complete(&mut document).map_err(LoadErrorKind::Parse)?;
    let deferred = render::deferred(&document);
    let mut manifest = parse_manifest(document, &deferred).map_err(|err| {
        // A misspelled required field is the likely cause
        let hints: String = unknown.iter().map(|key| format!("; {key}")).collect();
        LoadErrorKind::Parse(format!("{err}{hints}"))
    })?;
    for key in unknown {
        warn!("{}: {key}", path.display());
    }

    let extras = titles.into_iter().zip(registers);
    for (action, (title, register)) in manifest.actions.iter_mut().zip(extras) {
        if let Some(title) = title {
            action.set_title(title);
        }
        if let Some(register) = register {
            action.set_register(register);
        }
    }
    Ok(manifest)
}

/// Reads an action entry at `at` whose values at the paths in `deferred` read `registered.*`,
/// when it's planned: renders them with the contexts and what was registered, then reads the
/// action. The error names the value or the action
pub(crate) fn read_deferred(
    mut entry: Value,
    at: &str,
    deferred: &[String],
    contexts: &Contexts,
    registers: &Registers,
) -> Result<Actions, String> {
    let rendered = render::registered(&mut entry, at, deferred, contexts, registers)?;
    typed::action(&mut entry, at, &rendered);
    Actions::deserialize(entry).map_err(|err| format!("{at}: {err}"))
}

/// Fails on a value that reads `registered.*` outside of the actions, like in `depends`: only
/// actions are planned after earlier ones ran
fn only_in_actions(deferred: &[String]) -> Result<(), String> {
    match deferred.iter().find(|at| !at.starts_with("actions[")) {
        Some(at) => Err(format!(
            "{at}: only the fields of actions can read `registered` values"
        )),
        None => Ok(()),
    }
}

/// Removes `register` from each action, in the order of the actions. Only `command.run` has
/// it, and its name has to be one templates can read, see [`registered::is_name`]
fn take_registers(document: &mut Value) -> Result<Vec<Option<String>>, String> {
    let Some(actions) = document.get_mut("actions").and_then(Value::as_sequence_mut) else {
        return Ok(vec![]);
    };

    actions
        .iter_mut()
        .enumerate()
        .map(|(index, action)| {
            let name = action
                .get("action")
                .and_then(Value::as_str)
                .map(str::to_string);
            let register = action
                .as_mapping_mut()
                .and_then(|fields| fields.remove("register"));
            match register {
                None => Ok(None),
                Some(_) if name.as_deref() != Some("command.run") => Err(format!(
                    "actions[{index}]: only `command.run` can `register` what it printed"
                )),
                Some(Value::String(register)) if registered::is_name(&register) => {
                    Ok(Some(register))
                }
                Some(_) => Err(format!(
                    "actions[{index}]: `register` has to be a name of letters, digits and `_`, \
                     like `rustc_version`"
                )),
            }
        })
        .collect()
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
fn parse_manifest(mut document: Value, deferred: &[String]) -> Result<Manifest, String> {
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
            .map(|(index, action)| read_action(action, index, deferred))
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(String::from("`actions` has to be a list")),
    };
    Ok(manifest)
}

/// Reads the action entry at `index`, or keeps it to read when it's planned if values in it
/// read `registered.*`, see [`Deferred`]
fn read_action(action: Value, index: usize, deferred: &[String]) -> Result<Actions, String> {
    let at = format!("actions[{index}]");
    let paths: Vec<String> = deferred
        .iter()
        .filter(|path| is_below(path, &at))
        .cloned()
        .collect();
    let name = action
        .get("action")
        .and_then(Value::as_str)
        .and_then(|name| Actions::NAMES.iter().find(|(known, _)| *known == name));

    match (paths.is_empty(), name) {
        (false, Some((name, _))) => Ok(Actions::Deferred(Box::new(Deferred::new(
            name, at, action, paths,
        )))),
        _ => Actions::deserialize(action).map_err(|err| format!("{at}: {err}")),
    }
}

/// Whether the value at `path` is `at` or inside it
fn is_below(path: &str, at: &str) -> bool {
    path.strip_prefix(at)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(['.', '[']))
}

#[cfg(test)]
mod tests;
