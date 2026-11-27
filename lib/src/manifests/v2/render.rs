//! Renders the templates in the values of a parsed manifest, each on its own, so an error
//! names the value: `actions[2].to: …`. Keys aren't templates, and neither are `where`
//! conditions: they are Rhai, which reads the contexts itself

use crate::contexts::{to_tera, Contexts};
use crate::tera_functions::register_functions;
use crate::utilities::error_chain;
use serde_yaml_ng::Value;
use tera::{Context, Tera};

/// Renders every template in `document`. The error names the value that failed
pub(super) fn all(document: &mut Value, contexts: &Contexts) -> Result<(), String> {
    let mut renderer = Renderer {
        tera: Tera::default(),
        context: to_tera(contexts),
    };
    register_functions(&mut renderer.tera);

    renderer.value(document, "")
}

/// Tera with the contexts to render with
struct Renderer {
    /// Tera with comtrya's functions
    tera: Tera,
    /// The contexts, as Tera reads them
    context: Context,
}

impl Renderer {
    /// Renders the templates in `value`, which is at `at` in the manifest
    fn value(&mut self, value: &mut Value, at: &str) -> Result<(), String> {
        match value {
            Value::String(text) if is_template(text) => {
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
