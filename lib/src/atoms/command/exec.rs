//! Runs a program and returns its output

use crate::atoms::{Check, Error, Output, Result};
use serde::{Deserialize, Serialize};

use super::super::Change;
use crate::contexts::privilege::Privilege;
use crate::utilities;
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
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

/// Which output of a program a line is from
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    /// Standard output
    Stdout,
    /// Standard error
    Stderr,
}

/// Gets each line a program prints, while it runs, without the line break
pub type OnLine<'a> = &'a (dyn Fn(Stream, &str) + Sync);

/// How a program ended, and everything it printed
struct Finished {
    /// The exit status
    status: ExitStatus,
    /// All of stdout
    stdout: Vec<u8>,
    /// All of stderr
    stderr: Vec<u8>,
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

    /// The process to start, with the environment and working directory of this command.
    ///
    /// Stdin is empty, so a program that asks a question gets an end of file instead of waiting
    /// for an answer nobody gives. sudo, doas and run0 ask for the password on the terminal
    /// itself, not on stdin, so they still can
    fn process(&self, command: &str, arguments: &[String]) -> Command {
        let mut process = Command::new(command);
        process
            .envs(self.environment.clone())
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Without one, the command runs where comtrya runs
        if let Some(working_dir) = &self.working_dir {
            process.current_dir(working_dir);
        }
        process
    }

    /// Runs the program to its end, and passes each line it prints to `on_line` as it comes.
    /// Stdout isn't passed when it goes to `stdout_file`, it may be a secret
    fn run(&self, command: &str, on_line: OnLine) -> Result<Finished> {
        let spawn_error = |source| Error::Spawn {
            command: command.to_string(),
            source,
        };

        let mut child = self
            .process(command, &self.arguments)
            .spawn()
            .map_err(spawn_error)?;
        let stdout = child.stdout.take().expect("stdout is piped");
        let stderr = child.stderr.take().expect("stderr is piped");
        let show_stdout = self.stdout_file.is_none();

        // Both pipes are read at once, so a program that fills one while comtrya waits on the
        // other can't get stuck
        let (stdout, stderr) = std::thread::scope(|scope| {
            let stderr = scope.spawn(|| read_lines(stderr, &|line| on_line(Stream::Stderr, line)));
            let stdout = read_lines(stdout, &|line| {
                if show_stdout {
                    on_line(Stream::Stdout, line)
                }
            });
            (stdout, stderr.join().expect("reading stderr doesn't panic"))
        });

        let status = child.wait().map_err(spawn_error)?;
        Ok(Finished {
            status,
            stdout: stdout.map_err(spawn_error)?,
            stderr: stderr.map_err(spawn_error)?,
        })
    }

    /// The output for the finalizers. Stdout goes to `stdout_file` instead, if there is one
    fn succeeded(&self, finished: Finished) -> Result<Output> {
        let stderr = String::from_utf8_lossy(&finished.stderr).into_owned();

        if let Some(stdout_file) = &self.stdout_file {
            write_stdout(stdout_file, &finished.stdout)?;
            return Ok(Output {
                stdout: String::new(),
                stderr,
            });
        }

        let stdout = String::from_utf8_lossy(&finished.stdout).into_owned();
        Ok(Output { stdout, stderr })
    }
}

/// Why a program failed: its exit code, or the signal that ended it, and its stderr
fn failed(finished: Finished) -> Error {
    let stderr = String::from_utf8_lossy(&finished.stderr).trim().to_string();

    if let Some(signal) = signal(&finished.status) {
        debug!("killed by signal {signal}");
        return Error::Killed { signal, stderr };
    }

    let code = finished.status.code().unwrap_or(1);
    debug!("exit code: {code}");
    Error::CommandFailed { code, stderr }
}

/// The signal that ended the program, if one did
#[cfg(unix)]
fn signal(status: &ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;

    status.signal()
}

/// Windows has no signals, a program always has an exit code
#[cfg(not(unix))]
fn signal(_: &ExitStatus) -> Option<i32> {
    None
}

/// Reads `pipe` to its end, passes each line to `on_line` as it arrives, and returns all of it.
/// Lines that aren't UTF-8 are passed with `�` for the invalid bytes
fn read_lines(pipe: impl Read, on_line: &dyn Fn(&str)) -> io::Result<Vec<u8>> {
    let mut reader = BufReader::new(pipe);
    let mut all = Vec::new();

    loop {
        let start = all.len();
        if reader.read_until(b'\n', &mut all)? == 0 {
            return Ok(all);
        }
        let line = String::from_utf8_lossy(&all[start..]);
        on_line(line.trim_end_matches(['\n', '\r']));
    }
}

