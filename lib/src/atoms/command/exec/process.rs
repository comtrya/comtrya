//! Starting the program, reading its output line by line while it runs, and why it failed

use super::Exec;
use crate::atoms::{Error, Result};
use std::io::{self, BufRead, BufReader, Read};
use std::process::{Command, ExitStatus, Stdio};
use tracing::debug;

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
pub(super) struct Finished {
    /// The exit status
    pub(super) status: ExitStatus,
    /// All of stdout
    pub(super) stdout: Vec<u8>,
    /// All of stderr
    pub(super) stderr: Vec<u8>,
}

impl Exec {
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
    pub(super) fn run(&self, command: &str, on_line: OnLine) -> Result<Finished> {
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
                    on_line(Stream::Stdout, line);
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
}

/// Why a program failed: its exit code, or the signal that ended it, and its stderr
pub(super) fn failed(finished: Finished) -> Error {
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
pub(super) fn log_line(stream: Stream, line: &str) {
    match stream {
        Stream::Stdout => debug!("stdout: {line}"),
        Stream::Stderr => debug!("stderr: {line}"),
    }
}

// Every test runs `sh`
#[cfg(test)]
#[cfg(unix)]
mod tests {
    use super::*;
    use crate::atoms::command::exec::new_run_command;
    use crate::atoms::Change;
    use pretty_assertions::assert_eq;

    #[test]
    fn the_error_has_the_exit_code() {
        let mut command_run = new_run_command(String::from("sh"));
        command_run.arguments = vec![String::from("-c"), String::from("exit 3")];
        let error = command_run.execute().expect_err("Command should fail");

        assert_eq!("Command failed with exit code: 3", error.to_string());
    }

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

    /// Runs `script` with `sh -c` and returns the lines it printed, in order
    fn lines_of(command_run: &mut Exec, script: &str) -> (Result<()>, Vec<(Stream, String)>) {
        use std::sync::Mutex;

        command_run.command = String::from("sh");
        command_run.arguments = vec![String::from("-c"), String::from(script)];
        let lines = Mutex::new(vec![]);
        let result = command_run.execute_with_lines(&|stream, line| {
            lines.lock().unwrap().push((stream, line.to_string()));
        });

        (result, lines.into_inner().unwrap())
    }

    #[test]
    fn lines_arrive_in_order() {
        let (result, lines) = lines_of(&mut Exec::default(), "echo one; echo two; echo oops >&2");

        let stdout: Vec<_> = lines.iter().filter(|(s, _)| *s == Stream::Stdout).collect();
        assert_eq!(
            vec![
                &(Stream::Stdout, String::from("one")),
                &(Stream::Stdout, String::from("two"))
            ],
            stdout
        );
        assert!(lines.contains(&(Stream::Stderr, String::from("oops"))));
        result.unwrap();
    }

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
                    sender.lock().unwrap().send(line.to_string()).unwrap();
                })
            });

            // Long before the program ends
            let first = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
            assert_eq!("early", first);
            running.join().unwrap().unwrap();
        });
    }

    #[test]
    fn a_program_reading_stdin_gets_an_end_of_file() {
        // Waits forever if stdin is the terminal or an open pipe
        let (result, lines) = lines_of(&mut Exec::default(), "cat; echo done");

        result.unwrap();
        assert_eq!(vec![(Stream::Stdout, String::from("done"))], lines);
    }

    #[test]
    fn lines_that_arent_utf8_are_passed_lossily() {
        let (_, lines) = lines_of(&mut Exec::default(), "printf 'a\\377b\\n'");

        assert_eq!(vec![(Stream::Stdout, String::from("a\u{FFFD}b"))], lines);
    }

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

    #[test]
    fn a_program_killed_by_a_signal_says_so() {
        let (result, _) = lines_of(&mut Exec::default(), "echo dying >&2; kill -9 $$");

        assert_eq!(
            "Command was killed by signal 9: dying",
            result.unwrap_err().to_string()
        );
    }
}
