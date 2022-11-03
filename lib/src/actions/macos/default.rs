use crate::atoms::command::Exec;
use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// I went through all the examples here: https://macos-defaults.com/
// and while arrays and dictionaries are valid values, I couldn't
// find any usable examples. So omitting for now
#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacOSDefault {
    pub domain: String,
    pub key: String,
    pub kind: String,
    pub value: String,
}

impl Action for MacOSDefault {
    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        Ok(vec![Step {
            atom: Box::new(Exec {
                command: String::from("defaults"),
                arguments: vec![
                    String::from("write"),
                    self.domain.clone(),
                    self.key.clone(),
                    format!("-{}", self.kind),
                    self.value.clone(),
                ],
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
