use crate::{actions::Action, manifests::Manifest};
use serde::{Deserialize, Serialize};
use tera::Context;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunCommand {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false")]
    pub sudo: bool,

    pub dir: Option<String>,
}

fn get_false() -> bool {
    false
}

impl Action for RunCommand {
    fn plan(&self, _: &Manifest, _: &Context) -> Vec<Box<dyn crate::atoms::Atom>> {
        use crate::atoms::command::Exec;

        vec![Box::new(Exec {
            command: self.command.clone(),
            arguments: self.args.clone(),
            privileged: self.sudo,
            working_dir: self.dir.clone(),
            ..Default::default()
        })]
    }
}