/// Logs a line of a program's output, the default for [`Exec::execute_with_lines`]
fn log_line(stream: Stream, line: &str) {
    match stream {
        Stream::Stdout => debug!("stdout: {line}"),
        Stream::Stderr => debug!("stderr: {line}"),
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
        self.execute_with_lines(&log_line)
    }
}

impl Exec {
    /// Runs the program like [`Change::execute_with_output`], and passes each line it prints to
    /// `on_line` while it runs
    pub fn execute_with_lines(&self, on_line: OnLine) -> Result<Output> {
        let command = self
            .find_binary(&self.command)
            .map_err(|_| Error::CommandNotFound(self.command.clone()))?;

        let finished = self.run(&command, on_line)?;
        if finished.status.success() {
            self.succeeded(finished)
        } else {
            Err(failed(finished))
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

    /// Runs `script` with `sh -c` and returns the lines it printed, in order
    #[cfg(unix)]
    fn lines_of(command_run: &mut Exec, script: &str) -> (Result<Output>, Vec<(Stream, String)>) {
        use std::sync::Mutex;

        command_run.command = String::from("sh");
        command_run.arguments = vec![String::from("-c"), String::from(script)];
        let lines = Mutex::new(vec![]);
        let result = command_run.execute_with_lines(&|stream, line| {
            lines.lock().unwrap().push((stream, line.to_string()))
        });

        (result, lines.into_inner().unwrap())
    }

    #[cfg(unix)]
    #[test]
    fn lines_arrive_in_order_and_are_still_returned() {
        let (output, lines) = lines_of(&mut Exec::default(), "echo one; echo two; echo oops >&2");

        let stdout: Vec<_> = lines.iter().filter(|(s, _)| *s == Stream::Stdout).collect();
        assert_eq!(
            vec![
                &(Stream::Stdout, String::from("one")),
                &(Stream::Stdout, String::from("two"))
            ],
            stdout
        );
        assert!(lines.contains(&(Stream::Stderr, String::from("oops"))));
        let output = output.unwrap();
        assert_eq!("one\ntwo\n", output.stdout);
        assert_eq!("oops\n", output.stderr);
    }

    #[cfg(unix)]
    #[test]
    fn lines_are_passed_while_the_program_runs() {
        use std::sync::mpsc;
        use std::time::Duration;

        let (sender, receiver) = mpsc::channel();
        let sender = std::sync::Mutex::new(sender);
        let command_run = Exec {
            command: String::from("sh"),
            arguments: vec![String::from("-c"), String::from("echo early; sleep 2")],
            ..Default::default()
        };

        std::thread::scope(|scope| {
            let running = scope.spawn(|| {
                command_run.execute_with_lines(&|_, line| {
                    sender.lock().unwrap().send(line.to_string()).unwrap()
                })
            });

            // Long before the program ends
            let first = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
            assert_eq!("early", first);
            running.join().unwrap().unwrap();
        });
    }

    #[cfg(unix)]
    #[test]
    fn a_program_reading_stdin_gets_an_end_of_file() {
        // Waits forever if stdin is the terminal or an open pipe
        let (output, _) = lines_of(&mut Exec::default(), "cat; echo done");

        assert_eq!("done\n", output.unwrap().stdout);
    }

    #[cfg(unix)]
    #[test]
    fn lines_that_arent_utf8_are_passed_lossily() {
        let (_, lines) = lines_of(&mut Exec::default(), "printf 'a\\377b\\n'");

        assert_eq!(vec![(Stream::Stdout, String::from("a\u{FFFD}b"))], lines);
    }

    #[cfg(unix)]
    #[test]
    fn stdout_for_a_file_isnt_passed() {
        let dir = tempfile::tempdir().unwrap();
        let mut command_run = Exec {
            stdout_file: Some(dir.path().join("out.txt")),
            ..Default::default()
        };

        let (_, lines) = lines_of(&mut command_run, "echo secret; echo note >&2");

        assert_eq!(vec![(Stream::Stderr, String::from("note"))], lines);
    }

    #[cfg(unix)]
    #[test]
    fn a_program_killed_by_a_signal_says_so() {
        let (result, _) = lines_of(&mut Exec::default(), "echo dying >&2; kill -9 $$");

        assert_eq!(
            "Command was killed by signal 9: dying",
            result.unwrap_err().to_string()
        );
    }
}
