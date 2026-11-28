//! Actions of manifest version 2 whose fields read `registered.*`. What an action registers
//! only exists once it ran, so such an action is read when it's planned: its fields are
//! rendered then, with what was registered, and the action is read from them. The manifest
//! checks its keys when it's loaded, but its fields' types and required fields only now

use super::{Action, Actions, Error, PlanCx, Result};
use crate::manifests::{read_deferred, Manifest};
use crate::registered;
use crate::steps::Step;
use std::sync::OnceLock;

/// An action entry of a version 2 manifest that is read when it's planned, see the module docs
#[derive(Clone, Debug)]
pub struct Deferred {
    /// The action's name, like `command.run`
    name: &'static str,
    /// Where the entry is in its manifest, like `actions[2]`
    at: String,
    /// The entry, with every value rendered but those in `deferred`
    entry: serde_yaml_ng::Value,
    /// The paths of the values that read `registered.*`, like `actions[2].args[0]`
    deferred: Vec<String>,
    /// Whether the manifest goes on when it fails, from the entry's `continue_on_error`
    continue_on_error: bool,
    /// See [`super::ConditionalVariantAction::title`]
    pub(crate) title: Option<String>,
    /// See [`super::ConditionalVariantAction::register`]
    pub(crate) register: Option<String>,
    /// The summary of the action once it was read, for the log line after it ran
    summary: OnceLock<String>,
}

impl Deferred {
    /// The action called `name` from `entry`, at `at` in its manifest, whose values at the
    /// paths in `deferred` are rendered when it's planned
    pub(crate) fn new(
        name: &'static str,
        at: String,
        entry: serde_yaml_ng::Value,
        deferred: Vec<String>,
    ) -> Self {
        let continue_on_error = entry
            .get("continue_on_error")
            .and_then(serde_yaml_ng::Value::as_bool)
            .unwrap_or_default();

        Self {
            name,
            at,
            entry,
            deferred,
            continue_on_error,
            title: None,
            register: None,
            summary: OnceLock::new(),
        }
    }

    /// The action's name, like `command.run`
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The action, read from its fields rendered with what was registered so far
    fn read(&self, cx: &PlanCx) -> Result<Actions> {
        if let Some(name) = self.unregistered(cx) {
            return Err(Error::NotRegistered {
                name: name.to_string(),
            });
        }

        let mut action = read_deferred(
            self.entry.clone(),
            &self.at,
            &self.deferred,
            cx.contexts,
            cx.registered,
        )
        .map_err(Error::Registered)?;
        if let Some(title) = &self.title {
            action.set_title(title.clone());
        }
        Ok(action)
    }

    /// The first `registered.<name>` that a deferred value reads and no action registered
    fn unregistered<'a>(&'a self, cx: &PlanCx) -> Option<&'a str> {
        let mut texts = self.deferred_texts();
        texts.find_map(|text| registered::missing(text, cx.registered))
    }

    /// The text of each deferred value: the values that are still templates, since the others
    /// were rendered when the manifest was loaded
    fn deferred_texts(&self) -> impl Iterator<Item = &str> {
        let mut texts = vec![];
        strings(&self.entry, &mut texts);
        texts
            .into_iter()
            .filter(|text| text.contains("{{") || text.contains("{%"))
    }
}

/// Collects the text values in `value`
fn strings<'a>(value: &'a serde_yaml_ng::Value, texts: &mut Vec<&'a str>) {
    match value {
        serde_yaml_ng::Value::String(text) => texts.push(text),
        serde_yaml_ng::Value::Sequence(values) => {
            values.iter().for_each(|value| strings(value, texts));
        }
        serde_yaml_ng::Value::Mapping(fields) => {
            fields.values().for_each(|value| strings(value, texts));
        }
        serde_yaml_ng::Value::Tagged(tagged) => strings(&tagged.value, texts),
        _ => {}
    }
}

impl Action for Deferred {
    fn summarize(&self) -> String {
        self.title
            .clone()
            .or_else(|| self.summary.get().cloned())
            .unwrap_or_else(|| self.name.to_string())
    }

    fn plan(&self, manifest: &Manifest, cx: &PlanCx) -> Result<Vec<Step>> {
        let action = self.read(cx)?;
        let _ = self.summary.set(action.summarize());
        action.plan(manifest, cx)
    }

    fn continue_on_error(&self) -> bool {
        self.continue_on_error
    }

    fn register(&self) -> Option<&str> {
        self.register.as_deref()
    }

    fn reads_registered(&self) -> bool {
        true
    }
}
