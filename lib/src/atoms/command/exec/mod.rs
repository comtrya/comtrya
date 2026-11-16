//! Runs a program and returns its output
//!
//! The atom itself lives here. Starting the program and reading what it prints is in
//! [`process`], what happens with stdout once it ended is in [`stdout`]

mod process;
mod stdout;

pub use process::{OnLine, Stream};

use process::{failed, log_line};

use crate::atoms::{Check, Error, Result};
use serde::{Deserialize, Serialize};

use super::super::Change;
use crate::contexts::privilege::Privilege;
use crate::utilities;
use std::path::PathBuf;

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
    /// Fails the step when stdout contains this text, in any case, like a dry run of a package
    /// manager that would remove packages. The action stops there
    pub stop_if_stdout_contains: Option<String>,
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
    fn find_binary(&self, command: &str) -> Option<String> {
        match self.environment.iter().rev().find(|(key, _)| key == "PATH") {
            Some((_, path)) => {
                let cwd = match &self.working_dir {
                    Some(dir) => PathBuf::from(dir),
                    None => std::env::current_dir().ok()?,
                };
                let binary = which::which_in(command, Some(path), cwd).ok()?;
                Some(binary.to_string_lossy().to_string())
            }
            None => utilities::get_binary_path(command).ok(),
        }
    }
}

impl std::fmt::Display for Exec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Not the environment, it may hold secrets
        write!(f, "Run {}", self.command)?;
        for argument in &self.arguments {
            write!(f, " {argument}")?;
        }
        Ok(())
    }
}

impl Change for Exec {
    fn check(&self) -> Result<Check> {
        Ok(Check::Drift(None))
    }

    fn execute(&self) -> Result<()> {
        self.execute_with_lines(&log_line)
    }
}

impl Exec {
    /// Runs the program like [`Atom::execute`](crate::atoms::Atom::execute), and passes each line it prints to `on_line`
    /// while it runs
    ///
    /// # Errors
    ///
    /// When the program isn't found, can't start, exits with an error or is killed, when its
    /// stdout contains the `stop_if_stdout_contains` text, or when `stdout_file` can't be written
    pub fn execute_with_lines(&self, on_line: OnLine) -> Result<()> {
        let command = self
            .find_binary(&self.command)
            .ok_or_else(|| Error::CommandNotFound(self.command.clone()))?;

        let finished = self.run(&command, on_line)?;
        if !finished.status.success() {
            return Err(failed(finished));
        }
        self.stop_if_stdout_contains(&finished.stdout)?;
        self.write_stdout_file(&finished.stdout)
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

    #[test]
    fn error_propagation() {
        let command_run = new_run_command(String::from("non-existant-command"));
        command_run.execute().expect_err("Command should fail");
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
}
