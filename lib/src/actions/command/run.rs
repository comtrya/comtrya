use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunCommand {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,

    #[serde(default = "get_cwd")]
    pub dir: String,
}

fn get_false() -> bool {
    false
}

fn get_cwd() -> String {
    std::env::current_dir()
        .map(|current_dir| current_dir.display().to_string())
        .expect("Failed to get current directory")
}

impl Action for RunCommand {
    fn summarize(&self) -> String {
        format!("Running {} command", self.command)
    }

    fn plan(&self, _: &Manifest, _: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: self.command.clone(),
                arguments: self.args.clone(),
                privileged: self.privileged,
                working_dir: Some(self.dir.clone()),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
        }])
    }
}
