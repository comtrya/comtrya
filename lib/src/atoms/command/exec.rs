//! Runs a program, with the privilege provider if needed, and keeps its output

use crate::atoms::Outcome;

use super::super::Atom;
use crate::utilities;
use anyhow::{anyhow, Context};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tracing::debug;

/// Runs a program, every time. Fails when the program exits with an error
#[derive(Default)]
pub struct Exec {
    /// The program to run, found in `PATH`
    pub command: String,
    /// The arguments of the program
    pub arguments: Vec<String>,
    /// Where the program runs. Where comtrya runs without it
    pub working_dir: Option<String>,
    /// Environment variables for the program only. A `PATH` here is also where the program is looked up
    pub environment: Vec<(String, String)>,
    /// Run the program with `privilege_provider`, unless comtrya runs as root
    pub privileged: bool,
    /// The program that runs commands as root, like `sudo`, `doas` or `run0`
    pub privilege_provider: String,
    /// Writes stdout to this file instead of logging it
    pub stdout_file: Option<PathBuf>,
    /// What the last run returned
    pub(crate) status: ExecStatus,
}

/// What a run of the program returned
#[derive(Default)]
pub(crate) struct ExecStatus {
    /// The exit code: 0 on success, 1 when a signal ended the program
    code: i32,
    /// The output, unless it went to `stdout_file`
    stdout: String,
    /// The error output
    stderr: String,
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
    /// The program and arguments to run: prefixed with the privilege provider if needed
    fn elevate_if_required(&self) -> (String, Vec<String>) {
        // Depending on the priviledged flag and who who the current user is
        // we can determine if we need to prepend sudo to the command

        let privilege_provider = self.privilege_provider.clone();
        let username = whoami::username().unwrap_or_else(|_| String::from("unknown"));

        match (self.privileged, username.as_str()) {
            // Hasn't requested priviledged, so never try to elevate
            (false, _) => (self.command.clone(), self.arguments.clone()),

            // Requested priviledged, but is already root
            (true, "root") => (self.command.clone(), self.arguments.clone()),

            // Requested priviledged, but is not root
            (true, _) => (
                privilege_provider,
                [vec![self.command.clone()], self.arguments.clone()].concat(),
            ),
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

    /// Keeps stderr, and writes stdout to `stdout_file` or keeps it for `output_string`
    fn succeeded(&mut self, output: Output) -> anyhow::Result<()> {
        self.status.stderr = String::from_utf8(output.stderr)?;

        if let Some(stdout_file) = &self.stdout_file {
            return write_stdout(stdout_file, &output.stdout);
        }

        self.status.stdout = String::from_utf8(output.stdout)?;
        debug!("stdout: {}", &self.status.stdout);

        Ok(())
    }

    /// Keeps the output, and fails with the exit code and stderr
    fn failed(&mut self, output: Output) -> anyhow::Result<()> {
        let code = output.status.code().unwrap_or(1);
        self.status.code = code;
        self.status.stdout = String::from_utf8(output.stdout)?;
        self.status.stderr = String::from_utf8(output.stderr)?;

        debug!("exit code: {}", &self.status.code);
        debug!("stdout: {}", &self.status.stdout);
        debug!("stderr: {}", &self.status.stderr);

        match self.status.stderr.trim() {
            "" => Err(anyhow!("Command failed with exit code: {code}")),
            stderr => Err(anyhow!("Command failed with exit code: {code}: {stderr}")),
        }
    }
}

/// Writes the output of a command to `path`, creating its directory if needed
fn write_stdout(path: &Path, stdout: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, stdout)
        .with_context(|| format!("Failed to write stdout to {}", path.display()))?;

    debug!("stdout written to {}", path.display());

    Ok(())
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
        let (command, arguments) = self.elevate_if_required();

        let command = self
            .find_binary(&command)
            .map_err(|_| anyhow!("Command `{command}` not found in path"))?;

        let output = self.process(&command, &arguments).output()?;

        if output.status.success() {
            self.succeeded(output)
        } else {
            self.failed(output)
        }
    }

    fn output_string(&self) -> String {
        self.status.stdout.clone()
    }

    fn error_message(&self) -> String {
        self.status.stderr.clone()
    }

    fn status_code(&self) -> i32 {
        self.status.code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::privilege::Privilege;
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
        command_run.privilege_provider = Privilege::Sudo.to_string();
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("sudo"), command);
        assert_eq!(
            vec![String::from("echo"), String::from("Hello, world!")],
            args
        );
    }

    #[test]
    fn elevate_doas() {
        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("echo"), command);
        assert_eq!(vec![String::from("Hello, world!")], args);

        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        command_run.privileged = true;
        command_run.privilege_provider = Privilege::Doas.to_string();
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("doas"), command);
        assert_eq!(
            vec![String::from("echo"), String::from("Hello, world!")],
            args
        );
    }
    #[test]
    fn elevate_run0() {
        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("echo"), command);
        assert_eq!(vec![String::from("Hello, world!")], args);

        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("Hello, world!")];
        command_run.privileged = true;
        command_run.privilege_provider = Privilege::Run0.to_string();
        let (command, args) = command_run.elevate_if_required();

        assert_eq!(String::from("run0"), command);
        assert_eq!(
            vec![String::from("echo"), String::from("Hello, world!")],
            args
        );
    }

    #[cfg(unix)]
    #[test]
    fn stdout_is_written_to_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let stdout_file = dir.path().join("nested").join("out.txt");

        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("hello")];
        command_run.stdout_file = Some(stdout_file.clone());
        command_run.execute().unwrap();

        assert_eq!("hello\n", std::fs::read_to_string(stdout_file).unwrap());
        assert_eq!("", command_run.output_string());
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
    fn the_exit_code_is_kept() {
        let mut command_run = new_run_command(String::from("sh"));
        command_run.arguments = vec![String::from("-c"), String::from("exit 3")];
        command_run.execute().expect_err("Command should fail");

        assert_eq!(3, command_run.status_code());
    }

    #[test]
    fn error_propagation() {
        let mut command_run = new_run_command(String::from("non-existant-command"));
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
}
