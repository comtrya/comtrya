//! Runs a program and returns its output

use crate::atoms::{Check, Error, Output, Result};
use serde::{Deserialize, Serialize};

use super::super::Change;
use crate::contexts::privilege::Privilege;
use crate::utilities;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use tracing::debug;

/// Runs a program, every time. Fails when the program exits with an error
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Exec {
    /// The program to run, found in `PATH`
    pub command: String,
    /// The arguments of the program
    pub arguments: Vec<String>,
    /// Where the program runs. Where comtrya runs without it
    pub working_dir: Option<String>,
    /// Environment variables for the program only. A `PATH` here is also where the program is looked up
    pub environment: Vec<(String, String)>,
    /// Writes stdout to this file instead of logging it
    pub stdout_file: Option<PathBuf>,
}

/// Runs `command` without arguments
#[allow(dead_code)]
pub fn new_run_command(command: String) -> Exec {
    Exec {
        command,
        ..Default::default()
    }
}

impl Exec {
    /// The same command run through `privilege`, like `sudo pacman -S …`
    pub fn elevated(self, privilege: &Privilege) -> Exec {
        Exec {
            command: privilege.to_string(),
            arguments: [vec![self.command], self.arguments].concat(),
            ..self
        }
    }

    /// Looks the command up in the `PATH` set for it, if there is one
    fn find_binary(&self, command: &str) -> anyhow::Result<String> {
        match self.environment.iter().rev().find(|(key, _)| key == "PATH") {
            Some((_, path)) => {
                let cwd = match &self.working_dir {
                    Some(dir) => PathBuf::from(dir),
                    None => std::env::current_dir()?,
                };
                Ok(which::which_in(command, Some(path), cwd)?
                    .to_string_lossy()
                    .to_string())
            }
            None => utilities::get_binary_path(command),
        }
    }

    /// The process to start, with the environment and working directory of this command
    fn process(&self, command: &str, arguments: &[String]) -> Command {
        let mut process = Command::new(command);
        process.envs(self.environment.clone()).args(arguments);
        // Without one, the command runs where comtrya runs
        if let Some(working_dir) = &self.working_dir {
            process.current_dir(working_dir);
        }
        process
    }

    /// The output for the finalizers. Stdout goes to `stdout_file` instead, if there is one
    fn succeeded(&self, output: process::Output) -> Result<Output> {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if let Some(stdout_file) = &self.stdout_file {
            write_stdout(stdout_file, &output.stdout)?;
            return Ok(Output {
                stdout: String::new(),
                stderr,
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        debug!("stdout: {stdout}");

        Ok(Output { stdout, stderr })
    }

    /// Fails with the exit code (1 when a signal ended the program) and stderr
    fn failed(&self, output: process::Output) -> Result<Output> {
        let code = output.status.code().unwrap_or(1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        debug!("exit code: {code}");
        debug!("stdout: {stdout}");
        debug!("stderr: {stderr}");

        Err(Error::CommandFailed {
            code,
            stderr: stderr.trim().to_string(),
        })
    }
}

/// Writes the output of a command to `path`, creating its directory if needed
fn write_stdout(path: &Path, stdout: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::io("create the directory", parent))?;
    }
    std::fs::write(path, stdout).map_err(Error::io("write stdout to", path))?;

    debug!("stdout written to {}", path.display());

    Ok(())
}

impl std::fmt::Display for Exec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CommandExec with: {} {}",
            self.command,
            self.arguments.join(" ")
        )
    }
}

impl Change for Exec {
    fn check(&self) -> Result<Check> {
        Ok(Check::Drift(None))
    }

    fn execute(&self) -> Result<()> {
        self.execute_with_output().map(|_| ())
    }

    fn execute_with_output(&self) -> Result<Output> {
        let command = self
            .find_binary(&self.command)
            .map_err(|_| Error::CommandNotFound(self.command.clone()))?;

        let output = self
            .process(&command, &self.arguments)
            .output()
            .map_err(|source| Error::Spawn {
                command: command.clone(),
                source,
            })?;

        if output.status.success() {
            self.succeeded(output)
        } else {
            self.failed(output)
        }
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

        let command_run = new_run_command(String::from("echo"));

        assert_eq!(String::from("echo"), command_run.command);
        assert_eq!(0, command_run.arguments.len());
        assert_eq!(None, command_run.working_dir);
        assert_eq!(0, command_run.environment.len());
    }

