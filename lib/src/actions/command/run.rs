use crate::contexts::Contexts;
use crate::steps::Step;
use crate::{actions::Action, manifests::Manifest, utilities};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(JsonSchema, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunCommand {
    pub command: String,

    #[serde(default)]
    pub args: Vec<String>,

    #[serde(default = "get_false", alias = "sudo")]
    pub privileged: bool,

    /// Working directory of the command, defaults to the directory comtrya is run from
    #[serde(default = "get_cwd")]
    #[schemars(transform = remove_default)]
    pub dir: String,

    #[serde(default)]
    pub env: HashMap<String, String>,

    /// File to write the command's output to, instead of logging it. Relative to `dir`
    #[serde(default)]
    pub stdout: Option<String>,
}

fn get_false() -> bool {
    false
}

/// The default depends on where comtrya runs, so it must not end up in the generated JSON schema
fn remove_default(schema: &mut schemars::Schema) {
    schema.remove("default");
}

/// The directory comtrya runs in, or `.` if it's gone, which fails later with a clear error
fn get_cwd() -> String {
    std::env::current_dir()
        .map(|current_dir| current_dir.display().to_string())
        .unwrap_or_else(|_| String::from("."))
}

impl RunCommand {
    fn stdout_file(&self) -> Option<PathBuf> {
        self.stdout
            .as_ref()
            .map(|stdout| Path::new(&self.dir).join(stdout))
    }
}

impl Action for RunCommand {
    fn summarize(&self) -> String {
        format!("Running {} command", self.command)
    }

    fn plan(&self, _: &Manifest, contexts: &Contexts) -> anyhow::Result<Vec<Step>> {
        use crate::atoms::command::Exec;

        let privilege_provider = utilities::get_privilege_provider(contexts);

        Ok(vec![Step {
            atom: Box::new(Exec {
                command: self.command.clone(),
                arguments: self.args.clone(),
                privileged: self.privileged,
                working_dir: Some(self.dir.clone()),
                privilege_provider: privilege_provider.clone(),
                stdout_file: self.stdout_file(),
                // Only for the command, comtrya's own environment stays as it is
                environment: self.env.clone().into_iter().collect(),
                ..Default::default()
            }),
            initializers: vec![],
            finalizers: vec![],
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

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

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

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

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

    #[test]
    fn stdout_is_relative_to_dir() {
        let yaml = r#"
  - action: command.run
    command: echo
    dir: /tmp/work
    stdout: completions/echo
"#;

        let mut actions: Vec<Actions> = serde_yaml_ng::from_str(yaml).unwrap();

        let Some(Actions::CommandRun(action)) = actions.pop() else {
            panic!("command.run didn't deserialize to the correct type");
        };
        assert_eq!(
            Some(std::path::Path::new("/tmp/work").join("completions/echo")),
            action.action.stdout_file()
        );

        let absolute = super::RunCommand {
            dir: String::from("/tmp/work"),
            stdout: Some(String::from("/etc/out")),
            ..Default::default()
        };
        assert_eq!(
            Some(std::path::Path::new("/etc/out").to_path_buf()),
            absolute.stdout_file()
        );
    }
}
