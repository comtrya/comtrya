use std::collections::HashMap;

use crate::actions::{ActionError, ActionResult};
use tracing::{error, trace, warn};

#[derive(Clone)]
pub struct Command {
    pub name: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub require_root: bool,
}

pub fn run_command(command: Command) -> Result<ActionResult, ActionError> {
    let mut command = command.clone();

    command.elevate();

    match std::process::Command::new(&command.name)
        .envs(&command.env)
        .args(&command.args)
        .output()
    {
        Ok(std::process::Output { status, stdout, .. }) if status.success() => {
            trace!("{}", String::from_utf8(stdout).unwrap().as_str());
            Ok(ActionResult {
                message: String::from("Success"),
            })
        }

        Ok(std::process::Output {
            status,
            stdout,
            stderr,
            ..
        }) => {
            warn!("{}", String::from_utf8(stdout).unwrap().as_str());
            error!("{}", String::from_utf8(stderr).unwrap().as_str());

            Err(ActionError {
                message: String::from(format!(
                    "Exit code: {}. Failed to run {} {}",
                    status.code().unwrap(),
                    command.name,
                    command.args.join(" ")
                )),
            })
        }

        Err(e) => Err(ActionError {
            message: e.to_string(),
        }),
    }
}

impl Command {
    fn elevate(&mut self) -> &mut Self {
        if !self.require_root {
            return self;
        }

        if "root" == whoami::username() {
            return self;
        }

        self.args.insert(0, self.name.clone());
        self.name = String::from("sudo");
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // Commenting this out until I know how to mock `whoami::username` for test
    // conditions
    // #[test]
    fn it_can_elevate() {
        let mut command = Command {
            name: String::from("apt"),
            args: vec![String::from("install")],
            require_root: true,
        };

        command.elevate();

        assert_eq!("sudo", command.name);
    }
}
