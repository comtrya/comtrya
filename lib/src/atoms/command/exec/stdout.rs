//! What happens with stdout once the program ended: stop the action when it contains a text,
//! or write it to a file

use super::Exec;
use crate::atoms::{Error, Result};
use std::path::Path;
use tracing::debug;

impl Exec {
    /// Fails if `stdout` contains [`Exec::stop_if_stdout_contains`], in any case
    pub(super) fn stop_if_stdout_contains(&self, stdout: &[u8]) -> Result<()> {
        let Some(text) = &self.stop_if_stdout_contains else {
            return Ok(());
        };
        let stdout = String::from_utf8_lossy(stdout).to_lowercase();
        if !stdout.contains(&text.to_lowercase()) {
            return Ok(());
        }
        Err(Error::StdoutContains {
            command: [vec![self.command.clone()], self.arguments.clone()]
                .concat()
                .join(" "),
            text: text.clone(),
        })
    }

    /// Writes stdout to `stdout_file`, if there is one
    pub(super) fn write_stdout_file(&self, stdout: &[u8]) -> Result<()> {
        match &self.stdout_file {
            Some(stdout_file) => write_stdout(stdout_file, stdout),
            None => Ok(()),
        }
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

#[cfg(test)]
mod tests {
    use crate::atoms::command::exec::new_run_command;
    use crate::atoms::Change;
    use pretty_assertions::assert_eq;

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
    fn it_stops_when_stdout_contains_the_text() {
        let mut command_run = new_run_command(String::from("echo"));
        command_run.arguments = vec![String::from("1 package to be REMOVED")];
        command_run.stop_if_stdout_contains = Some(String::from("removed"));

        let error = command_run
            .execute()
            .expect_err("the output contains the text");
        assert_eq!(
            "the output of `echo 1 package to be REMOVED` contains \"removed\"",
            error.to_string()
        );

        command_run.arguments = vec![String::from("nothing to do")];
        command_run.execute().unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn output_that_isnt_utf8_doesnt_fail_the_command() {
        let dir = tempfile::tempdir().unwrap();
        let stdout_file = dir.path().join("out.bin");
        let mut command_run = new_run_command(String::from("printf"));
        command_run.arguments = vec![String::from("a\\377b")];
        command_run.stdout_file = Some(stdout_file.clone());

        command_run.execute().unwrap();

        // The file gets the bytes as they are
        assert_eq!(b"a\xffb".to_vec(), std::fs::read(stdout_file).unwrap());
    }
}
