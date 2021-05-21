use super::super::Atom;
use anyhow::anyhow;

#[derive(Default)]
pub struct Exec {
    pub command: String,
    pub arguments: Vec<String>,
    pub working_dir: Option<String>,
    pub environment: Vec<(String, String)>,
    pub privileged: bool,
    pub(crate) status: ExecStatus,
}

#[derive(Default)]
pub(crate) struct ExecStatus {
    code: i32,
    stdout: String,
    stderr: String,
}

#[allow(dead_code)]
pub fn new_run_command(command: String) -> Exec {
    Exec {
        command,
        ..Default::default()
    }
}

impl Exec {
    fn elevate_if_required(&self) -> (String, Vec<String>) {
        if !self.privileged {
            return (self.command.clone(), self.arguments.clone());
        }

        if "root" == whoami::username() {
            return (self.command.clone(), self.arguments.clone());
        }

        (
            String::from("sudo"),
            [vec![self.command.clone()], self.arguments.clone()].concat(),
        )
    }
}

impl std::fmt::Display for Exec {
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

impl Atom for Exec {
    fn plan(&self) -> bool {
        true
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let (command, arguments) = self.elevate_if_required();

        // If we require root, we need to use sudo with inherited IO
        // to ensure the user can respond if prompted for a password
        if command.eq("sudo") {
            tracing::info!(
                "Sudo required for privilege elevation to run `{} {}`. Validating sudo ...",
                &command,
                arguments.join(" ")
            );

            match std::process::Command::new("sudo")
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .arg("--validate")
                .output()
            {
                Ok(std::process::Output { status, .. }) if status.success() => (),

                _ => {
                    return Err(anyhow!(
                        "Command requires sudo, but couldn't elevate privileges."
                    ))
                }
            };
        }

        match std::process::Command::new(&command)
            .envs(self.environment.clone())
            .args(&arguments)
            .current_dir(
                &self.working_dir.clone().unwrap_or(
                    std::env::current_dir()
                        .unwrap()
                        .into_os_string()
                        .into_string()
                        .unwrap(),
                ),
            )
            .output()
        {
            Ok(output) => {
                self.status.code = output.status.code().unwrap();
                self.status.stdout = String::from_utf8(output.stdout).unwrap();
                self.status.stderr = String::from_utf8(output.stderr).unwrap();

                Ok(())
            }
            Err(err) => Err(anyhow!(err)),
        }
    }

    fn output_string(&self) -> String {
        self.status.stdout.clone()
    }

    fn error_message(&self) -> String {
        self.status.stderr.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        let command_run = Exec {
            ..Default::default()
        };

        assert_eq!(String::from(""), command_run.command);
        assert_eq!(0, command_run.arguments.len());
        assert_eq!(None, command_run.working_dir);
        assert_eq!(0, command_run.environment.len());
        assert_eq!(false, command_run.privileged);

        let command_run = new_run_command(String::from("echo"));

        assert_eq!(String::from("echo"), command_run.command);
        assert_eq!(0, command_run.arguments.len());
        assert_eq!(None, command_run.working_dir);
        assert_eq!(0, command_run.environment.len());
        assert_eq!(false, command_run.privileged);
    }

    #[test]
    fn elevate() {
        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("echo"), command);
        assert_eq!(vec![String::from("Hello, world!")], args);

        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        command_run.privileged = true;
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("sudo"), command);
        assert_eq!(
            vec![String::from("echo"), String::from("Hello, world!")],
            args
        );
    }

    // #[test]
    // fn initializers() {
    //     use super::initializers::command_found::CommandFound;
    //     use super::initializers::FlowControl::SkipIf;

    //     // Ensure that no initializers always returns true
    //     let command_run = Exec {
    //         command: String::from("echo"),
    //         ..Default::default()
    //     };

    //     assert_eq!(true, command_run.plan());

    //     // Ensure that SkipIf returns false when satisfied
    //     let command_run = Exec {
    //         command: String::from("echo"),
    //         initializers: vec![
    //             SkipIf(Box::new(CommandFound("not-a-real-command"))),
    //             SkipIf(Box::new(CommandFound("ls"))),
    //             SkipIf(Box::new(CommandFound("not-a-real-command"))),
    //         ],
    //         ..Default::default()
    //     };

    //     assert_eq!(false, command_run.plan());

    //     // Ensure that SkipIf returns true when not satisfied
    //     let command_run = Exec {
    //         command: String::from("echo"),
    //         initializers: vec![
    //             SkipIf(Box::new(CommandFound("not-a-real-command"))),
    //             SkipIf(Box::new(CommandFound("not-a-real-command"))),
    //         ],
    //         ..Default::default()
    //     };

    //     assert_eq!(true, command_run.plan())
    // }

    // #[test]
    // fn finalizers() {
    //     use super::finalizers::always_succeed::AlwaysSucceed;
    //     use super::finalizers::FlowControl::{ErrorIf, FinishIf};

    //     // Nothing changes when using no finalizers
    //     let mut command_run = Exec {
    //         command: String::from("echo"),
    //         ..Default::default()
    //     };

    //     let result = command_run.execute();
    //     assert_eq!(true, result.is_ok());

    //     let mut command_run = Exec {
    //         command: String::from("not-a-command"),
    //         ..Default::default()
    //     };

    //     let result = command_run.execute();
    //     assert_eq!(true, result.is_err());

    //     // AlwaysSucceed
    //     let mut command_run = Exec {
    //         command: String::from("not-a-command"),
    //         finalizers: vec![FinishIf(Box::new(AlwaysSucceed {}))],
    //         ..Default::default()
    //     };

    //     let result = command_run.execute();
    //     assert_eq!(true, result.is_ok());

    //     let mut command_run = Exec {
    //         command: String::from("not-a-command"),
    //         finalizers: vec![ErrorIf(Box::new(AlwaysSucceed {}))],
    //         ..Default::default()
    //     };

    //     let result = command_run.execute();
    //     assert_eq!(true, result.is_err());

    //     let mut command_run = Exec {
    //         command: String::from("not-a-command"),
    //         finalizers: vec![
    //             FinishIf(Box::new(AlwaysSucceed {})),
    //             ErrorIf(Box::new(AlwaysSucceed {})),
    //         ],
    //         ..Default::default()
    //     };

    //     let result = command_run.execute();
    //     assert_eq!(true, result.is_ok());
    // }
}
