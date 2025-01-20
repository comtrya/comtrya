use crate::contexts::Contexts;
use crate::steps::finalizers::RemoveEnvVars;
use crate::steps::initializers::SetEnvVars;
use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest, steps, utilities};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunCommand {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,

    #[serde(default = "get_cwd")]
    pub dir: String,

    #[serde(default)]
    pub env: HashMap<String, String>,
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

    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let privilege_provider =
            utilities::get_privilege_provider(contexts).unwrap_or_else(|| "sudo".to_string());

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: self.command.clone(),
                arguments: self.args.clone(),
                privileged: self.privileged,
                working_dir: Some(self.dir.clone()),
                privilege_provider: privilege_provider.clone(),
                ..Default::default()
            }),
            initializers: vec![steps::initializers::FlowControl::Ensure(Box::new(
                SetEnvVars(self.env.clone()),
            ))],
            finalizers: vec![steps::finalizers::FlowControl::Ensure(Box::new(
                RemoveEnvVars(self.env.clone()),
            ))],
        }])
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::Actions;

    #[test]
    fn it_can_be_deserialize() {
        let yaml = r#"
  - action: command.run
    command: echo
    args:
      - hi
"#;

        let mut actions: Vec<Actions> = serde_yml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::CommandRun(action)) => {
                assert_eq!("echo", action.action.command);
                assert_eq!("hi", action.action.args.first().unwrap().as_str());
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }

    #[test]
    fn it_can_deserialize_env_vars() {
        let yaml = r#"
  - action: command.run
    command: echo
    args:
      - hi
    env:
        GOROOT: test
"#;

        let mut actions: Vec<Actions> = serde_yml::from_str(yaml).unwrap();

        match actions.pop() {
            Some(Actions::CommandRun(action)) => {
                assert_eq!("echo", action.action.command);
                assert_eq!("hi", action.action.args.first().unwrap().as_str());

                let value = action.action.env.get("GOROOT");
                assert!(value.is_some());
                assert_eq!("test", value.unwrap());
            }
            _ => {
                panic!("FileCopy didn't deserialize to the correct type");
            }
        };
    }
}
