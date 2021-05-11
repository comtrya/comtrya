use super::super::Atom;
use super::finalizers;
use super::initializers;
use super::CommandAtom;

#[derive(Default)]
pub struct CommandRun {
    command: String,
    arguments: Vec<String>,
    working_dir: Option<String>,
    environment: Vec<(String, String)>,
    privileged: bool,
    initializers: Vec<initializers::FlowControl>,
    finalizers: Vec<finalizers::FlowControl>,
}

pub fn new_run_command(command: String) -> CommandRun {
    CommandRun {
        command,
        ..Default::default()
    }
}

impl CommandRun {
    fn elevate(&mut self) {
        if !self.privileged {
            return;
        }

        if "root" == whoami::username() {
            return;
        }

        self.arguments.insert(0, self.command.clone());
        self.command = String::from("sudo");
    }
}

impl CommandAtom for CommandRun {}

impl std::fmt::Display for CommandRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RunCommand with privileged {}: {} {}",
            self.privileged,
            self.command,
            self.arguments.join(" ")
        )
    }
}

impl Atom for CommandRun {
    fn plan(&self) -> bool {
        let mut initializers = self.initializers.iter();

        while let Some(initializer) = initializers.next() {
            match initializer {
                initializers::FlowControl::SkipIf(skip) => {
                    if skip.run() {
                        return false;
                    }
                }
            }
        }

        true
    }

    fn execute(&self) -> anyhow::Result<()> {
        // Do we need to sudo?
        //?

        // If we require root, we need to use sudo with inheritted IO
        // to ensure the user can respond if prompted for a password
        // if command.require_root {
        //     match std::process::Command::new("sudo")
        //         .stdin(std::process::Stdio::inherit())
        //         .stdout(std::process::Stdio::inherit())
        //         .stderr(std::process::Stdio::inherit())
        //         .arg("--validate")
        //         .output()
        //     {
        //         Ok(std::process::Output { status, .. }) if status.success() => (),

        //         _ => return Err(anyhow!("Failed to get sudo access")),
        //     };
        // }

        // match std::process::Command::new(&command.name)
        //     .envs(&command.env)
        //     .args(&command.args)
        //     .current_dir(
        //         &command.dir.clone().unwrap_or(
        //             std::env::current_dir()
        //                 .unwrap()
        //                 .into_os_string()
        //                 .into_string()
        //                 .unwrap(),
        //         ),
        //     )
        //     .output()
        // {
        //     Ok(std::process::Output { status, stdout, .. }) if status.success() => {
        //         trace!("{}", String::from_utf8(stdout).unwrap().as_str());
        //         Ok(ActionResult {
        //             message: String::from("Success"),
        //         })
        //     }

        //     Ok(std::process::Output {
        //         status,
        //         stdout,
        //         stderr,
        //         ..
        //     }) => {
        //         warn!("{}", String::from_utf8(stdout).unwrap().as_str());
        //         error!("{}", String::from_utf8(stderr).unwrap().as_str());

        //         Err(anyhow!(format!(
        //             "Exit code: {}. Failed to run {} {}",
        //             status.code().unwrap(),
        //             command.name,
        //             command.args.join(" ")
        //         )))
        //     }
        //     Err(e) => Err(anyhow!(e)),
        // }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let mut command_run = CommandRun {
            ..Default::default()
        };

        assert_eq!(String::from(""), command_run.command);
        assert_eq!(0, command_run.arguments.len());
        assert_eq!(None, command_run.working_dir);
        assert_eq!(0, command_run.environment.len());
        assert_eq!(0, command_run.initializers.len());
        assert_eq!(0, command_run.finalizers.len());
        assert_eq!(false, command_run.privileged);

        let mut command_run = new_run_command(String::from("echo"));

        assert_eq!(String::from("echo"), command_run.command);
        assert_eq!(0, command_run.arguments.len());
        assert_eq!(None, command_run.working_dir);
        assert_eq!(0, command_run.environment.len());
        assert_eq!(0, command_run.initializers.len());
        assert_eq!(0, command_run.finalizers.len());
        assert_eq!(false, command_run.privileged);
    }

    #[test]
    fn elevate() {
        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        command_run.elevate();

        assert_eq!(String::from("echo"), command_run.command);

        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        command_run.privileged = true;
        command_run.elevate();

        assert_eq!(String::from("sudo"), command_run.command);
        assert_eq!(
            vec![String::from("echo"), String::from("Hello, world!")],
            command_run.arguments
        );
    }

    #[test]
    fn initializers() {
        use super::initializers::command_found::CommandFound;
        use super::initializers::FlowControl::SkipIf;

        // Ensure that no initializers always returns true
        let command_run = CommandRun {
            command: String::from("echo"),
            ..Default::default()
        };

        assert_eq!(true, command_run.plan());

        // Ensure that SkipIf returns false when satisfied
        let command_run = CommandRun {
            command: String::from("echo"),
            initializers: vec![
                SkipIf(Box::new(CommandFound("not-a-real-command"))),
                SkipIf(Box::new(CommandFound("ls"))),
                SkipIf(Box::new(CommandFound("not-a-real-command"))),
            ],
            ..Default::default()
        };

        assert_eq!(false, command_run.plan());

        // Ensure that SkipIf returns true when not satisfied
        let command_run = CommandRun {
            command: String::from("echo"),
            initializers: vec![
                SkipIf(Box::new(CommandFound("not-a-real-command"))),
                SkipIf(Box::new(CommandFound("not-a-real-command"))),
            ],
            ..Default::default()
        };

        assert_eq!(true, command_run.plan())
    }
}
