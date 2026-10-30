//! Runs read-only commands, and fakes them in tests

use std::{
    collections::HashMap,
    fmt, io,
    sync::{Arc, LazyLock, Mutex},
};

/// A command that only reads the system, like asking a package manager what's installed.
/// Commands that change the system are atoms, see [`crate::atoms::command::Exec`]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    /// The program, found in `PATH`
    pub program: String,
    /// Its arguments
    pub args: Vec<String>,
}

impl CommandSpec {
    /// `program` without arguments
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: vec![],
        }
    }

    /// Adds an argument
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Adds arguments
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }
}

impl fmt::Display for CommandSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.program)?;
        for arg in &self.args {
            write!(f, " {arg}")?;
        }
        Ok(())
    }
}

/// What a command returned
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandOutput {
    /// Whether it exited with 0
    pub success: bool,
    /// Its output
    pub stdout: Vec<u8>,
    /// Its error output
    pub stderr: Vec<u8>,
}

/// Runs read-only commands, so tests can replace the system with [`fake::FakeRunner`]
pub trait Runner: Send + Sync {
    /// Runs `command` and waits for it. Fails only if it can't be started
    fn run(&self, command: &CommandSpec) -> io::Result<CommandOutput>;

    /// Forgets remembered answers, after something changed the system. Only [`Cached`]
    /// remembers any
    fn forget(&self) {}
}

/// Runs commands on this machine
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, command: &CommandSpec) -> io::Result<CommandOutput> {
        let output = std::process::Command::new(&command.program)
            .args(&command.args)
            .output()?;

        Ok(CommandOutput {
            success: output.status.success(),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }
}

/// Remembers the answers of another runner, so each command runs once until [`Runner::forget`].
/// Failures to start a command aren't remembered
pub struct Cached {
    /// The runner that runs the commands
    inner: Arc<dyn Runner>,
    /// The answers by command line
    answers: Mutex<HashMap<String, CommandOutput>>,
}

impl Cached {
    /// Remembers the answers of `inner`
    pub fn new(inner: Arc<dyn Runner>) -> Self {
        Self {
            inner,
            answers: Mutex::default(),
        }
    }
}

impl Runner for Cached {
    fn run(&self, command: &CommandSpec) -> io::Result<CommandOutput> {
        let line = command.to_string();
        if let Some(output) = self.answers.lock().unwrap().get(&line) {
            return Ok(output.clone());
        }

        let output = self.inner.run(command)?;
        self.answers.lock().unwrap().insert(line, output.clone());
        Ok(output)
    }

    fn forget(&self) {
        self.answers.lock().unwrap().clear();
    }
}

/// The [`SystemRunner`], shared by the whole run, so a package manager is asked once what's
/// installed until a step installs something
pub fn system() -> Arc<dyn Runner> {
    static SYSTEM: LazyLock<Arc<Cached>> =
        LazyLock::new(|| Arc::new(Cached::new(Arc::new(SystemRunner))));

    SYSTEM.clone()
}

/// A runner that answers from a script, for tests
#[cfg(test)]
pub mod fake {
    use super::*;
    use std::{collections::HashMap, sync::Mutex};

    /// Answers commands from a script. A command without an answer fails like a missing program
    #[derive(Default)]
    pub struct FakeRunner {
        /// The answers by command line
        answers: HashMap<String, CommandOutput>,
        /// The command lines that ran, in order
        calls: Mutex<Vec<String>>,
    }

    impl FakeRunner {
        /// A runner without answers
        pub fn new() -> Self {
            Self::default()
        }

        /// Answers `command`, written like on a shell: `pacman -Q -q git`
        pub fn answer(mut self, command: &str, output: CommandOutput) -> Self {
            self.answers.insert(command.to_string(), output);
            self
        }

        /// The commands that ran, written like on a shell
        pub fn calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }

        /// The runner, to share with the code under test
        pub fn shared(self) -> Arc<Self> {
            Arc::new(self)
        }
    }

    impl Runner for FakeRunner {
        fn run(&self, command: &CommandSpec) -> io::Result<CommandOutput> {
            let line = command.to_string();
            self.calls.lock().unwrap().push(line.clone());

            self.answers.get(&line).cloned().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("no answer for `{line}`"))
            })
        }
    }

    /// A command that exits with 0 and prints `stdout`
    pub fn succeeds(stdout: &str) -> CommandOutput {
        CommandOutput {
            success: true,
            stdout: stdout.as_bytes().to_vec(),
            stderr: vec![],
        }
    }

    /// A command that fails, and prints `stdout` and `stderr`
    pub fn fails(stdout: &str, stderr: &str) -> CommandOutput {
        CommandOutput {
            success: false,
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_read_like_a_shell_line() {
        let command = CommandSpec::new("pacman").args(["-Q", "-q"]).arg("git");

        assert_eq!("pacman -Q -q git", command.to_string());
    }

    #[test]
    #[cfg(unix)]
    fn the_system_runner_runs_the_program() {
        let output = SystemRunner
            .run(&CommandSpec::new("sh").args(["-c", "echo out; echo err >&2; exit 3"]))
            .unwrap();

        assert!(!output.success);
        assert_eq!(b"out\n".to_vec(), output.stdout);
        assert_eq!(b"err\n".to_vec(), output.stderr);
    }

    #[test]
    fn cached_runs_each_command_once_until_it_forgets() {
        let fake = fake::FakeRunner::new()
            .answer("pacman -Q -q", fake::succeeds("git\n"))
            .shared();
        let cached = Cached::new(fake.clone());
        let command = CommandSpec::new("pacman").args(["-Q", "-q"]);

        cached.run(&command).unwrap();
        cached.run(&command).unwrap();
        cached.forget();
        let output = cached.run(&command).unwrap();

        assert_eq!(b"git\n".to_vec(), output.stdout);
        assert_eq!(vec!["pacman -Q -q", "pacman -Q -q"], fake.calls());
    }

    #[test]
    fn a_missing_program_is_not_found() {
        let error = SystemRunner
            .run(&CommandSpec::new("comtrya-no-such-program"))
            .unwrap_err();

        assert_eq!(io::ErrorKind::NotFound, error.kind());
    }
}
