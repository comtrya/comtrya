use crate::actions::{Action, ActionError, ActionResult};
use std::process::{Command, Stdio};

pub trait CommandAction: Action {
    fn init(&self, command: &str) -> Command {
        Command::new(command)
    }

    fn inherit<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
    }

    fn args<'a>(&self, command: &'a mut Command, args: Vec<String>) -> &'a mut Command {
        command.args(args)
    }

    fn execute(&self, command: &mut Command) -> Result<ActionResult, ActionError> {
        match command.output() {
            Ok(result) => Ok(ActionResult {
                message: String::from_utf8(result.stdout).unwrap(),
            }),
            Err(error) => Err(ActionError {
                message: error.to_string(),
            }),
        }
    }
}
