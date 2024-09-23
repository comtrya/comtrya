use crate::atoms::Outcome;

use super::super::Atom;
use crate::utilities;
use anyhow::anyhow;
use tracing::debug;

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
        // Depending on the priviledged flag and who who the current user is
        // we can determine if we need to prepend sudo to the command
        match (self.privileged, whoami::username().as_str()) {
            // Hasn't requested priviledged, so never try to elevate
            (false, _) => (self.command.clone(), self.arguments.clone()),

            // Requested priviledged, but is already root
            (true, "root") => (self.command.clone(), self.arguments.clone()),

            // Requested priviledged, but is not root
            (true, _) => (
                self.sudo_path().unwrap(),
                [vec![self.command.clone()], self.arguments.clone()].concat(),
            ),
        }
    }

    fn sudo_path(&self) -> anyhow::Result<String> {
        match utilities::get_binary_path("sudo") {
            Ok(path) => Ok(path),
            Err(_) => match utilities::get_binary_path("doas") {
                Ok(path) => Ok(path),
                Err(err) => {
                    return Err(anyhow!(
                        "Command requires sudo, but neither sudo nor doas exists: {}",
                        err
                    ))
                }
            },
        }
    }

    fn elevate(&mut self) -> anyhow::Result<()> {
        tracing::info!(
            "Sudo required for privilege elevation to run `{} {}`. Validating sudo ...",
            &self.command,
            &self.arguments.join(" ")
        );

        match std::process::Command::new(self.sudo_path().unwrap())
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .arg("--validate")
            .output()
        {
            Ok(std::process::Output { status, .. }) if status.success() => Ok(()),

            Ok(std::process::Output { stderr, .. }) => Err(anyhow!(
                "Command requires sudo, but couldn't elevate privileges: {}",
                String::from_utf8(stderr)?
            )),

            Err(err) => Err(anyhow!(
                "Command requires sudo, but couldn't elevate privileges: {}",
                err
            )),
        }
    }
}

impl std::fmt::Display for Exec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CommandExec with: privileged={}: {} {}",
            self.privileged,
            self.command,
            self.arguments.join(" ")
        )
    }
}

impl Atom for Exec {
    fn plan(&self) -> anyhow::Result<Outcome> {
        Ok(Outcome {
            // Commands may have side-effects, but none that can be "known"
            // without some sandboxed operations to detect filesystem and network
            // affects.
            // Maybe we'll look into this one day?
            side_effects: vec![],
            // Commands should always run, we have no cache-key based
            // determinism atm the moment.
            should_run: true,
        })
    }

    fn execute(&mut self) -> anyhow::Result<()> {
        let (mut command, arguments) = self.elevate_if_required();

        // If we require root, we need to use sudo with inherited IO
        // to ensure the user can respond if prompted for a password
        let command_prefix = command
            .split(' ')
            .next()
            .expect("Could not find command prefix");
        if command_prefix.ends_with("sudo") || command.ends_with("doas") {
            match self.elevate() {
                Ok(_) => (),
                Err(err) => {
                    return Err(anyhow!(err));
                }
            }
        } else {
            command = utilities::get_binary_path(&command)
                .or_else(|_| Err(anyhow!("Command `{}` not found in path", command)))?;
        }

        match std::process::Command::new(&command)
            .envs(self.environment.clone())
            .args(&arguments)
            .current_dir(&self.working_dir.clone().unwrap_or_else(|| {
                std::env::current_dir()
                    .map(|current_dir| current_dir.display().to_string())
                    .expect("Failed to get current directory")
            }))
            .output()
        {
            Ok(output) if output.status.success() => {
                self.status.stdout = String::from_utf8(output.stdout)?;
                self.status.stderr = String::from_utf8(output.stderr)?;

                debug!("stdout: {}", &self.status.stdout);

                Ok(())
            }

            Ok(output) => {
                self.status.stdout = String::from_utf8(output.stdout)?;
                self.status.stderr = String::from_utf8(output.stderr)?;

                debug!("exit code: {}", &self.status.code);
                debug!("stdout: {}", &self.status.stdout);
                debug!("stderr: {}", &self.status.stderr);

                Err(anyhow!(
                    "Command failed with exit code: {}",
                    output.status.code().unwrap_or(1)
                ))
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
    use pretty_assertions::assert_eq;

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

        assert_eq!(command.ends_with("sudo") || command.ends_with("doas"), true);
        assert_eq!(
            vec![String::from("echo"), String::from("Hello, world!")],
            args
        );
    }

    #[test]
    fn error_propagation() {
        let mut command_run = new_run_command(String::from("non-existant-command"));
        command_run.execute().expect_err("Command should fail");
    }
}
