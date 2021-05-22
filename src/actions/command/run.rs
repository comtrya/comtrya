use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest};
use serde::{Deserialize, Serialize};
use tera::Context;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunCommand {
    pub command: String,

    pub only: Option<String>,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false")]
    pub sudo: bool,

    pub dir: Option<String>,

    #[serde(default)]
    pub variants: Vec<Variant<RunCommand>>,
}

type Where = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variant<T> {
    #[serde(rename = "where")]
    pub where_clause: Where,
    #[serde(flatten)]
    pub command: T,
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
