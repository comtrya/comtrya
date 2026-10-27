//! `macos.default`: `defaults write`

use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// I went through all the examples here: https://macos-defaults.com/
// and while arrays and dictionaries are valid values, I couldn't
// find any usable examples. So omitting for now
/// Writes a macOS preference with `defaults write`, every time
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSDefault {
    /// The domain, like `com.apple.dock`
    pub domain: String,
    /// The key, like `autohide`
    pub key: String,
    /// The type of `value`, like `bool`, `int`, `float` or `string`
    pub kind: String,
    /// The value to write
    pub value: String,
}

impl Action for MacOSDefault {
    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        Ok(vec![Step::new(Exec {
            command: String::from("defaults"),
            arguments: vec![
                String::from("write"),
                self.domain.clone(),
                self.key.clone(),
                format!("-{}", self.kind),
                self.value.clone(),
            ],
            ..Default::default()
        })])
    }
}
