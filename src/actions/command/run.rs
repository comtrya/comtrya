use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest};
use serde::{Deserialize, Serialize};
use tera::Context;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct RunCommand {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false")]
    pub sudo: bool,

    pub dir: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct Extended<T> {
    #[serde(rename = "only")]
    pub condition: Option<Condition>,

    #[serde(flatten)]
    pub action: T,

    #[serde(default)]
    pub variants: Vec<Variant<T>>,
}

impl<T> Action for Extended<T>
where
    T: Action,
{
    fn plan(&self, manifest: &Manifest, context: &Context) -> Vec<Step> {
        self.action.plan(manifest, context)
    }
}

type Condition = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variant<T> {
    #[serde(rename = "where")]
    pub condition: Condition,
    #[serde(flatten)]
    pub action: T,
}

fn get_false() -> bool {
    false
}

impl Action for RunCommand {
    fn plan(&self, _: &Manifest, _: &Context) -> Vec<Step> {
        use crate::atoms::command::Exec;

        vec![Step {
            atom: Box::new(Exec {
                command: self.command.clone(),
                arguments: self.args.clone(),
                privileged: self.sudo,
                working_dir: self.dir.clone(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}
