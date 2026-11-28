//! Renders the templates in the values of a parsed manifest, each on its own, so an error
//! names the value: `actions[2].to: …`. Keys aren't templates, and neither are `where`
//! conditions: they are Rhai, which reads the contexts itself
//!
//! Values that use `registered.*` aren't rendered here: what an action registers only exists
//! once it ran, so they are rendered when their action is planned (see [`Rendered::deferred`])

use crate::contexts::{to_tera, Contexts};
use crate::tera_functions::register_functions;
use crate::utilities::error_chain;
use serde_yaml_ng::Value;
use std::collections::HashSet;
use tera::{Context, Tera};

/// What rendering found out about the values, by their path like `actions[2].to`
#[derive(Debug, Default)]
pub(super) struct Rendered {
    /// Values that were one template expression and nothing else, like `"{{ x }}"`. Their
    /// result may be a number or a boolean, see the `typed` module
    pub(super) whole: HashSet<String>,
    /// Values that use `registered.*`, left as they are to be rendered at plan time
    pub(super) deferred: Vec<String>,
}

/// Renders every template in `document`. The error names the value that failed
pub(super) fn all(document: &mut Value, contexts: &Contexts) -> Result<Rendered, String> {
    let mut renderer = Renderer {
        tera: Tera::default(),
        context: to_tera(contexts),
        rendered: Rendered::default(),
    };
    register_functions(&mut renderer.tera);

    renderer.value(document, "")?;
    Ok(renderer.rendered)
}

/// Tera with the contexts to render with
struct Renderer {
    /// Tera with comtrya's functions
    tera: Tera,
    /// The contexts, as Tera reads them
    context: Context,
    /// What was found so far
    rendered: Rendered,
}

impl Renderer {
    /// Renders the templates in `value`, which is at `at` in the manifest
    fn value(&mut self, value: &mut Value, at: &str) -> Result<(), String> {
        match value {
            Value::String(text) if uses_registered(text) => {
                self.rendered.deferred.push(at.to_string());
            }
            Value::String(text) if is_template(text) => {
                if is_one_expression(text) {
                    self.rendered.whole.insert(at.to_string());
                }
                *text = self
                    .tera
                    .render_str(text, &self.context)
                    .map_err(|err| format!("{}: {}", shown(at), error_chain(&err)))?;
            }
            Value::Sequence(values) => {
                for (index, value) in values.iter_mut().enumerate() {
                    self.value(value, &format!("{at}[{index}]"))?;
                }
            }
            Value::Mapping(fields) => {
                for (key, value) in fields.iter_mut() {
                    let key = key.as_str().unwrap_or("?");
                    if key != "where" {
                        self.value(value, &joined(at, key))?;
                    }
                }
            }
            Value::Tagged(tagged) => self.value(&mut tagged.value, at)?,
            _ => {}
        }
        Ok(())
    }
}

/// Whether `text` has Tera syntax. Other text is left as it is
fn is_template(text: &str) -> bool {
    ["{{", "{%", "{#"].iter().any(|start| text.contains(start))
}

/// Whether `text` is one `{{ … }}` and nothing else, apart from spaces around it
fn is_one_expression(text: &str) -> bool {
    let Some(inner) = text
        .trim()
        .strip_prefix("{{")
        .and_then(|rest| rest.strip_suffix("}}"))
    else {
        return false;
    };
    !["{{", "}}", "{%", "{#"]
        .iter()
        .any(|marker| inner.contains(marker))
}

/// Whether the template in `text` reads what an action registered
fn uses_registered(text: &str) -> bool {
    is_template(text) && text.contains("registered.")
}

/// The path of `key` below `at`, like `actions[2].to`
fn joined(at: &str, key: &str) -> String {
    if at.is_empty() {
        key.to_string()
    } else {
        format!("{at}.{key}")
    }
}

/// `at` for an error, the whole manifest if it's empty
fn shown(at: &str) -> &str {
    if at.is_empty() {
        "the manifest"
    } else {
        at
    }
}