    #[test]
    fn elevated_runs_the_command_through_the_privilege_provider() {
        for (provider, name) in [
            (Privilege::Sudo, "sudo"),
            (Privilege::Doas, "doas"),
            (Privilege::Run0, "run0"),
        ] {
            let mut command_run = new_run_command(String::from("echo"));
            command_run.arguments = vec![String::from("Hello, world!")];
            command_run.working_dir = Some(String::from("/tmp"));

            let elevated = command_run.elevated(&provider);

            assert_eq!(name, elevated.command);
            assert_eq!(vec!["echo", "Hello, world!"], elevated.arguments);
            assert_eq!(Some(String::from("/tmp")), elevated.working_dir);
        }
    }

    #[cfg(unix)]
    #[test]
    fn stdout_is_written_to_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let stdout_file = dir.path().join("nested").join("out.txt");

        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("hello")];
        command_run.stdout_file = Some(stdout_file.clone());
        let output = command_run.execute_with_output().unwrap();

        assert_eq!("hello\n", std::fs::read_to_string(stdout_file).unwrap());
        assert_eq!("", output.stdout);
    }

    #[cfg(unix)]
    #[test]
    fn stdout_file_is_left_alone_when_the_command_fails() {
        let dir = tempfile::tempdir().unwrap();
        let stdout_file = dir.path().join("out.txt");
        std::fs::write(&stdout_file, "before").unwrap();

        let mut command_run = new_run_command(String::from("sh"));
        command_run.arguments = vec![String::from("-c"), String::from("echo after; exit 1")];
        command_run.stdout_file = Some(stdout_file.clone());
        command_run.execute().expect_err("Command should fail");

        assert_eq!("before", std::fs::read_to_string(stdout_file).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn the_error_has_the_exit_code() {
        let mut command_run = new_run_command(String::from("sh"));
        command_run.arguments = vec![String::from("-c"), String::from("exit 3")];
        let error = command_run.execute().expect_err("Command should fail");

        assert_eq!("Command failed with exit code: 3", error.to_string());
    }

    #[cfg(unix)]
    #[test]
    fn the_output_is_returned() {
        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("hello")];

        assert_eq!(
            "hello",
            command_run.execute_with_output().unwrap().stdout.trim()
        );
    }

    #[test]
    fn error_propagation() {
        let command_run = new_run_command(String::from("non-existant-command"));
        command_run.execute().expect_err("Command should fail");
    }

    #[cfg(unix)]
    #[test]
    fn environment_is_only_set_for_the_command() {
        let dir = tempfile::tempdir().unwrap();
        let stdout_file = dir.path().join("out.txt");

        let mut command_run = new_run_command(String::from("sh"));
        command_run.arguments = vec![
            String::from("-c"),
            String::from("printf %s \"$COMTRYA_EXEC_TEST\""),
        ];
        command_run.environment = vec![(
            String::from("COMTRYA_EXEC_TEST"),
            String::from("only-for-the-child"),
        )];
        command_run.stdout_file = Some(stdout_file.clone());
        command_run.execute().unwrap();

        assert_eq!(
            "only-for-the-child",
            std::fs::read_to_string(stdout_file).unwrap()
        );
        assert!(std::env::var("COMTRYA_EXEC_TEST").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn the_command_is_found_in_its_own_path() {
        use std::os::unix::fs::PermissionsExt;

        let bin = tempfile::tempdir().unwrap();
        let script = bin.path().join("comtrya-exec-test");
        std::fs::write(&script, "#!/bin/sh\nprintf found\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let stdout_file = bin.path().join("out.txt");

        let mut command_run = new_run_command(String::from("comtrya-exec-test"));
        command_run.environment = vec![(
            String::from("PATH"),
            format!("{}:/usr/bin:/bin", bin.path().display()),
        )];
        command_run.stdout_file = Some(stdout_file.clone());
        command_run.execute().unwrap();

        assert_eq!("found", std::fs::read_to_string(stdout_file).unwrap());
        assert!(std::env::var("PATH").is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn output_that_isnt_utf8_doesnt_fail_the_command() {
        let mut command_run = new_run_command(String::from("printf"));
        command_run.arguments = vec![String::from("a\\377b")];

        let output = command_run.execute_with_output().unwrap();

        assert_eq!("a\u{FFFD}b", output.stdout);
    }
}
