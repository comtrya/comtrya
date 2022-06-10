use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RunCommand {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false")]
    pub sudo: bool,

    #[serde(default = "get_cwd")]
    pub dir: String,
}

fn get_false() -> bool {
    false
}

fn get_cwd() -> String {
    std::env::current_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap()
}

impl Action for RunCommand {
    fn plan(&self, _: &Manifest, _: &Contexts) -> Vec<Step> {
        use crate::atoms::command::Exec;

        vec![Step {
            atom: Box::new(Exec {
                command: self.command.clone(),
                arguments: self.args.clone(),
                privileged: self.sudo,
                working_dir: Some(self.dir.clone()),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }]
    }
}
