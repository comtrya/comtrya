//! The privileged helper: `sudo comtrya __helper` runs the atoms of root steps that aren't
//! programs, like writing a file below `/etc`. It's started once per run, on the first such
//! step, so the privilege provider asks for the password once.
//!
//! The protocol is one JSON line per request, a serialized [`Atom`], and one JSON line per
//! answer, `{"Ok": <Output>}` or `{"Err": "<the error and its causes>"}`. The helper only runs
//! what its parent sends, the same trust as `sudo` running a program. Requests can hold secrets,
//! like the passphrase of encrypted contents, so neither side logs them

use crate::atoms::{self, Atom, Output};
use crate::contexts::privilege::Privilege;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// The answer to one request
type Answer = std::result::Result<Output, String>;

/// Answers requests from `requests` on `answers` until `requests` ends. This is
/// `comtrya __helper`, it runs as root
pub fn serve(requests: impl BufRead, mut answers: impl Write) -> io::Result<()> {
    for request in requests.lines() {
        let answer = answer(&request?);
        serde_json::to_writer(&mut answers, &answer)?;
        answers.write_all(b"\n")?;
        answers.flush()?;
    }
    Ok(())
}

/// Runs the atom of one request
fn answer(request: &str) -> Answer {
    let atom: Atom = serde_json::from_str(request).map_err(|err| format!("bad request: {err}"))?;
    atom.execute().map_err(|err| chain(&err))
}

/// The error and its causes, like `{:#}` of [`anyhow::Error`]
fn chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(&format!(": {cause}"));
        source = cause.source();
    }
    text
}

/// Sends requests to a helper and reads its answers
struct Connection<W, R> {
    /// The helper's stdin
    requests: W,
    /// The helper's stdout
    answers: R,
}

impl<W: Write, R: BufRead> Connection<W, R> {
    /// Runs `atom` in the helper and waits for the answer
    fn run(&mut self, atom: &Atom) -> atoms::Result<Output> {
        self.send(atom).map_err(|_| quit())?;

        let mut line = String::new();
        if self.answers.read_line(&mut line).map_err(|_| quit())? == 0 {
            return Err(quit());
        }

        let answer: Answer = serde_json::from_str(&line)
            .map_err(|err| atoms::Error::AsRoot(format!("bad answer: {err}")))?;
        answer.map_err(atoms::Error::AsRoot)
    }

    /// Writes one request
    fn send(&mut self, atom: &Atom) -> io::Result<()> {
        serde_json::to_writer(&mut self.requests, atom)?;
        self.requests.write_all(b"\n")?;
        self.requests.flush()
    }
}

/// The error when the helper is gone, e.g. because the password was wrong
fn quit() -> atoms::Error {
    atoms::Error::AsRoot(String::from("the privileged helper quit"))
}

/// A helper process, and the connection to it
struct Running {
    /// The process, waited for when the run ends, see [`Helper`]'s `Drop`
    child: Child,
    /// Its stdin and stdout
    connection: Connection<ChildStdin, BufReader<ChildStdout>>,
}

/// The privileged helper of a run, started on first use
pub(crate) struct Helper {
    /// How to become root
    privilege: Privilege,
    /// The helper, once it's started
    running: Option<Running>,
}

impl Helper {
    /// A helper that becomes root with `privilege` when it's first needed
    pub(crate) fn new(privilege: Privilege) -> Self {
        Self {
            privilege,
            running: None,
        }
    }

    /// Runs `atom` as root
    pub(crate) fn run(&mut self, atom: &Atom) -> atoms::Result<Output> {
        if self.running.is_none() {
            self.running = Some(self.start()?);
        }
        let running = self.running.as_mut().expect("started above");
        running.connection.run(atom)
    }

    /// Starts `<privilege> <comtrya> __helper`. The privilege provider asks for the password
    /// on the terminal, not on stdin, so stdin and stdout can be the connection
    fn start(&self) -> atoms::Result<Running> {
        let program = self.privilege.to_string();
        let spawn_error = |source| atoms::Error::Spawn {
            command: program.clone(),
            source,
        };
        let comtrya = std::env::current_exe().map_err(spawn_error)?;

        let mut child = Command::new(&program)
            .arg(comtrya)
            .arg("__helper")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(spawn_error)?;

        let connection = Connection {
            requests: child.stdin.take().expect("stdin is piped"),
            answers: BufReader::new(child.stdout.take().expect("stdout is piped")),
        };
        Ok(Running { child, connection })
    }
}

impl Drop for Helper {
    /// Closing its stdin ends the helper, then it's waited for
    fn drop(&mut self) {
        if let Some(Running {
            mut child,
            connection,
        }) = self.running.take()
        {
            drop(connection);
            let _ = child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atoms::file::Create;
    use pretty_assertions::assert_eq;

    /// A helper serving in a thread, connected through pipes like the real one
    fn connected() -> Connection<io::PipeWriter, BufReader<io::PipeReader>> {
        let (requests_in, requests) = io::pipe().unwrap();
        let (answers_out, answers) = io::pipe().unwrap();
        std::thread::spawn(move || serve(BufReader::new(requests_in), answers));

        Connection {
            requests,
            answers: BufReader::new(answers_out),
        }
    }

    #[test]
    fn the_helper_runs_atoms_and_answers_each() {
        let dir = tempfile::tempdir().unwrap();
        let mut helper = connected();

        for name in ["a", "b"] {
            let atom = Atom::from(Create {
                path: dir.path().join(name),
            });
            assert_eq!(Output::default(), helper.run(&atom).unwrap());
        }

        assert!(dir.path().join("a").is_file());
        assert!(dir.path().join("b").is_file());
    }

    #[test]
    fn an_error_comes_back_with_its_causes() {
        let mut helper = connected();
        let atom = Atom::from(Create {
            path: "/comtrya-no-such-dir/file".into(),
        });

        let error = helper.run(&atom).unwrap_err().to_string();

        assert!(
            error.starts_with("as root: can't create /comtrya-no-such-dir/file: "),
            "{error}"
        );
    }

    #[test]
    fn a_helper_that_quits_is_an_error() {
        let (answers, _) = io::pipe().unwrap();
        let mut helper = Connection {
            requests: io::sink(),
            answers: BufReader::new(answers),
        };
        let atom = Atom::from(Create {
            path: "/tmp/x".into(),
        });

        assert_eq!(
            "as root: the privileged helper quit",
            helper.run(&atom).unwrap_err().to_string()
        );
    }

    #[test]
    fn a_bad_request_is_answered_not_fatal() {
        let mut answers = vec![];

        serve(io::Cursor::new("nope\n"), &mut answers).unwrap();

        let answer: Answer = serde_json::from_slice(&answers).unwrap();
        assert!(answer.unwrap_err().starts_with("bad request: "));
    }
}
